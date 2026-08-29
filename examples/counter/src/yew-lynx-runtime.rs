//! Yew-Lynx runtime backed by the Lynx native renderer function table.

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::rc::Rc;

#[cfg(any(target_arch = "wasm32", test))]
use lynx_element_bridge_core::BridgeError;
use lynx_element_bridge_core::{CommandBatch, EventMessage, NodeId, Status};
#[cfg(not(target_arch = "wasm32"))]
use lynx_element_bridge_ffi::native_host::{NativeHostHandle, NativeRendererGetApiFn};
#[cfg(not(target_arch = "wasm32"))]
use lynx_element_bridge_ffi::{
    BackendError, BridgeBackend, LynxElementBridgeNativeDestroyResult,
    LynxElementBridgeNativeMountResult, LynxElementBridgeSession,
};
#[cfg(any(target_arch = "wasm32", test))]
use lynx_element_bridge_wasm_guest::{GuestApplication, MountRequest};
use lynx_element_bridge_yew::{YewAdapter, YewAdapterError};
use yew::{NativeAppHandle, NativeNode, NativeRenderer};

use crate::app::Counter;

pub struct YewCounter {
    adapter: Rc<YewAdapter>,
    app: Option<NativeAppHandle<Counter>>,
}

impl YewCounter {
    fn mount(root: NodeId) -> Result<(Self, CommandBatch), YewAdapterError> {
        let adapter = YewAdapter::new(root)?;
        let rendered = catch_unwind(AssertUnwindSafe({
            let adapter = Rc::clone(&adapter);
            move || NativeRenderer::<Counter>::new(adapter, NativeNode(root.get().into())).render()
        }));
        let mut app = match rendered {
            Ok(app) => app,
            Err(payload) => {
                adapter.discard_pending();
                resume_unwind(payload);
            }
        };
        let batch = match adapter.take_batch() {
            Ok(batch) => batch,
            Err(error) => {
                let _ = catch_unwind(AssertUnwindSafe(|| app.destroy()));
                return Err(error);
            }
        };
        Ok((
            Self {
                adapter,
                app: Some(app),
            },
            batch,
        ))
    }

    fn dispatch(&mut self, event: EventMessage) -> Result<CommandBatch, YewAdapterError> {
        self.adapter.dispatch_event(&event)?;
        self.adapter.take_batch()
    }

    #[cfg(any(target_arch = "wasm32", test))]
    fn destroy(mut self) -> Result<CommandBatch, BridgeError> {
        let mut app = self
            .app
            .take()
            .expect("a mounted Yew counter must retain its application handle");
        let destroyed = catch_unwind(AssertUnwindSafe(|| app.destroy()));
        match destroyed {
            Ok(Ok(())) => self.adapter.destroy().map_err(guest_error),
            Ok(Err(error)) => {
                app.abandon();
                self.adapter.discard_pending();
                Err(BridgeError::new(Status::InternalError, error.to_string()))
            }
            Err(payload) => {
                self.adapter.discard_pending();
                resume_unwind(payload);
            }
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
impl GuestApplication for YewCounter {
    fn mount(request: MountRequest) -> Result<(Self, CommandBatch), BridgeError> {
        YewCounter::mount(request.root).map_err(guest_error)
    }

    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BridgeError> {
        self.dispatch(event).map_err(guest_error)
    }

    fn destroy(self) -> Result<CommandBatch, BridgeError> {
        YewCounter::destroy(self)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl BridgeBackend for YewCounter {
    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BackendError> {
        self.dispatch(event).map_err(adapter_error)
    }

    fn destroy(mut self: Box<Self>, poisoned: bool) -> Result<CommandBatch, BackendError> {
        let mut app = self.app.take().ok_or_else(|| {
            BackendError::fatal(
                Status::InternalError,
                "Yew backend has no application handle",
            )
        })?;
        let destroyed = catch_unwind(AssertUnwindSafe(|| app.destroy()));
        if poisoned {
            self.adapter.discard_pending();
            return Err(BackendError::recoverable(
                Status::HostError,
                "Yew backend was destroyed after becoming permanently poisoned",
            ));
        }
        match destroyed {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                app.abandon();
                self.adapter.discard_pending();
                return Err(BackendError::fatal(
                    Status::InternalError,
                    error.to_string(),
                ));
            }
            Err(payload) => {
                self.adapter.discard_pending();
                resume_unwind(payload);
            }
        }
        self.adapter.destroy().map_err(adapter_error)
    }

    fn discard_pending(&mut self) {
        self.adapter.discard_pending();
    }

    fn abandon(&mut self) {
        if let Some(mut app) = self.app.take() {
            app.abandon();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn adapter_error(error: YewAdapterError) -> BackendError {
    match error {
        YewAdapterError::InvalidListener(_) | YewAdapterError::EventMismatch { .. } => {
            BackendError::recoverable(Status::InvalidListener, error.to_string())
        }
        YewAdapterError::Bridge(error) => BackendError::fatal(error.status, error.message),
        YewAdapterError::CallbackExhausted => {
            BackendError::fatal(Status::ResourceExhausted, error.to_string())
        }
        YewAdapterError::InvalidNode(_) | YewAdapterError::Borrowed(_) => {
            BackendError::fatal(Status::HostError, error.to_string())
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn guest_error(error: YewAdapterError) -> BridgeError {
    match error {
        YewAdapterError::Bridge(error) => error,
        YewAdapterError::InvalidListener(_) | YewAdapterError::EventMismatch { .. } => {
            BridgeError::new(Status::InvalidListener, error.to_string())
        }
        YewAdapterError::CallbackExhausted => {
            BridgeError::new(Status::ResourceExhausted, error.to_string())
        }
        YewAdapterError::InvalidNode(_) | YewAdapterError::Borrowed(_) => {
            BridgeError::new(Status::HostError, error.to_string())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn create_backend(root: NodeId) -> Result<(Box<dyn BridgeBackend>, CommandBatch), BackendError> {
    let (counter, batch) = YewCounter::mount(root).map_err(adapter_error)?;
    Ok((Box::new(counter), batch))
}

/// Mounts the counter directly through the native renderer function table.
///
/// # Safety
///
/// `get_api` and `host` must obey the native renderer C ABI for the mounted session lifetime.
#[unsafe(no_mangle)]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn lynx_element_bridge_native_mount(
    get_api: Option<NativeRendererGetApiFn>,
    host: NativeHostHandle,
) -> LynxElementBridgeNativeMountResult {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_ffi::native_mount(get_api, host, create_backend) }
}

#[unsafe(no_mangle)]
#[cfg(not(target_arch = "wasm32"))]
pub extern "C" fn lynx_element_bridge_native_destroy_session(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    lynx_element_bridge_ffi::native_destroy_session(session)
}

#[unsafe(no_mangle)]
#[cfg(not(target_arch = "wasm32"))]
pub extern "C" fn lynx_element_bridge_native_abandon_session(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    lynx_element_bridge_ffi::native_abandon_session(session)
}

#[unsafe(no_mangle)]
#[cfg(not(target_arch = "wasm32"))]
pub extern "C" fn lynx_element_bridge_backend() -> *const std::ffi::c_char {
    c"yew".as_ptr()
}

#[unsafe(no_mangle)]
#[cfg(not(target_arch = "wasm32"))]
pub extern "C" fn lynx_element_bridge_backend_marker() -> *const std::ffi::c_char {
    c"lynx-element-bridge-backend:yew".as_ptr()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(unsafe_code)]
#[path = "../../native_lifecycle_tests.rs"]
mod native_lifecycle_tests;

#[cfg(test)]
#[path = "../wasm_guest_lifecycle_tests.rs"]
mod wasm_guest_lifecycle_tests;
