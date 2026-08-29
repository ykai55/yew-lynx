use std::rc::Rc;

use dioxus_core::{Event, VirtualDom};
#[cfg(any(target_arch = "wasm32", test))]
use lynx_element_bridge_core::BridgeError;
use lynx_element_bridge_core::{CommandBatch, EventMessage, NodeId, Status};
use lynx_element_bridge_dioxus::{DioxusAdapter, DioxusAdapterError};
#[cfg(not(target_arch = "wasm32"))]
use lynx_element_bridge_ffi::native_host::{NativeHostHandle, NativeRendererGetApiFn};
#[cfg(not(target_arch = "wasm32"))]
use lynx_element_bridge_ffi::{
    BackendError, BridgeBackend, LynxElementBridgeNativeDestroyResult,
    LynxElementBridgeNativeMountResult, LynxElementBridgeSession,
};
#[cfg(any(target_arch = "wasm32", test))]
use lynx_element_bridge_wasm_guest::{GuestApplication, MountRequest};

use crate::INITIAL_COUNT;
use crate::app::{CounterModel, counter};

pub struct DioxusCounter {
    dom: VirtualDom,
    adapter: DioxusAdapter,
}

impl DioxusCounter {
    pub fn mount(root: NodeId) -> Result<(Self, CommandBatch), DioxusAdapterError> {
        let mut adapter = DioxusAdapter::new(root)?;
        let model = Rc::new(CounterModel::new(INITIAL_COUNT));
        let mut dom = VirtualDom::new_with_props(counter, Rc::clone(&model));
        dom.rebuild(&mut adapter);
        let batch = adapter.take_batch()?;
        Ok((Self { dom, adapter }, batch))
    }

    pub fn dispatch(&mut self, event: EventMessage) -> Result<CommandBatch, DioxusAdapterError> {
        let (target, name) = self.adapter.resolve_event(&event)?;
        self.dom
            .runtime()
            .handle_event(name, Event::new(Rc::new(event.payload), true), target);
        self.dom.render_immediate(&mut self.adapter);
        self.adapter.take_batch()
    }

    pub fn discard_pending(&mut self) {
        self.adapter.discard_pending();
    }

    pub fn destroy(mut self) -> Result<CommandBatch, DioxusAdapterError> {
        drop(self.dom);
        self.adapter.destroy()
    }
}

#[cfg(any(target_arch = "wasm32", test))]
impl GuestApplication for DioxusCounter {
    fn mount(request: MountRequest) -> Result<(Self, CommandBatch), BridgeError> {
        DioxusCounter::mount(request.root).map_err(guest_error)
    }

    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BridgeError> {
        self.dispatch(event).map_err(guest_error)
    }

    fn destroy(self) -> Result<CommandBatch, BridgeError> {
        DioxusCounter::destroy(self).map_err(guest_error)
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn guest_error(error: DioxusAdapterError) -> BridgeError {
    match error {
        DioxusAdapterError::Bridge(error) => error,
        DioxusAdapterError::InvalidListener(_) | DioxusAdapterError::EventMismatch { .. } => {
            BridgeError::new(Status::InvalidListener, error.to_string())
        }
        DioxusAdapterError::CallbackExhausted => {
            BridgeError::new(Status::ResourceExhausted, error.to_string())
        }
        DioxusAdapterError::InvalidElement(_)
        | DioxusAdapterError::InvalidStack(_)
        | DioxusAdapterError::InvalidTemplatePath(_)
        | DioxusAdapterError::UnsupportedAttribute => {
            BridgeError::new(Status::HostError, error.to_string())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct DioxusBackend(Option<DioxusCounter>);

#[cfg(not(target_arch = "wasm32"))]
impl BridgeBackend for DioxusBackend {
    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BackendError> {
        self.0
            .as_mut()
            .ok_or_else(|| BackendError::fatal(Status::InternalError, "Dioxus backend is empty"))?
            .dispatch(event)
            .map_err(adapter_error)
    }

    fn destroy(mut self: Box<Self>, _: bool) -> Result<CommandBatch, BackendError> {
        self.0
            .take()
            .ok_or_else(|| BackendError::fatal(Status::InternalError, "Dioxus backend is empty"))?
            .destroy()
            .map_err(adapter_error)
    }

    fn discard_pending(&mut self) {
        if let Some(counter) = self.0.as_mut() {
            counter.discard_pending();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn adapter_error(error: DioxusAdapterError) -> BackendError {
    match error {
        DioxusAdapterError::Bridge(error) if error.status == Status::InvalidListener => {
            BackendError::recoverable(error.status, error.message)
        }
        DioxusAdapterError::Bridge(error) => BackendError::from(error),
        DioxusAdapterError::InvalidListener(_) | DioxusAdapterError::EventMismatch { .. } => {
            BackendError::recoverable(Status::InvalidListener, error.to_string())
        }
        DioxusAdapterError::CallbackExhausted => {
            BackendError::fatal(Status::ResourceExhausted, error.to_string())
        }
        DioxusAdapterError::InvalidElement(_)
        | DioxusAdapterError::InvalidStack(_)
        | DioxusAdapterError::InvalidTemplatePath(_)
        | DioxusAdapterError::UnsupportedAttribute => {
            BackendError::fatal(Status::HostError, error.to_string())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn create_backend(root: NodeId) -> Result<(Box<dyn BridgeBackend>, CommandBatch), BackendError> {
    let (counter, batch) = DioxusCounter::mount(root).map_err(adapter_error)?;
    Ok((Box::new(DioxusBackend(Some(counter))), batch))
}

/// Mounts the counter directly through the native renderer function table.
///
/// # Safety
///
/// `get_api` and `host` must obey the native renderer C ABI for the mounted session lifetime.
#[unsafe(no_mangle)]
#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)]
pub unsafe extern "C" fn lynx_element_bridge_native_mount(
    get_api: Option<NativeRendererGetApiFn>,
    host: NativeHostHandle,
) -> LynxElementBridgeNativeMountResult {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_ffi::native_mount(get_api, host, create_backend) }
}

#[unsafe(no_mangle)]
#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)]
pub extern "C" fn lynx_element_bridge_native_destroy_session(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    lynx_element_bridge_ffi::native_destroy_session(session)
}

#[unsafe(no_mangle)]
#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)]
pub extern "C" fn lynx_element_bridge_native_abandon_session(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    lynx_element_bridge_ffi::native_abandon_session(session)
}

#[unsafe(no_mangle)]
#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)]
pub extern "C" fn lynx_element_bridge_backend() -> *const std::ffi::c_char {
    c"dioxus".as_ptr()
}

#[unsafe(no_mangle)]
#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)]
pub extern "C" fn lynx_element_bridge_backend_marker() -> *const std::ffi::c_char {
    c"lynx-element-bridge-backend:dioxus".as_ptr()
}

#[cfg(test)]
mod tests {
    use lynx_element_bridge_core::{CallbackId, Command, HostFake, ListenerId};

    use super::*;

    #[test]
    fn fixture_virtual_dom_mounts_updates_and_destroys_the_counter() {
        let root = NodeId::new(1).unwrap();
        let (mut counter, mounted) = DioxusCounter::mount(root).unwrap();
        let (listener, callback) = mounted
            .commands
            .iter()
            .find_map(|command| match command {
                Command::AddEventListener {
                    listener, callback, ..
                } => Some((*listener, *callback)),
                _ => None,
            })
            .unwrap();
        let mut host = HostFake::new(root);
        host.apply(&mounted).unwrap();
        {
            let snapshot = host.snapshot();
            let screen = &snapshot.children[0];
            assert_eq!(screen.tag, "view");
            assert_eq!(screen.children[0].tag, "text");
            assert_eq!(screen.children[1].tag, "view");
            assert_eq!(screen.children[1].children[0].tag, "text");
            assert_eq!(
                screen.children[1].children[0].children[0].text.as_deref(),
                Some("Increment")
            );
        }
        assert_eq!(
            host.snapshot().children[0].children[0].children[0]
                .text
                .as_deref(),
            Some(format!("Count: {INITIAL_COUNT}").as_str())
        );
        let updated = counter
            .dispatch(EventMessage {
                listener,
                callback,
                content_type: "application/vnd.lynx.tap".into(),
                payload: vec![0, 255],
            })
            .unwrap();
        host.apply(&updated).unwrap();
        assert_eq!(
            host.snapshot().children[0].children[0].children[0]
                .text
                .as_deref(),
            Some(format!("Count: {}", INITIAL_COUNT + 1).as_str())
        );

        let mismatch = counter.dispatch(EventMessage {
            listener,
            callback: CallbackId::new(callback.get() + 1).unwrap(),
            content_type: "application/vnd.lynx.tap".into(),
            payload: Vec::new(),
        });
        assert!(matches!(
            mismatch,
            Err(DioxusAdapterError::EventMismatch { .. })
        ));
        let updated = counter
            .dispatch(EventMessage {
                listener,
                callback,
                content_type: "application/vnd.lynx.tap".into(),
                payload: Vec::new(),
            })
            .unwrap();
        host.apply(&updated).unwrap();
        assert_eq!(
            host.snapshot().children[0].children[0].children[0]
                .text
                .as_deref(),
            Some(format!("Count: {}", INITIAL_COUNT + 2).as_str())
        );

        let destroyed = counter.destroy().unwrap();
        host.apply(&destroyed).unwrap();
        assert!(host.snapshot().children.is_empty());
        assert_eq!(host.listener_count(), 0);
    }

    #[test]
    fn fixture_rejects_unknown_listener_without_updating() {
        let root = NodeId::new(1).unwrap();
        let (mut counter, mounted) = DioxusCounter::mount(root).unwrap();
        let callback = mounted
            .commands
            .iter()
            .find_map(|command| match command {
                Command::AddEventListener { callback, .. } => Some(*callback),
                _ => None,
            })
            .unwrap();
        assert!(matches!(
            counter.dispatch(EventMessage {
                listener: ListenerId::new(999).unwrap(),
                callback,
                content_type: "application/vnd.lynx.tap".into(),
                payload: Vec::new(),
            }),
            Err(DioxusAdapterError::InvalidListener(999))
        ));
        counter.destroy().unwrap();
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(unsafe_code)]
#[path = "../../native_lifecycle_tests.rs"]
mod native_lifecycle_tests;

#[cfg(test)]
#[path = "../wasm_guest_lifecycle_tests.rs"]
mod wasm_guest_lifecycle_tests;
