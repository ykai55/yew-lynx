#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;

use lynx_element_bridge_core::{BridgeError, CommandBatch, EventMessage, NodeId, Status};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub const PROTOCOL_VERSION_V2: u32 = 2;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct MountRequest {
    pub protocol_version: u32,
    pub root: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct EventRequest {
    pub protocol_version: u32,
    pub event: EventMessage,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct GuestResponse {
    pub protocol_version: u32,
    pub result: GuestResult,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum GuestResult {
    Ok(CommandBatch),
    Err { status: Status, message: String },
}

impl GuestResponse {
    pub fn from_result(result: Result<CommandBatch, BridgeError>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION_V2,
            result: match result {
                Ok(batch) => GuestResult::Ok(batch),
                Err(error) => GuestResult::Err {
                    status: error.status,
                    message: error.message,
                },
            },
        }
    }
}

pub fn encode_mount_request(request: &MountRequest) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(request)
}

pub fn decode_mount_request(bytes: &[u8]) -> Result<MountRequest, BridgeError> {
    decode_versioned(bytes, |request: &MountRequest| request.protocol_version)
}

pub fn encode_event_request(request: &EventRequest) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(request)
}

pub fn decode_event_request(bytes: &[u8]) -> Result<EventRequest, BridgeError> {
    decode_versioned(bytes, |request: &EventRequest| request.protocol_version)
}

pub fn encode_guest_response(response: &GuestResponse) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(response)
}

pub fn decode_guest_response(bytes: &[u8]) -> Result<GuestResponse, BridgeError> {
    decode_versioned(bytes, |response: &GuestResponse| response.protocol_version)
}

fn decode_versioned<T: DeserializeOwned>(
    bytes: &[u8],
    version: impl FnOnce(&T) -> u32,
) -> Result<T, BridgeError> {
    let value = postcard::from_bytes(bytes).map_err(|error| {
        BridgeError::new(
            Status::InvalidArgument,
            format!("invalid postcard message: {error}"),
        )
    })?;
    let actual = version(&value);
    if actual != PROTOCOL_VERSION_V2 {
        return Err(BridgeError::new(
            Status::Unsupported,
            format!("unsupported protocol version {actual}"),
        ));
    }
    Ok(value)
}

pub trait GuestApplication: Sized + 'static {
    fn mount(request: MountRequest) -> Result<(Self, CommandBatch), BridgeError>;

    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BridgeError>;

    fn destroy(self) -> Result<CommandBatch, BridgeError>;
}

pub struct GuestRuntime<A> {
    application: Option<A>,
}

impl<A: GuestApplication> GuestRuntime<A> {
    pub const fn new() -> Self {
        Self { application: None }
    }

    pub fn mount(&mut self, input: &[u8]) -> GuestResponse {
        let result = decode_mount_request(input).and_then(|request| {
            if self.application.is_some() {
                return Err(BridgeError::new(
                    Status::InvalidSession,
                    "guest application is already mounted",
                ));
            }
            let (application, batch) = A::mount(request)?;
            self.application = Some(application);
            Ok(batch)
        });
        GuestResponse::from_result(result)
    }

    pub fn dispatch_event(&mut self, input: &[u8]) -> GuestResponse {
        let result = decode_event_request(input).and_then(|request| {
            self.application
                .as_mut()
                .ok_or_else(not_mounted)?
                .dispatch_event(request.event)
        });
        GuestResponse::from_result(result)
    }

    pub fn destroy(&mut self) -> GuestResponse {
        GuestResponse::from_result(
            self.application
                .take()
                .ok_or_else(not_mounted)
                .and_then(GuestApplication::destroy),
        )
    }
}

impl<A: GuestApplication> Default for GuestRuntime<A> {
    fn default() -> Self {
        Self::new()
    }
}

fn not_mounted() -> BridgeError {
    BridgeError::new(Status::InvalidSession, "guest application is not mounted")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BufferDescriptor {
    pub pointer: usize,
    pub length: u32,
}

impl BufferDescriptor {
    pub fn pack_wasm32(self) -> u64 {
        ((self.pointer as u64) << 32) | u64::from(self.length)
    }
}

pub struct AbiRuntime<A> {
    guest: GuestRuntime<A>,
    inputs: HashMap<usize, Box<[u8]>>,
    outputs: HashMap<usize, Box<[u8]>>,
}

impl<A: GuestApplication> AbiRuntime<A> {
    pub fn new() -> Self {
        Self {
            guest: GuestRuntime::new(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        }
    }

    pub fn alloc(&mut self, length: u32) -> usize {
        if length == 0 {
            return 0;
        }
        let mut bytes = vec![0; length as usize].into_boxed_slice();
        let pointer = bytes.as_mut_ptr() as usize;
        self.inputs.insert(pointer, bytes);
        pointer
    }

    pub fn dealloc(&mut self, pointer: usize, length: u32) -> bool {
        remove_buffer(&mut self.inputs, pointer, length)
    }

    pub fn mount(&mut self, pointer: usize, length: u32) -> BufferDescriptor {
        let response = match self.input(pointer, length) {
            Ok(input) => self.guest.mount(&input),
            Err(error) => GuestResponse::from_result(Err(error)),
        };
        self.store_output(response)
    }

    pub fn dispatch_event(&mut self, pointer: usize, length: u32) -> BufferDescriptor {
        let response = match self.input(pointer, length) {
            Ok(input) => self.guest.dispatch_event(&input),
            Err(error) => GuestResponse::from_result(Err(error)),
        };
        self.store_output(response)
    }

    pub fn destroy(&mut self) -> BufferDescriptor {
        let response = self.guest.destroy();
        self.store_output(response)
    }

    pub fn output(&self, descriptor: BufferDescriptor) -> Option<&[u8]> {
        self.outputs
            .get(&descriptor.pointer)
            .filter(|bytes| bytes.len() == descriptor.length as usize)
            .map(AsRef::as_ref)
    }

    pub fn output_dealloc(&mut self, pointer: usize, length: u32) -> bool {
        remove_buffer(&mut self.outputs, pointer, length)
    }

    fn input(&self, pointer: usize, length: u32) -> Result<Vec<u8>, BridgeError> {
        self.inputs
            .get(&pointer)
            .filter(|bytes| bytes.len() == length as usize)
            .map(|bytes| bytes.to_vec())
            .ok_or_else(|| BridgeError::new(Status::InvalidArgument, "invalid guest input buffer"))
    }

    fn store_output(&mut self, response: GuestResponse) -> BufferDescriptor {
        let mut bytes = postcard::to_allocvec(&response)
            .expect("serializing a guest response into a Vec must succeed")
            .into_boxed_slice();
        let descriptor = BufferDescriptor {
            pointer: bytes.as_mut_ptr() as usize,
            length: bytes.len() as u32,
        };
        self.outputs.insert(descriptor.pointer, bytes);
        descriptor
    }
}

impl<A: GuestApplication> Default for AbiRuntime<A> {
    fn default() -> Self {
        Self::new()
    }
}

fn remove_buffer(buffers: &mut HashMap<usize, Box<[u8]>>, pointer: usize, length: u32) -> bool {
    if buffers
        .get(&pointer)
        .is_some_and(|bytes| bytes.len() == length as usize)
    {
        buffers.remove(&pointer);
        true
    } else {
        false
    }
}

#[macro_export]
macro_rules! export_guest {
    ($application:ty) => {
        #[cfg(target_arch = "wasm32")]
        mod __lynx_element_bridge_wasm_exports {
            use std::cell::RefCell;

            use super::*;

            std::thread_local! {
                static RUNTIME: RefCell<$crate::AbiRuntime<$application>> =
                    RefCell::new($crate::AbiRuntime::new());
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn version() -> u32 {
                $crate::PROTOCOL_VERSION_V2
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn alloc(length: u32) -> u32 {
                RUNTIME.with(|runtime| runtime.borrow_mut().alloc(length) as u32)
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn dealloc(pointer: u32, length: u32) -> u32 {
                RUNTIME.with(|runtime| {
                    u32::from(runtime.borrow_mut().dealloc(pointer as usize, length))
                })
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn mount(pointer: u32, length: u32) -> u64 {
                RUNTIME.with(|runtime| {
                    runtime
                        .borrow_mut()
                        .mount(pointer as usize, length)
                        .pack_wasm32()
                })
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn dispatch_event(pointer: u32, length: u32) -> u64 {
                RUNTIME.with(|runtime| {
                    runtime
                        .borrow_mut()
                        .dispatch_event(pointer as usize, length)
                        .pack_wasm32()
                })
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn destroy() -> u64 {
                RUNTIME.with(|runtime| runtime.borrow_mut().destroy().pack_wasm32())
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn output_dealloc(pointer: u32, length: u32) -> u32 {
                RUNTIME.with(|runtime| {
                    u32::from(
                        runtime
                            .borrow_mut()
                            .output_dealloc(pointer as usize, length),
                    )
                })
            }
        }
    };
}
