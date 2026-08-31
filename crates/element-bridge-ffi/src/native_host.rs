#![allow(unsafe_code)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::rc::Rc;
use std::thread::{self, ThreadId};

use lynx_element_bridge_core::{
    BridgeError, CallbackId, Command, CommandBatch, EventMessage, ListenerId, NodeId, Status,
};

pub const NATIVE_RENDERER_ABI_VERSION: u32 = 1;

pub type NativeStatus = u32;
pub const NATIVE_STATUS_OK: NativeStatus = 0;
pub const NATIVE_STATUS_INVALID_ARGUMENT: NativeStatus = 1;
pub const NATIVE_STATUS_INVALID_SESSION: NativeStatus = 2;
pub const NATIVE_STATUS_WRONG_THREAD: NativeStatus = 3;
pub const NATIVE_STATUS_UNSUPPORTED: NativeStatus = 4;
pub const NATIVE_STATUS_INVALID_OWNERSHIP: NativeStatus = 5;
pub const NATIVE_STATUS_INVALID_LISTENER: NativeStatus = 6;
pub const NATIVE_STATUS_RESOURCE_EXHAUSTED: NativeStatus = 7;
pub const NATIVE_STATUS_HOST_ERROR: NativeStatus = 8;
pub const NATIVE_STATUS_PANIC: NativeStatus = 9;
pub const NATIVE_STATUS_INTERNAL_ERROR: NativeStatus = 10;

pub type NativeHostHandle = u64;
pub type NativeRendererHandle = u64;
pub type NativeNodeHandle = u32;
pub type NativeListenerHandle = u32;
pub type NativeTimerHandle = u32;
pub type NativeCallbackHandle = u32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeUtf8 {
    pub data: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeBytes {
    pub data: *const u8,
    pub len: usize,
}

pub type NativeOnEventFn = unsafe extern "C" fn(
    context: *mut c_void,
    renderer: NativeRendererHandle,
    listener: NativeListenerHandle,
    callback: NativeCallbackHandle,
    name: NativeUtf8,
    content_type: NativeUtf8,
    payload: NativeBytes,
) -> NativeStatus;

pub type NativeOnTimerFn = unsafe extern "C" fn(
    context: *mut c_void,
    renderer: NativeRendererHandle,
    timer: NativeTimerHandle,
    callback: NativeCallbackHandle,
) -> NativeStatus;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeRendererCallbacksV1 {
    pub context: *mut c_void,
    pub on_event: Option<NativeOnEventFn>,
    pub on_timer: Option<NativeOnTimerFn>,
}

pub type NativeAcquireFn = unsafe extern "C" fn(
    host: NativeHostHandle,
    callbacks: *const NativeRendererCallbacksV1,
    renderer: *mut NativeRendererHandle,
) -> NativeStatus;
pub type NativeReleaseFn = unsafe extern "C" fn(renderer: NativeRendererHandle) -> NativeStatus;
pub type NativeGetRootFn = unsafe extern "C" fn(
    renderer: NativeRendererHandle,
    root: *mut NativeNodeHandle,
) -> NativeStatus;
pub type NativeCreateElementFn = unsafe extern "C" fn(
    renderer: NativeRendererHandle,
    tag: NativeUtf8,
    node: *mut NativeNodeHandle,
) -> NativeStatus;
pub type NativeCreateRawTextFn = unsafe extern "C" fn(
    renderer: NativeRendererHandle,
    text: NativeUtf8,
    node: *mut NativeNodeHandle,
) -> NativeStatus;
pub type NativeSetRawTextFn = unsafe extern "C" fn(
    renderer: NativeRendererHandle,
    node: NativeNodeHandle,
    text: NativeUtf8,
) -> NativeStatus;
pub type NativeSetAttributeFn = unsafe extern "C" fn(
    renderer: NativeRendererHandle,
    node: NativeNodeHandle,
    name: NativeUtf8,
    value: NativeUtf8,
) -> NativeStatus;
pub type NativeInsertBeforeFn = unsafe extern "C" fn(
    renderer: NativeRendererHandle,
    parent: NativeNodeHandle,
    child: NativeNodeHandle,
    reference: NativeNodeHandle,
) -> NativeStatus;
pub type NativeRemoveChildFn = unsafe extern "C" fn(
    renderer: NativeRendererHandle,
    parent: NativeNodeHandle,
    child: NativeNodeHandle,
) -> NativeStatus;
pub type NativeDestroyNodeFn =
    unsafe extern "C" fn(renderer: NativeRendererHandle, node: NativeNodeHandle) -> NativeStatus;
pub type NativeAddEventListenerFn = unsafe extern "C" fn(
    renderer: NativeRendererHandle,
    node: NativeNodeHandle,
    listener: NativeListenerHandle,
    callback: NativeCallbackHandle,
    name: NativeUtf8,
) -> NativeStatus;
pub type NativeRemoveEventListenerFn = unsafe extern "C" fn(
    renderer: NativeRendererHandle,
    node: NativeNodeHandle,
    listener: NativeListenerHandle,
    callback: NativeCallbackHandle,
    name: NativeUtf8,
) -> NativeStatus;
pub type NativeFlushFn = unsafe extern "C" fn(renderer: NativeRendererHandle) -> NativeStatus;
pub type NativeCreateTimerFn = unsafe extern "C" fn(
    renderer: NativeRendererHandle,
    delay_millis: u64,
    repeating: u32,
    callback: NativeCallbackHandle,
    timer: *mut NativeTimerHandle,
) -> NativeStatus;
pub type NativeCancelTimerFn =
    unsafe extern "C" fn(renderer: NativeRendererHandle, timer: NativeTimerHandle) -> NativeStatus;
pub type NativeImportStyleSheetFn =
    unsafe extern "C" fn(renderer: NativeRendererHandle, fragment: NativeBytes) -> NativeStatus;
pub type NativeClearStyleSheetsFn =
    unsafe extern "C" fn(renderer: NativeRendererHandle) -> NativeStatus;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeRendererApiV1 {
    pub abi_version: u32,
    pub struct_size: usize,
    pub acquire: Option<NativeAcquireFn>,
    pub release: Option<NativeReleaseFn>,
    pub get_root: Option<NativeGetRootFn>,
    pub create_element: Option<NativeCreateElementFn>,
    pub create_raw_text: Option<NativeCreateRawTextFn>,
    pub set_raw_text: Option<NativeSetRawTextFn>,
    pub set_attribute: Option<NativeSetAttributeFn>,
    pub insert_before: Option<NativeInsertBeforeFn>,
    pub remove_child: Option<NativeRemoveChildFn>,
    pub destroy_node: Option<NativeDestroyNodeFn>,
    pub add_event_listener: Option<NativeAddEventListenerFn>,
    pub remove_event_listener: Option<NativeRemoveEventListenerFn>,
    pub flush: Option<NativeFlushFn>,
    pub create_timer: Option<NativeCreateTimerFn>,
    pub cancel_timer: Option<NativeCancelTimerFn>,
    pub import_style_sheet: Option<NativeImportStyleSheetFn>,
    pub clear_style_sheets: Option<NativeClearStyleSheetsFn>,
}

pub type NativeRendererGetApiFn = unsafe extern "C" fn(version: u32) -> *const NativeRendererApiV1;

#[repr(C)]
#[derive(Clone, Copy)]
struct NativeRendererApiHeader {
    abi_version: u32,
    struct_size: usize,
}

#[derive(Clone)]
struct ListenerMapping {
    node: NodeId,
    listener: ListenerId,
    callback: CallbackId,
    name: String,
}

#[derive(Clone, Copy)]
struct TimerMapping {
    callback: CallbackId,
    repeating: bool,
}

pub struct NativeHost {
    api: NativeRendererApiV1,
    renderer: Option<NativeRendererHandle>,
    root: NodeId,
    owner: ThreadId,
    nodes: HashMap<NodeId, NativeNodeHandle>,
    listeners: HashMap<NativeListenerHandle, ListenerMapping>,
    next_native_listener: Option<NativeListenerHandle>,
    timers: HashMap<NativeTimerHandle, TimerMapping>,
    last_sequence: Option<u32>,
    poisoned: bool,
    not_send_or_sync: PhantomData<Rc<()>>,
}

impl NativeHost {
    /// Acquires a renderer through the versioned production function-table seam.
    ///
    /// # Safety
    ///
    /// `get_api`, `host`, and `callbacks` must obey the contract declared in
    /// `include/lynx_native_renderer.h` for the lifetime of the returned host.
    pub unsafe fn acquire(
        get_api: NativeRendererGetApiFn,
        host: NativeHostHandle,
        root: NodeId,
        callbacks: NativeRendererCallbacksV1,
    ) -> Result<Self, BridgeError> {
        if host == 0 {
            return Err(BridgeError::new(
                Status::InvalidArgument,
                "native host handle must not be zero",
            ));
        }
        if callbacks.on_event.is_none() || callbacks.on_timer.is_none() {
            return Err(BridgeError::new(
                Status::InvalidArgument,
                "native renderer callbacks must not be null",
            ));
        }
        // SAFETY: The caller guarantees that the resolver obeys the C ABI contract.
        let api_pointer = unsafe { get_api(NATIVE_RENDERER_ABI_VERSION) };
        if api_pointer.is_null() {
            return Err(BridgeError::new(
                Status::Unsupported,
                "native renderer API V1 is unavailable",
            ));
        }
        // SAFETY: The resolver contract provides a readable, aligned table header.
        let header = unsafe { ptr::read(api_pointer.cast::<NativeRendererApiHeader>()) };
        if header.abi_version != NATIVE_RENDERER_ABI_VERSION {
            return Err(BridgeError::new(
                Status::Unsupported,
                format!(
                    "unsupported native renderer ABI version {}",
                    header.abi_version
                ),
            ));
        }
        if header.struct_size < mem::size_of::<NativeRendererApiV1>() {
            return Err(BridgeError::new(
                Status::Unsupported,
                format!(
                    "native renderer API V1 table is too small: {} bytes",
                    header.struct_size
                ),
            ));
        }
        // SAFETY: The advertised size covers V1 and the resolver contract makes it readable.
        let api = unsafe { ptr::read(api_pointer) };
        if api.acquire.is_none()
            || api.release.is_none()
            || api.get_root.is_none()
            || api.create_element.is_none()
            || api.create_raw_text.is_none()
            || api.set_raw_text.is_none()
            || api.set_attribute.is_none()
            || api.insert_before.is_none()
            || api.remove_child.is_none()
            || api.destroy_node.is_none()
            || api.add_event_listener.is_none()
            || api.remove_event_listener.is_none()
            || api.flush.is_none()
            || api.create_timer.is_none()
            || api.cancel_timer.is_none()
            || api.import_style_sheet.is_none()
            || api.clear_style_sheets.is_none()
        {
            return Err(BridgeError::new(
                Status::Unsupported,
                "native renderer API V1 has a null function",
            ));
        }

        let mut renderer = 0;
        // SAFETY: The table and arguments were validated above and the caller owns the host.
        let status = unsafe {
            api.acquire.expect("validated function")(
                host,
                ptr::addr_of!(callbacks),
                ptr::addr_of_mut!(renderer),
            )
        };
        if status != NATIVE_STATUS_OK {
            return Err(native_error(status, "acquire"));
        }
        if renderer == 0 {
            return Err(BridgeError::new(
                Status::HostError,
                "native acquire returned a zero renderer handle",
            ));
        }

        let mut native_root = 0;
        // SAFETY: Acquisition returned a live renderer and the output is writable.
        let status = unsafe {
            api.get_root.expect("validated function")(renderer, ptr::addr_of_mut!(native_root))
        };
        if status != NATIVE_STATUS_OK || native_root == 0 {
            // SAFETY: This is the one cleanup attempt for the acquired renderer.
            let _ = unsafe { api.release.expect("validated function")(renderer) };
            return if status != NATIVE_STATUS_OK {
                Err(native_error(status, "get_root"))
            } else {
                Err(BridgeError::new(
                    Status::HostError,
                    "native get_root returned a zero node handle",
                ))
            };
        }

        Ok(Self {
            api,
            renderer: Some(renderer),
            root,
            owner: thread::current().id(),
            nodes: HashMap::from([(root, native_root)]),
            listeners: HashMap::new(),
            next_native_listener: Some(1),
            timers: HashMap::new(),
            last_sequence: None,
            poisoned: false,
            not_send_or_sync: PhantomData,
        })
    }

    pub fn apply(&mut self, batch: &CommandBatch) -> Result<(), BridgeError> {
        let renderer = self.ensure_usable()?;
        if self
            .last_sequence
            .is_some_and(|sequence| batch.sequence <= sequence)
        {
            return Err(BridgeError::new(
                Status::InvalidArgument,
                format!(
                    "command batch sequence {} is not newer than the last applied sequence",
                    batch.sequence
                ),
            ));
        }
        for (applied, command) in batch.commands.iter().enumerate() {
            if let Err(error) = self.apply_command(renderer, command) {
                if applied != 0 {
                    self.poisoned = true;
                }
                return Err(error);
            }
        }
        if batch.final_commit {
            // SAFETY: The copied table was validated and the renderer is still live.
            let status = unsafe { self.api.flush.expect("validated function")(renderer) };
            self.require_host_success(status, "flush")?;
        }
        self.last_sequence = Some(batch.sequence);
        Ok(())
    }

    /// Starts a fresh application lifecycle on the acquired renderer.
    ///
    /// The previous application must have removed everything it owned first.
    pub fn reset_application_epoch(&mut self) -> Result<(), BridgeError> {
        let renderer = self.ensure_usable()?;
        if self.nodes.len() != 1
            || !self.nodes.contains_key(&self.root)
            || !self.listeners.is_empty()
            || !self.timers.is_empty()
        {
            return Err(BridgeError::new(
                Status::InvalidOwnership,
                "cannot reset native application epoch while application resources remain",
            ));
        }
        // SAFETY: The copied table was validated and the renderer is still live.
        let status = unsafe { self.api.clear_style_sheets.expect("validated function")(renderer) };
        self.require_host_success(status, "clear_style_sheets")?;
        self.last_sequence = None;
        Ok(())
    }

    pub fn event_message(
        &self,
        renderer: NativeRendererHandle,
        listener: NativeListenerHandle,
        callback: NativeCallbackHandle,
        name: &str,
        content_type: String,
        payload: Vec<u8>,
    ) -> Result<EventMessage, BridgeError> {
        self.validate_renderer(renderer)?;
        let callback = CallbackId::new(callback)?;
        let mapping = self.listeners.get(&listener).ok_or_else(|| {
            BridgeError::new(
                Status::InvalidListener,
                format!("invalid or stale native listener {listener}"),
            )
        })?;
        if mapping.callback != callback || mapping.name != name {
            return Err(BridgeError::new(
                Status::InvalidListener,
                format!("native listener {listener} identity does not match"),
            ));
        }
        Ok(EventMessage {
            listener: mapping.listener,
            callback: mapping.callback,
            content_type,
            payload,
        })
    }

    pub fn validate_renderer(&self, renderer: NativeRendererHandle) -> Result<(), BridgeError> {
        let active_renderer = self.ensure_usable()?;
        if renderer != active_renderer {
            return Err(BridgeError::new(
                Status::InvalidSession,
                "native callback renderer does not match the active renderer",
            ));
        }
        Ok(())
    }

    pub fn create_timer(
        &mut self,
        delay_millis: u64,
        repeating: bool,
        callback: CallbackId,
    ) -> Result<NativeTimerHandle, BridgeError> {
        let renderer = self.ensure_usable()?;
        let mut timer = 0;
        // SAFETY: The copied table was validated and the output remains writable for the call.
        let status = unsafe {
            self.api.create_timer.expect("validated function")(
                renderer,
                delay_millis,
                u32::from(repeating),
                callback.get(),
                ptr::addr_of_mut!(timer),
            )
        };
        self.require_host_success(status, "create_timer")?;
        if timer == 0 || self.timers.contains_key(&timer) {
            self.poisoned = true;
            return Err(BridgeError::new(
                Status::HostError,
                "native create_timer returned an invalid timer handle",
            ));
        }
        self.timers.insert(
            timer,
            TimerMapping {
                callback,
                repeating,
            },
        );
        Ok(timer)
    }

    pub fn timer_callback(
        &mut self,
        renderer: NativeRendererHandle,
        timer: NativeTimerHandle,
        callback: NativeCallbackHandle,
    ) -> Result<CallbackId, BridgeError> {
        self.validate_renderer(renderer)?;
        let callback = CallbackId::new(callback)?;
        let mapping = self.timers.get(&timer).copied().ok_or_else(|| {
            BridgeError::new(
                Status::InvalidArgument,
                format!("invalid or stale native timer {timer}"),
            )
        })?;
        if mapping.callback != callback {
            return Err(BridgeError::new(
                Status::InvalidArgument,
                format!("native timer {timer} callback identity does not match"),
            ));
        }
        if !mapping.repeating {
            self.timers.remove(&timer);
        }
        Ok(callback)
    }

    pub fn cancel_timer(&mut self, timer: NativeTimerHandle) -> Result<(), BridgeError> {
        let renderer = self.ensure_usable()?;
        if timer == 0 || !self.timers.contains_key(&timer) {
            return Err(BridgeError::new(
                Status::InvalidArgument,
                format!("invalid or stale native timer {timer}"),
            ));
        }
        // SAFETY: The copied table was validated and this host owns the timer.
        let status = unsafe { self.api.cancel_timer.expect("validated function")(renderer, timer) };
        self.require_host_success(status, "cancel_timer")?;
        self.timers.remove(&timer);
        Ok(())
    }

    pub fn release(&mut self) -> Result<(), BridgeError> {
        self.ensure_owner()?;
        let renderer = self.renderer.take().ok_or_else(|| {
            BridgeError::new(
                Status::InvalidSession,
                "native renderer is already released",
            )
        })?;
        self.nodes.clear();
        self.listeners.clear();
        self.timers.clear();
        // SAFETY: Taking the handle makes this the only release attempt by NativeHost.
        let status = unsafe { self.api.release.expect("validated function")(renderer) };
        if status != NATIVE_STATUS_OK {
            return Err(native_error(status, "release"));
        }
        Ok(())
    }

    fn apply_command(
        &mut self,
        renderer: NativeRendererHandle,
        command: &Command,
    ) -> Result<(), BridgeError> {
        match command {
            Command::ImportStyleSheet { fragment } => {
                if fragment.is_empty() {
                    return Err(BridgeError::new(
                        Status::InvalidArgument,
                        "compiled stylesheet fragment must not be empty",
                    ));
                }
                // SAFETY: The bytes are borrowed only for the duration of this call.
                let status = unsafe {
                    self.api.import_style_sheet.expect("validated function")(
                        renderer,
                        NativeBytes::from_slice(fragment),
                    )
                };
                self.require_host_success(status, "import_style_sheet")?;
            }
            Command::CreateElement { node, tag } => {
                self.ensure_new_node(*node)?;
                let mut native_node = 0;
                // SAFETY: The string is borrowed only for the call and the output is writable.
                let status = unsafe {
                    self.api.create_element.expect("validated function")(
                        renderer,
                        NativeUtf8::from_str(tag),
                        ptr::addr_of_mut!(native_node),
                    )
                };
                self.finish_create(*node, native_node, status, "create_element")?;
            }
            Command::CreateRawText { node, text } => {
                self.ensure_new_node(*node)?;
                let mut native_node = 0;
                // SAFETY: The string is borrowed only for the call and the output is writable.
                let status = unsafe {
                    self.api.create_raw_text.expect("validated function")(
                        renderer,
                        NativeUtf8::from_str(text),
                        ptr::addr_of_mut!(native_node),
                    )
                };
                self.finish_create(*node, native_node, status, "create_raw_text")?;
            }
            Command::AppendElement { parent, child } => {
                let parent = self.native_node(*parent)?;
                let child = self.native_node(*child)?;
                // SAFETY: The copied table was validated and all handles are mapped.
                let status = unsafe {
                    self.api.insert_before.expect("validated function")(renderer, parent, child, 0)
                };
                self.require_host_success(status, "insert_before")?;
            }
            Command::InsertElementBefore {
                parent,
                child,
                reference,
            } => {
                let parent = self.native_node(*parent)?;
                let child = self.native_node(*child)?;
                let reference = self.native_node(*reference)?;
                // SAFETY: The copied table was validated and all handles are mapped.
                let status = unsafe {
                    self.api.insert_before.expect("validated function")(
                        renderer, parent, child, reference,
                    )
                };
                self.require_host_success(status, "insert_before")?;
            }
            Command::RemoveElement { parent, child } => {
                let parent = self.native_node(*parent)?;
                let child = self.native_node(*child)?;
                // SAFETY: The copied table was validated and both handles are mapped.
                let status = unsafe {
                    self.api.remove_child.expect("validated function")(renderer, parent, child)
                };
                self.require_host_success(status, "remove_child")?;
            }
            Command::DestroyNode { node } => {
                if *node == self.root {
                    return Err(BridgeError::new(
                        Status::InvalidOwnership,
                        "the native root is caller-owned",
                    ));
                }
                let native_node = self.native_node(*node)?;
                // SAFETY: The copied table was validated and the node handle is mapped.
                let status = unsafe {
                    self.api.destroy_node.expect("validated function")(renderer, native_node)
                };
                self.require_host_success(status, "destroy_node")?;
                self.nodes.remove(node);
            }
            Command::SetAttribute { node, name, value } => {
                let node = self.native_node(*node)?;
                let value = value
                    .as_deref()
                    .map_or_else(NativeUtf8::removed, NativeUtf8::from_str);
                // SAFETY: Both strings are borrowed only for the duration of this call.
                let status = unsafe {
                    self.api.set_attribute.expect("validated function")(
                        renderer,
                        node,
                        NativeUtf8::from_str(name),
                        value,
                    )
                };
                self.require_host_success(status, "set_attribute")?;
            }
            Command::AddEventListener {
                node,
                listener,
                callback,
                name,
            } => {
                if self
                    .listeners
                    .values()
                    .any(|mapping| mapping.listener == *listener)
                {
                    return Err(BridgeError::new(
                        Status::InvalidListener,
                        format!("native listener {} already exists", listener.get()),
                    ));
                }
                let native_node = self.native_node(*node)?;
                let native_listener = self.next_native_listener.ok_or_else(|| {
                    BridgeError::new(
                        Status::ResourceExhausted,
                        "native listener handle space is exhausted",
                    )
                })?;
                self.next_native_listener = native_listener.checked_add(1);
                // SAFETY: The name is borrowed only for the call and all IDs are nonzero.
                let status = unsafe {
                    self.api.add_event_listener.expect("validated function")(
                        renderer,
                        native_node,
                        native_listener,
                        callback.get(),
                        NativeUtf8::from_str(name),
                    )
                };
                self.require_host_success(status, "add_event_listener")?;
                self.listeners.insert(
                    native_listener,
                    ListenerMapping {
                        node: *node,
                        listener: *listener,
                        callback: *callback,
                        name: name.clone(),
                    },
                );
            }
            Command::RemoveEventListener {
                node,
                listener,
                callback,
                name,
            } => {
                let (native_listener, _) = self
                    .listeners
                    .iter()
                    .find(|(_, mapping)| {
                        mapping.node == *node
                            && mapping.listener == *listener
                            && mapping.callback == *callback
                            && mapping.name == *name
                    })
                    .ok_or_else(|| {
                        BridgeError::new(
                            Status::InvalidListener,
                            format!("native listener {} identity does not match", listener.get()),
                        )
                    })?;
                let native_listener = *native_listener;
                let native_node = self.native_node(*node)?;
                // SAFETY: The exact registered listener identity is borrowed for this call.
                let status = unsafe {
                    self.api.remove_event_listener.expect("validated function")(
                        renderer,
                        native_node,
                        native_listener,
                        callback.get(),
                        NativeUtf8::from_str(name),
                    )
                };
                self.require_host_success(status, "remove_event_listener")?;
                self.listeners.remove(&native_listener);
            }
        }
        Ok(())
    }

    fn ensure_new_node(&self, node: NodeId) -> Result<(), BridgeError> {
        if self.nodes.contains_key(&node) {
            Err(BridgeError::new(
                Status::InvalidOwnership,
                format!("native node {} already exists", node.get()),
            ))
        } else {
            Ok(())
        }
    }

    fn finish_create(
        &mut self,
        node: NodeId,
        native_node: NativeNodeHandle,
        status: NativeStatus,
        operation: &'static str,
    ) -> Result<(), BridgeError> {
        self.require_host_success(status, operation)?;
        if native_node == 0 || self.nodes.values().any(|handle| *handle == native_node) {
            self.poisoned = true;
            return Err(BridgeError::new(
                Status::HostError,
                format!("native {operation} returned an invalid node handle"),
            ));
        }
        self.nodes.insert(node, native_node);
        Ok(())
    }

    fn native_node(&self, node: NodeId) -> Result<NativeNodeHandle, BridgeError> {
        self.nodes.get(&node).copied().ok_or_else(|| {
            BridgeError::new(
                Status::InvalidOwnership,
                format!("invalid or stale native node {}", node.get()),
            )
        })
    }

    fn ensure_owner(&self) -> Result<(), BridgeError> {
        if self.owner != thread::current().id() {
            Err(BridgeError::new(
                Status::WrongThread,
                "NativeHost was called from a non-owner thread",
            ))
        } else {
            Ok(())
        }
    }

    fn ensure_usable(&self) -> Result<NativeRendererHandle, BridgeError> {
        self.ensure_owner()?;
        let renderer = self.renderer.ok_or_else(|| {
            BridgeError::new(Status::InvalidSession, "native renderer is released")
        })?;
        if self.poisoned {
            return Err(BridgeError::new(
                Status::HostError,
                "NativeHost is permanently poisoned",
            ));
        }
        Ok(renderer)
    }

    fn require_host_success(
        &mut self,
        status: NativeStatus,
        operation: &'static str,
    ) -> Result<(), BridgeError> {
        if status == NATIVE_STATUS_OK {
            Ok(())
        } else {
            self.poisoned = true;
            Err(native_error(status, operation))
        }
    }
}

impl Drop for NativeHost {
    fn drop(&mut self) {
        if self.owner != thread::current().id() {
            return;
        }
        if self.renderer.is_some() {
            let _ = self.release();
        }
    }
}

impl NativeUtf8 {
    fn from_str(value: &str) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len(),
        }
    }

    fn removed() -> Self {
        Self {
            data: ptr::null(),
            len: 0,
        }
    }
}

impl NativeBytes {
    fn from_slice(value: &[u8]) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len(),
        }
    }
}

fn native_error(status: NativeStatus, operation: &'static str) -> BridgeError {
    let status_kind = match status {
        NATIVE_STATUS_INVALID_ARGUMENT => Status::InvalidArgument,
        NATIVE_STATUS_INVALID_SESSION => Status::InvalidSession,
        NATIVE_STATUS_WRONG_THREAD => Status::WrongThread,
        NATIVE_STATUS_UNSUPPORTED => Status::Unsupported,
        NATIVE_STATUS_INVALID_OWNERSHIP => Status::InvalidOwnership,
        NATIVE_STATUS_INVALID_LISTENER => Status::InvalidListener,
        NATIVE_STATUS_RESOURCE_EXHAUSTED => Status::ResourceExhausted,
        NATIVE_STATUS_HOST_ERROR => Status::HostError,
        NATIVE_STATUS_PANIC => Status::Panic,
        NATIVE_STATUS_INTERNAL_ERROR => Status::InternalError,
        _ => Status::InternalError,
    };
    BridgeError::new(
        status_kind,
        format!("native {operation} failed with status {status}"),
    )
}

pub const fn status_to_native(status: Status) -> NativeStatus {
    match status {
        Status::Ok => NATIVE_STATUS_OK,
        Status::InvalidArgument => NATIVE_STATUS_INVALID_ARGUMENT,
        Status::InvalidSession => NATIVE_STATUS_INVALID_SESSION,
        Status::WrongThread => NATIVE_STATUS_WRONG_THREAD,
        Status::Unsupported => NATIVE_STATUS_UNSUPPORTED,
        Status::InvalidOwnership => NATIVE_STATUS_INVALID_OWNERSHIP,
        Status::InvalidListener => NATIVE_STATUS_INVALID_LISTENER,
        Status::ResourceExhausted => NATIVE_STATUS_RESOURCE_EXHAUSTED,
        Status::HostError => NATIVE_STATUS_HOST_ERROR,
        Status::Panic => NATIVE_STATUS_PANIC,
        Status::InternalError => NATIVE_STATUS_INTERNAL_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::mem;
    use std::ptr;

    use super::*;

    const TEST_HOST: NativeHostHandle = 55;
    const TEST_RENDERER: NativeRendererHandle = 77;
    const TEST_ROOT: NativeNodeHandle = 100;

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Call {
        Acquire(NativeHostHandle),
        Release(NativeRendererHandle),
        GetRoot(NativeRendererHandle),
        CreateElement(NativeRendererHandle, Vec<u8>, NativeNodeHandle),
        CreateRawText(NativeRendererHandle, Vec<u8>, NativeNodeHandle),
        SetAttribute(
            NativeRendererHandle,
            NativeNodeHandle,
            Vec<u8>,
            Option<Vec<u8>>,
        ),
        InsertBefore(
            NativeRendererHandle,
            NativeNodeHandle,
            NativeNodeHandle,
            NativeNodeHandle,
        ),
        RemoveChild(NativeRendererHandle, NativeNodeHandle, NativeNodeHandle),
        DestroyNode(NativeRendererHandle, NativeNodeHandle),
        AddListener(
            NativeRendererHandle,
            NativeNodeHandle,
            NativeListenerHandle,
            NativeCallbackHandle,
            Vec<u8>,
        ),
        RemoveListener(
            NativeRendererHandle,
            NativeNodeHandle,
            NativeListenerHandle,
            NativeCallbackHandle,
            Vec<u8>,
        ),
        Flush(NativeRendererHandle),
        CreateTimer(
            NativeRendererHandle,
            u64,
            u32,
            NativeCallbackHandle,
            NativeTimerHandle,
        ),
        CancelTimer(NativeRendererHandle, NativeTimerHandle),
        ImportStyleSheet(NativeRendererHandle, Vec<u8>),
        ClearStyleSheets(NativeRendererHandle),
    }

    struct Recorder {
        calls: Vec<Call>,
        requested_versions: Vec<u32>,
        next_node: NativeNodeHandle,
        timer_output: NativeTimerHandle,
        failure: Option<(&'static str, NativeStatus)>,
        null_api: bool,
    }

    impl Default for Recorder {
        fn default() -> Self {
            Self {
                calls: Vec::new(),
                requested_versions: Vec::new(),
                next_node: 200,
                timer_output: 300,
                failure: None,
                null_api: false,
            }
        }
    }

    thread_local! {
        static RECORDER: RefCell<Recorder> = RefCell::new(Recorder::default());
        static API: RefCell<NativeRendererApiV1> = RefCell::new(valid_api());
    }

    fn valid_api() -> NativeRendererApiV1 {
        NativeRendererApiV1 {
            abi_version: NATIVE_RENDERER_ABI_VERSION,
            struct_size: mem::size_of::<NativeRendererApiV1>(),
            acquire: Some(record_acquire),
            release: Some(record_release),
            get_root: Some(record_get_root),
            create_element: Some(record_create_element),
            create_raw_text: Some(record_create_raw_text),
            set_raw_text: Some(record_set_raw_text),
            set_attribute: Some(record_set_attribute),
            insert_before: Some(record_insert_before),
            remove_child: Some(record_remove_child),
            destroy_node: Some(record_destroy_node),
            add_event_listener: Some(record_add_listener),
            remove_event_listener: Some(record_remove_listener),
            flush: Some(record_flush),
            create_timer: Some(record_create_timer),
            cancel_timer: Some(record_cancel_timer),
            import_style_sheet: Some(record_import_style_sheet),
            clear_style_sheets: Some(record_clear_style_sheets),
        }
    }

    fn reset() {
        RECORDER.with(|recorder| *recorder.borrow_mut() = Recorder::default());
        API.with(|api| *api.borrow_mut() = valid_api());
    }

    fn clear_calls() {
        RECORDER.with(|recorder| recorder.borrow_mut().calls.clear());
    }

    fn calls() -> Vec<Call> {
        RECORDER.with(|recorder| recorder.borrow().calls.clone())
    }

    fn fail(operation: &'static str, status: NativeStatus) {
        RECORDER.with(|recorder| recorder.borrow_mut().failure = Some((operation, status)));
    }

    fn clear_failure() {
        RECORDER.with(|recorder| recorder.borrow_mut().failure = None);
    }

    fn record(operation: &'static str, call: Call) -> NativeStatus {
        RECORDER.with(|recorder| {
            let mut recorder = recorder.borrow_mut();
            recorder.calls.push(call);
            recorder
                .failure
                .filter(|(candidate, _)| *candidate == operation)
                .map_or(NATIVE_STATUS_OK, |(_, status)| status)
        })
    }

    unsafe fn bytes(span: NativeUtf8) -> Vec<u8> {
        if span.len == 0 {
            return Vec::new();
        }
        if span.data.is_null() {
            return Vec::new();
        }
        // SAFETY: Recording functions receive borrowed spans under the production ABI contract.
        unsafe { std::slice::from_raw_parts(span.data, span.len) }.to_vec()
    }

    unsafe extern "C" fn no_event(
        _: *mut c_void,
        _: NativeRendererHandle,
        _: NativeListenerHandle,
        _: NativeCallbackHandle,
        _: NativeUtf8,
        _: NativeUtf8,
        _: NativeBytes,
    ) -> NativeStatus {
        NATIVE_STATUS_OK
    }

    unsafe extern "C" fn no_timer(
        _: *mut c_void,
        _: NativeRendererHandle,
        _: NativeTimerHandle,
        _: NativeCallbackHandle,
    ) -> NativeStatus {
        NATIVE_STATUS_OK
    }

    unsafe extern "C" fn get_api(version: u32) -> *const NativeRendererApiV1 {
        let null_api = RECORDER.with(|recorder| {
            let mut recorder = recorder.borrow_mut();
            recorder.requested_versions.push(version);
            recorder.null_api
        });
        if null_api {
            ptr::null()
        } else {
            API.with(RefCell::as_ptr).cast_const()
        }
    }

    unsafe extern "C" fn record_acquire(
        host: NativeHostHandle,
        _: *const NativeRendererCallbacksV1,
        renderer: *mut NativeRendererHandle,
    ) -> NativeStatus {
        let status = record("acquire", Call::Acquire(host));
        if status == NATIVE_STATUS_OK && !renderer.is_null() {
            // SAFETY: NativeHost supplies a writable output pointer.
            unsafe { *renderer = TEST_RENDERER };
        }
        status
    }

    unsafe extern "C" fn record_release(renderer: NativeRendererHandle) -> NativeStatus {
        record("release", Call::Release(renderer))
    }

    unsafe extern "C" fn record_get_root(
        renderer: NativeRendererHandle,
        root: *mut NativeNodeHandle,
    ) -> NativeStatus {
        let status = record("get_root", Call::GetRoot(renderer));
        if status == NATIVE_STATUS_OK && !root.is_null() {
            // SAFETY: NativeHost supplies a writable output pointer.
            unsafe { *root = TEST_ROOT };
        }
        status
    }

    unsafe extern "C" fn record_create_element(
        renderer: NativeRendererHandle,
        tag: NativeUtf8,
        node: *mut NativeNodeHandle,
    ) -> NativeStatus {
        let native_node = RECORDER.with(|recorder| recorder.borrow().next_node);
        // SAFETY: The borrowed span is valid for this call.
        let status = record(
            "create_element",
            Call::CreateElement(renderer, unsafe { bytes(tag) }, native_node),
        );
        if status == NATIVE_STATUS_OK {
            RECORDER.with(|recorder| recorder.borrow_mut().next_node += 1);
            if !node.is_null() {
                // SAFETY: NativeHost supplies a writable output pointer.
                unsafe { *node = native_node };
            }
        }
        status
    }

    unsafe extern "C" fn record_create_raw_text(
        renderer: NativeRendererHandle,
        text: NativeUtf8,
        node: *mut NativeNodeHandle,
    ) -> NativeStatus {
        let native_node = RECORDER.with(|recorder| recorder.borrow().next_node);
        // SAFETY: The borrowed span is valid for this call.
        let status = record(
            "create_raw_text",
            Call::CreateRawText(renderer, unsafe { bytes(text) }, native_node),
        );
        if status == NATIVE_STATUS_OK {
            RECORDER.with(|recorder| recorder.borrow_mut().next_node += 1);
            if !node.is_null() {
                // SAFETY: NativeHost supplies a writable output pointer.
                unsafe { *node = native_node };
            }
        }
        status
    }

    unsafe extern "C" fn record_set_raw_text(
        _: NativeRendererHandle,
        _: NativeNodeHandle,
        _: NativeUtf8,
    ) -> NativeStatus {
        NATIVE_STATUS_OK
    }

    unsafe extern "C" fn record_set_attribute(
        renderer: NativeRendererHandle,
        node: NativeNodeHandle,
        name: NativeUtf8,
        value: NativeUtf8,
    ) -> NativeStatus {
        // SAFETY: Both borrowed spans are valid for this call.
        let name = unsafe { bytes(name) };
        let value = if value.data.is_null() {
            None
        } else {
            // SAFETY: A nonnull optional value is a valid borrowed span.
            Some(unsafe { bytes(value) })
        };
        record(
            "set_attribute",
            Call::SetAttribute(renderer, node, name, value),
        )
    }

    unsafe extern "C" fn record_insert_before(
        renderer: NativeRendererHandle,
        parent: NativeNodeHandle,
        child: NativeNodeHandle,
        reference: NativeNodeHandle,
    ) -> NativeStatus {
        record(
            "insert_before",
            Call::InsertBefore(renderer, parent, child, reference),
        )
    }

    unsafe extern "C" fn record_remove_child(
        renderer: NativeRendererHandle,
        parent: NativeNodeHandle,
        child: NativeNodeHandle,
    ) -> NativeStatus {
        record("remove_child", Call::RemoveChild(renderer, parent, child))
    }

    unsafe extern "C" fn record_destroy_node(
        renderer: NativeRendererHandle,
        node: NativeNodeHandle,
    ) -> NativeStatus {
        record("destroy_node", Call::DestroyNode(renderer, node))
    }

    unsafe extern "C" fn record_add_listener(
        renderer: NativeRendererHandle,
        node: NativeNodeHandle,
        listener: NativeListenerHandle,
        callback: NativeCallbackHandle,
        name: NativeUtf8,
    ) -> NativeStatus {
        // SAFETY: The borrowed span is valid for this call.
        let name = unsafe { bytes(name) };
        record(
            "add_event_listener",
            Call::AddListener(renderer, node, listener, callback, name),
        )
    }

    unsafe extern "C" fn record_remove_listener(
        renderer: NativeRendererHandle,
        node: NativeNodeHandle,
        listener: NativeListenerHandle,
        callback: NativeCallbackHandle,
        name: NativeUtf8,
    ) -> NativeStatus {
        // SAFETY: The borrowed span is valid for this call.
        let name = unsafe { bytes(name) };
        record(
            "remove_event_listener",
            Call::RemoveListener(renderer, node, listener, callback, name),
        )
    }

    unsafe extern "C" fn record_flush(renderer: NativeRendererHandle) -> NativeStatus {
        record("flush", Call::Flush(renderer))
    }

    unsafe extern "C" fn record_import_style_sheet(
        renderer: NativeRendererHandle,
        fragment: NativeBytes,
    ) -> NativeStatus {
        let fragment = if fragment.len == 0 {
            Vec::new()
        } else {
            // SAFETY: NativeHost provides a valid borrowed span for this call.
            unsafe { std::slice::from_raw_parts(fragment.data, fragment.len) }.to_vec()
        };
        record(
            "import_style_sheet",
            Call::ImportStyleSheet(renderer, fragment),
        )
    }

    unsafe extern "C" fn record_clear_style_sheets(renderer: NativeRendererHandle) -> NativeStatus {
        record("clear_style_sheets", Call::ClearStyleSheets(renderer))
    }

    unsafe extern "C" fn record_create_timer(
        renderer: NativeRendererHandle,
        delay_millis: u64,
        repeating: u32,
        callback: NativeCallbackHandle,
        timer: *mut NativeTimerHandle,
    ) -> NativeStatus {
        let native_timer = RECORDER.with(|recorder| recorder.borrow().timer_output);
        let status = record(
            "create_timer",
            Call::CreateTimer(renderer, delay_millis, repeating, callback, native_timer),
        );
        if status == NATIVE_STATUS_OK && !timer.is_null() {
            // SAFETY: NativeHost supplies a writable output pointer.
            unsafe { *timer = native_timer };
        }
        status
    }

    unsafe extern "C" fn record_cancel_timer(
        renderer: NativeRendererHandle,
        timer: NativeTimerHandle,
    ) -> NativeStatus {
        record("cancel_timer", Call::CancelTimer(renderer, timer))
    }

    fn callbacks() -> NativeRendererCallbacksV1 {
        NativeRendererCallbacksV1 {
            context: ptr::null_mut(),
            on_event: Some(no_event),
            on_timer: Some(no_timer),
        }
    }

    fn acquire_host(_session: u32, root: u32) -> Result<NativeHost, BridgeError> {
        // SAFETY: The recording table implements the production ABI for the test lifetime.
        unsafe { NativeHost::acquire(get_api, TEST_HOST, NodeId::new(root).unwrap(), callbacks()) }
    }

    fn recording_host(session: u32, root: u32) -> NativeHost {
        reset();
        acquire_host(session, root).unwrap_or_else(|error| panic!("acquire failed: {error}"))
    }

    fn item(command: Command) -> Command {
        command
    }

    fn batch(
        _session: u32,
        sequence: u32,
        commands: Vec<Command>,
        final_commit: bool,
    ) -> CommandBatch {
        CommandBatch {
            sequence,
            commands,
            final_commit,
        }
    }

    fn error_status<T>(result: Result<T, BridgeError>) -> Status {
        result.err().expect("expected an error").status
    }

    #[test]
    fn validates_api_version_size_and_required_functions_before_acquire() {
        reset();
        RECORDER.with(|recorder| recorder.borrow_mut().null_api = true);
        assert_eq!(error_status(acquire_host(1, 1)), Status::Unsupported);
        assert!(calls().is_empty());

        reset();
        API.with(|api| api.borrow_mut().abi_version = 2);
        assert_eq!(error_status(acquire_host(1, 1)), Status::Unsupported);
        assert!(calls().is_empty());

        reset();
        API.with(|api| api.borrow_mut().struct_size = mem::size_of::<NativeRendererApiV1>() - 1);
        assert_eq!(error_status(acquire_host(1, 1)), Status::Unsupported);
        assert!(calls().is_empty());

        reset();
        API.with(|api| api.borrow_mut().flush = None);
        assert_eq!(error_status(acquire_host(1, 1)), Status::Unsupported);
        assert!(calls().is_empty());

        reset();
        API.with(|api| api.borrow_mut().clear_style_sheets = None);
        assert_eq!(error_status(acquire_host(1, 1)), Status::Unsupported);
        assert!(calls().is_empty());

        reset();
        API.with(|api| api.borrow_mut().struct_size += mem::size_of::<usize>());
        let mut host = acquire_host(1, 1).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            RECORDER.with(|recorder| recorder.borrow().requested_versions.clone()),
            vec![NATIVE_RENDERER_ABI_VERSION]
        );
        host.release().unwrap();
    }

    #[test]
    fn maps_root_and_applies_mutations_in_order_before_one_final_flush() {
        let mut host = recording_host(9, 1);
        clear_calls();
        let element = NodeId::new(2).unwrap();
        let text = NodeId::new(3).unwrap();
        let listener = ListenerId::new(4).unwrap();
        let callback = CallbackId::new(5).unwrap();
        let commands = vec![
            item(Command::ImportStyleSheet {
                fragment: vec![0, 127, 255],
            }),
            item(Command::CreateElement {
                node: element,
                tag: "vi\0ew".into(),
            }),
            item(Command::CreateRawText {
                node: text,
                text: "te\0xt".into(),
            }),
            item(Command::SetAttribute {
                node: element,
                name: "class".into(),
                value: Some("hero".into()),
            }),
            item(Command::AppendElement {
                parent: NodeId::new(1).unwrap(),
                child: element,
            }),
            item(Command::AppendElement {
                parent: element,
                child: text,
            }),
            item(Command::AddEventListener {
                node: element,
                listener,
                callback,
                name: "ta\0p".into(),
            }),
            item(Command::RemoveEventListener {
                node: element,
                listener,
                callback,
                name: "ta\0p".into(),
            }),
            item(Command::RemoveElement {
                parent: element,
                child: text,
            }),
            item(Command::DestroyNode { node: text }),
            item(Command::RemoveElement {
                parent: NodeId::new(1).unwrap(),
                child: element,
            }),
            item(Command::DestroyNode { node: element }),
        ];

        host.apply(&batch(9, 1, commands, true)).unwrap();

        assert_eq!(
            calls(),
            vec![
                Call::ImportStyleSheet(TEST_RENDERER, vec![0, 127, 255]),
                Call::CreateElement(TEST_RENDERER, b"vi\0ew".to_vec(), 200),
                Call::CreateRawText(TEST_RENDERER, b"te\0xt".to_vec(), 201),
                Call::SetAttribute(
                    TEST_RENDERER,
                    200,
                    b"class".to_vec(),
                    Some(b"hero".to_vec())
                ),
                Call::InsertBefore(TEST_RENDERER, TEST_ROOT, 200, 0),
                Call::InsertBefore(TEST_RENDERER, 200, 201, 0),
                Call::AddListener(TEST_RENDERER, 200, 1, 5, b"ta\0p".to_vec()),
                Call::RemoveListener(TEST_RENDERER, 200, 1, 5, b"ta\0p".to_vec()),
                Call::RemoveChild(TEST_RENDERER, 200, 201),
                Call::DestroyNode(TEST_RENDERER, 201),
                Call::RemoveChild(TEST_RENDERER, TEST_ROOT, 200),
                Call::DestroyNode(TEST_RENDERER, 200),
                Call::Flush(TEST_RENDERER),
            ]
        );
        host.release().unwrap();
    }

    #[test]
    fn flushes_only_final_batches() {
        let mut host = recording_host(1, 1);
        clear_calls();

        host.apply(&batch(1, 1, Vec::new(), false)).unwrap();
        assert!(calls().is_empty());
        host.apply(&batch(1, 2, Vec::new(), true)).unwrap();
        assert_eq!(calls(), vec![Call::Flush(TEST_RENDERER)]);
        host.release().unwrap();
    }

    #[test]
    fn resets_sequence_only_after_application_resources_are_gone() {
        let mut host = recording_host(1, 1);
        host.apply(&batch(1, 5, Vec::new(), false)).unwrap();
        clear_calls();
        host.reset_application_epoch().unwrap();
        assert_eq!(calls(), vec![Call::ClearStyleSheets(TEST_RENDERER)]);
        host.apply(&batch(1, 1, Vec::new(), false)).unwrap();

        host.apply(&batch(
            1,
            2,
            vec![Command::CreateElement {
                node: NodeId::new(2).unwrap(),
                tag: "view".into(),
            }],
            false,
        ))
        .unwrap();
        assert_eq!(
            error_status(host.reset_application_epoch()),
            Status::InvalidOwnership
        );
        assert_eq!(
            calls()
                .iter()
                .filter(|call| matches!(call, Call::ClearStyleSheets(_)))
                .count(),
            1
        );
        host.release().unwrap();
    }

    #[test]
    fn application_epoch_reset_propagates_stylesheet_clear_failure() {
        let mut host = recording_host(1, 1);
        host.apply(&batch(1, 5, Vec::new(), false)).unwrap();
        fail("clear_style_sheets", NATIVE_STATUS_HOST_ERROR);

        assert_eq!(
            error_status(host.reset_application_epoch()),
            Status::HostError
        );
        assert_eq!(calls().last(), Some(&Call::ClearStyleSheets(TEST_RENDERER)));
        assert_eq!(
            error_status(host.apply(&batch(1, 1, Vec::new(), false))),
            Status::HostError
        );
        host.release().unwrap();
    }

    #[test]
    fn application_epoch_reset_does_not_reuse_native_listener_handles() {
        let mut host = recording_host(1, 1);
        let node = NodeId::new(2).unwrap();
        let listener = ListenerId::new(1).unwrap();
        let callback = CallbackId::new(1).unwrap();
        host.apply(&batch(
            1,
            1,
            vec![
                Command::CreateElement {
                    node,
                    tag: "view".into(),
                },
                Command::AddEventListener {
                    node,
                    listener,
                    callback,
                    name: "tap".into(),
                },
            ],
            false,
        ))
        .unwrap();
        assert!(
            host.event_message(
                TEST_RENDERER,
                1,
                callback.get(),
                "tap",
                String::new(),
                Vec::new(),
            )
            .is_ok()
        );

        host.apply(&batch(
            1,
            2,
            vec![
                Command::RemoveEventListener {
                    node,
                    listener,
                    callback,
                    name: "tap".into(),
                },
                Command::DestroyNode { node },
            ],
            false,
        ))
        .unwrap();
        host.reset_application_epoch().unwrap();
        clear_calls();

        host.apply(&batch(
            1,
            1,
            vec![
                Command::CreateElement {
                    node,
                    tag: "view".into(),
                },
                Command::AddEventListener {
                    node,
                    listener,
                    callback,
                    name: "tap".into(),
                },
            ],
            false,
        ))
        .unwrap();

        assert!(matches!(
            calls().as_slice(),
            [
                Call::CreateElement(_, _, _),
                Call::AddListener(_, _, 2, 1, _)
            ]
        ));
        assert_eq!(
            error_status(host.event_message(
                TEST_RENDERER,
                1,
                callback.get(),
                "tap",
                String::new(),
                Vec::new(),
            )),
            Status::InvalidListener
        );
        let event = host
            .event_message(
                TEST_RENDERER,
                2,
                callback.get(),
                "tap",
                String::new(),
                Vec::new(),
            )
            .unwrap();
        assert_eq!(event.listener, listener);
        host.release().unwrap();
    }

    #[test]
    fn reports_native_listener_handle_exhaustion_without_reusing_handles() {
        let mut host = recording_host(1, 1);
        host.next_native_listener = Some(u32::MAX);
        let node = NodeId::new(2).unwrap();
        host.apply(&batch(
            1,
            1,
            vec![
                Command::CreateElement {
                    node,
                    tag: "view".into(),
                },
                Command::AddEventListener {
                    node,
                    listener: ListenerId::new(1).unwrap(),
                    callback: CallbackId::new(1).unwrap(),
                    name: "tap".into(),
                },
            ],
            false,
        ))
        .unwrap();
        clear_calls();

        assert_eq!(
            error_status(host.apply(&batch(
                1,
                2,
                vec![Command::AddEventListener {
                    node,
                    listener: ListenerId::new(2).unwrap(),
                    callback: CallbackId::new(2).unwrap(),
                    name: "tap".into(),
                }],
                false,
            ))),
            Status::ResourceExhausted
        );
        assert!(calls().is_empty());
        host.release().unwrap();
    }

    #[test]
    fn maps_a_nonzero_insert_reference() {
        let mut host = recording_host(1, 1);
        let first = NodeId::new(2).unwrap();
        let second = NodeId::new(3).unwrap();
        host.apply(&batch(
            1,
            1,
            vec![
                item(Command::CreateElement {
                    node: first,
                    tag: "first".into(),
                }),
                item(Command::CreateElement {
                    node: second,
                    tag: "second".into(),
                }),
            ],
            false,
        ))
        .unwrap();
        clear_calls();

        host.apply(&batch(
            1,
            2,
            vec![item(Command::InsertElementBefore {
                parent: NodeId::new(1).unwrap(),
                child: second,
                reference: first,
            })],
            false,
        ))
        .unwrap();

        assert_eq!(
            calls(),
            vec![Call::InsertBefore(TEST_RENDERER, TEST_ROOT, 201, 200)]
        );
        host.release().unwrap();
    }

    #[test]
    fn maps_native_statuses_and_poison_rejects_later_use() {
        let expected = [
            Status::InvalidArgument,
            Status::InvalidSession,
            Status::WrongThread,
            Status::Unsupported,
            Status::InvalidOwnership,
            Status::InvalidListener,
            Status::ResourceExhausted,
            Status::HostError,
            Status::Panic,
            Status::InternalError,
        ];
        for (native_status, expected_status) in (1..=10).zip(expected) {
            let mut host = recording_host(1, 1);
            clear_calls();
            fail("flush", native_status);
            assert_eq!(
                error_status(host.apply(&batch(1, 1, Vec::new(), true))),
                expected_status
            );
            clear_failure();
            clear_calls();
            assert_eq!(
                error_status(host.apply(&batch(1, 2, Vec::new(), true))),
                Status::HostError
            );
            assert!(calls().is_empty());
            host.release().unwrap();
        }
    }

    #[test]
    fn rejects_wrong_thread_without_touching_the_native_table() {
        let mut host = recording_host(1, 1);
        clear_calls();
        let host_address = ptr::from_mut(&mut host) as usize;
        let wrong_thread = std::thread::spawn(move || {
            // SAFETY: The owner does not access the host until this worker has joined.
            let host = unsafe { &mut *(host_address as *mut NativeHost) };
            error_status(host.apply(&batch(1, 1, Vec::new(), true)))
        });

        assert_eq!(wrong_thread.join().unwrap(), Status::WrongThread);
        assert!(calls().is_empty());
        host.apply(&batch(1, 1, Vec::new(), true)).unwrap();
        assert_eq!(calls(), vec![Call::Flush(TEST_RENDERER)]);
        host.release().unwrap();
    }

    #[test]
    fn rejects_nonincreasing_sequences() {
        let mut host = recording_host(7, 1);
        clear_calls();

        host.apply(&batch(7, 10, Vec::new(), false)).unwrap();
        assert_eq!(
            error_status(host.apply(&batch(7, 10, Vec::new(), false))),
            Status::InvalidArgument
        );
        assert_eq!(
            error_status(host.apply(&batch(7, 9, Vec::new(), false))),
            Status::InvalidArgument
        );
        host.apply(&batch(7, 11, Vec::new(), false)).unwrap();
        assert!(calls().is_empty());
        host.release().unwrap();
    }

    #[test]
    fn duplicate_release_is_rejected_without_a_second_native_release() {
        let mut host = recording_host(1, 1);
        clear_calls();

        host.release().unwrap();
        assert_eq!(calls(), vec![Call::Release(TEST_RENDERER)]);
        clear_calls();
        assert_eq!(error_status(host.release()), Status::InvalidSession);
        assert!(calls().is_empty());
    }

    #[test]
    fn distinguishes_empty_and_removed_attributes_and_preserves_nul_bytes() {
        let mut host = recording_host(1, 1);
        host.apply(&batch(
            1,
            1,
            vec![item(Command::CreateElement {
                node: NodeId::new(2).unwrap(),
                tag: "view".into(),
            })],
            false,
        ))
        .unwrap();
        clear_calls();

        host.apply(&batch(
            1,
            2,
            vec![
                item(Command::SetAttribute {
                    node: NodeId::new(2).unwrap(),
                    name: "na\0me".into(),
                    value: Some(String::new()),
                }),
                item(Command::SetAttribute {
                    node: NodeId::new(2).unwrap(),
                    name: "na\0me".into(),
                    value: None,
                }),
            ],
            false,
        ))
        .unwrap();

        assert_eq!(
            calls(),
            vec![
                Call::SetAttribute(TEST_RENDERER, 200, b"na\0me".to_vec(), Some(Vec::new())),
                Call::SetAttribute(TEST_RENDERER, 200, b"na\0me".to_vec(), None),
            ]
        );
        host.release().unwrap();
    }

    #[test]
    fn listener_removal_requires_the_exact_registered_identity() {
        let mut host = recording_host(1, 1);
        let node = NodeId::new(2).unwrap();
        let listener = ListenerId::new(3).unwrap();
        let callback = CallbackId::new(4).unwrap();
        host.apply(&batch(
            1,
            1,
            vec![
                item(Command::CreateElement {
                    node,
                    tag: "view".into(),
                }),
                item(Command::AddEventListener {
                    node,
                    listener,
                    callback,
                    name: "tap".into(),
                }),
            ],
            false,
        ))
        .unwrap();
        clear_calls();

        let wrong_identity = batch(
            1,
            2,
            vec![item(Command::RemoveEventListener {
                node,
                listener,
                callback: CallbackId::new(5).unwrap(),
                name: "tap".into(),
            })],
            false,
        );
        assert_eq!(
            error_status(host.apply(&wrong_identity)),
            Status::InvalidListener
        );
        assert!(calls().is_empty());

        host.apply(&batch(
            1,
            2,
            vec![item(Command::RemoveEventListener {
                node,
                listener,
                callback,
                name: "tap".into(),
            })],
            false,
        ))
        .unwrap();
        assert_eq!(
            calls(),
            vec![Call::RemoveListener(
                TEST_RENDERER,
                200,
                1,
                callback.get(),
                b"tap".to_vec()
            )]
        );
        host.release().unwrap();
    }

    #[test]
    fn validates_timer_creation_and_cancellation() {
        let mut host = recording_host(1, 1);
        clear_calls();
        let callback = CallbackId::new(9).unwrap();

        let timer = host.create_timer(25, true, callback).unwrap();
        assert_eq!(timer, 300);
        host.cancel_timer(timer).unwrap();
        assert_eq!(
            calls(),
            vec![
                Call::CreateTimer(TEST_RENDERER, 25, 1, 9, 300),
                Call::CancelTimer(TEST_RENDERER, 300),
            ]
        );
        clear_calls();
        assert_eq!(
            error_status(host.cancel_timer(timer)),
            Status::InvalidArgument
        );
        assert_eq!(error_status(host.cancel_timer(0)), Status::InvalidArgument);
        assert!(calls().is_empty());
        host.release().unwrap();

        let mut host = recording_host(1, 1);
        clear_calls();
        RECORDER.with(|recorder| recorder.borrow_mut().timer_output = 0);
        assert_eq!(
            error_status(host.create_timer(0, false, callback)),
            Status::HostError
        );
        clear_calls();
        assert_eq!(
            error_status(host.create_timer(0, false, callback)),
            Status::HostError
        );
        assert!(calls().is_empty());
        host.release().unwrap();
    }

    #[test]
    fn timer_callbacks_require_exact_identity_and_consume_only_one_shots() {
        let mut host = recording_host(1, 1);
        clear_calls();
        let callback = CallbackId::new(9).unwrap();
        let timer = host.create_timer(25, false, callback).unwrap();

        assert_eq!(
            error_status(host.timer_callback(TEST_RENDERER + 1, timer, callback.get())),
            Status::InvalidSession
        );
        assert_eq!(
            error_status(host.timer_callback(TEST_RENDERER, timer + 1, callback.get())),
            Status::InvalidArgument
        );
        assert_eq!(
            error_status(host.timer_callback(TEST_RENDERER, timer, callback.get() + 1)),
            Status::InvalidArgument
        );
        assert_eq!(
            host.timer_callback(TEST_RENDERER, timer, callback.get())
                .unwrap(),
            callback
        );
        assert_eq!(
            error_status(host.timer_callback(TEST_RENDERER, timer, callback.get())),
            Status::InvalidArgument
        );

        let repeating_callback = CallbackId::new(10).unwrap();
        let repeating = host.create_timer(50, true, repeating_callback).unwrap();
        assert_eq!(
            host.timer_callback(TEST_RENDERER, repeating, repeating_callback.get())
                .unwrap(),
            repeating_callback
        );
        assert_eq!(
            host.timer_callback(TEST_RENDERER, repeating, repeating_callback.get())
                .unwrap(),
            repeating_callback
        );
        host.cancel_timer(repeating).unwrap();
        host.release().unwrap();
    }

    #[test]
    fn release_forgets_live_timers_and_releases_the_renderer_once() {
        let mut host = recording_host(1, 1);
        clear_calls();
        host.create_timer(25, false, CallbackId::new(9).unwrap())
            .unwrap();
        RECORDER.with(|recorder| recorder.borrow_mut().timer_output = 301);
        host.create_timer(50, true, CallbackId::new(10).unwrap())
            .unwrap();
        assert_eq!(host.timers.len(), 2);
        clear_calls();

        host.release().unwrap();

        assert!(host.timers.is_empty());
        assert_eq!(calls(), vec![Call::Release(TEST_RENDERER)]);
    }

    #[test]
    fn uses_the_copied_function_table_after_acquire() {
        let mut host = recording_host(1, 1);
        clear_calls();
        API.with(|api| api.borrow_mut().flush = None);

        host.apply(&batch(1, 1, Vec::new(), true)).unwrap();

        assert_eq!(calls(), vec![Call::Flush(TEST_RENDERER)]);
        host.release().unwrap();
    }
}
