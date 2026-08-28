use lynx_element_bridge_core::{CommandBatch, EventMessage, NodeId, SessionId, Status};
use lynx_element_bridge_dioxus::DioxusAdapterError;
use lynx_element_bridge_ffi::native_host::{NativeHostHandle, NativeRendererGetApiFn};
use lynx_element_bridge_ffi::{
    BackendError, BridgeBackend, LynxElementBridgeNativeDestroyResult,
    LynxElementBridgeNativeMountResult, LynxElementBridgeSession,
};

use crate::DioxusCounter;

struct DioxusBackend(Option<DioxusCounter>);

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

fn create_backend(
    session: SessionId,
    root: NodeId,
) -> Result<(Box<dyn BridgeBackend>, CommandBatch), BackendError> {
    let (counter, batch) = DioxusCounter::mount(session, root).map_err(adapter_error)?;
    Ok((Box::new(DioxusBackend(Some(counter))), batch))
}

/// Mounts the counter directly through the native renderer function table.
///
/// # Safety
///
/// `get_api` and `host` must obey the native renderer C ABI for the mounted session lifetime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lynx_element_bridge_native_mount(
    get_api: Option<NativeRendererGetApiFn>,
    host: NativeHostHandle,
) -> LynxElementBridgeNativeMountResult {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_ffi::native_mount(get_api, host, create_backend) }
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_native_destroy_session(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    lynx_element_bridge_ffi::native_destroy_session(session)
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_native_abandon_session(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    lynx_element_bridge_ffi::native_abandon_session(session)
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_backend() -> *const std::ffi::c_char {
    c"dioxus".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_backend_marker() -> *const std::ffi::c_char {
    c"lynx-element-bridge-backend:dioxus".as_ptr()
}
