use lynx_element_bridge_core::{CommandBatch, EventMessage, NodeId, SessionId, Status};
use lynx_element_bridge_dioxus::DioxusAdapterError;
use lynx_element_bridge_ffi::{
    BackendError, BridgeBackend, LynxElementBridgeBuffer, LynxElementBridgeDestroyResult,
    LynxElementBridgeMountResult, LynxElementBridgeSession,
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

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_mount(root_id: u32) -> LynxElementBridgeMountResult {
    lynx_element_bridge_ffi::mount(root_id, create_backend)
}

/// # Safety
///
/// When `event_len` is nonzero, `event` must point to that many readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lynx_element_bridge_dispatch_event(
    session: LynxElementBridgeSession,
    event: *const u8,
    event_len: usize,
) -> LynxElementBridgeBuffer {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_ffi::dispatch_event(session, event, event_len) }
}

/// # Safety
///
/// When `response_len` is nonzero, `response` must point to that many readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lynx_element_bridge_complete_batch(
    session: LynxElementBridgeSession,
    response: *const u8,
    response_len: usize,
) -> LynxElementBridgeBuffer {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_ffi::complete_batch(session, response, response_len) }
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_destroy_session(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeDestroyResult {
    lynx_element_bridge_ffi::destroy_session(session)
}

/// # Safety
///
/// `buffer` must be empty or an unmodified, not-yet-freed buffer returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lynx_element_bridge_buffer_free(buffer: LynxElementBridgeBuffer) {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_ffi::buffer_free(buffer) }
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_backend() -> *const std::ffi::c_char {
    c"dioxus".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_backend_marker() -> *const std::ffi::c_char {
    c"lynx-element-bridge-backend:dioxus".as_ptr()
}
