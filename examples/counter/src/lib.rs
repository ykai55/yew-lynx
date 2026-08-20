//! Counter static library backed by Lynx Element Bridge protocol v2.

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::rc::Rc;

use lynx_element_bridge_core::{CommandBatch, EventMessage, NodeId, SessionId, Status};
use lynx_element_bridge_ffi::{
    BackendError, BridgeBackend, LynxElementBridgeBuffer, LynxElementBridgeDestroyResult,
    LynxElementBridgeMountResult, LynxElementBridgeSession,
};
use lynx_element_bridge_yew::{YewAdapter, YewAdapterError};
use yew::prelude::*;
use yew::{NativeAppHandle, NativeEvent, NativeNode, NativeRenderer};

pub const YEW_LYNX_COUNTER_STATUS_OK: u32 = 0;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT: u32 = 1;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_SESSION: u32 = 2;
pub const YEW_LYNX_COUNTER_STATUS_WRONG_THREAD: u32 = 3;
pub const YEW_LYNX_COUNTER_STATUS_UNSUPPORTED: u32 = 4;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_OWNERSHIP: u32 = 5;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_LISTENER: u32 = 6;
pub const YEW_LYNX_COUNTER_STATUS_RESOURCE_EXHAUSTED: u32 = 7;
pub const YEW_LYNX_COUNTER_STATUS_HOST_ERROR: u32 = 8;
pub const YEW_LYNX_COUNTER_STATUS_PANIC: u32 = 9;
pub const YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR: u32 = 10;

#[function_component(Counter)]
pub fn counter() -> Html {
    let count = use_state(|| 0);
    let increment = {
        let count = count.clone();
        Callback::from(move |_: NativeEvent| count.set(*count + 1))
    };

    html! {
        <view style="height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center;">
            <text id="counter-value" style="font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 32px;">{format!("Count: {}", *count)}</text>
            <view id="counter-increment" style="height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;" ontap={increment}>
                <text style="font-size: 28px; font-weight: 600; color: #ffffff;">{"Increment"}</text>
            </view>
        </view>
    }
}

pub type YewLynxSession = LynxElementBridgeSession;
pub type YewLynxBuffer = LynxElementBridgeBuffer;
pub type YewLynxMountResult = LynxElementBridgeMountResult;
pub type YewLynxDestroyResult = LynxElementBridgeDestroyResult;

struct YewBackend {
    adapter: Rc<YewAdapter>,
    app: Option<NativeAppHandle<Counter>>,
}

impl BridgeBackend for YewBackend {
    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BackendError> {
        self.adapter.dispatch_event(&event).map_err(adapter_error)?;
        self.adapter.take_batch().map_err(adapter_error)
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

fn create_backend(
    session: SessionId,
    root: NodeId,
) -> Result<(Box<dyn BridgeBackend>, CommandBatch), BackendError> {
    let adapter = YewAdapter::new(session, root).map_err(adapter_error)?;
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
            return Err(adapter_error(error));
        }
    };
    Ok((
        Box::new(YewBackend {
            adapter,
            app: Some(app),
        }),
        batch,
    ))
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_mount(root_id: u32) -> YewLynxMountResult {
    lynx_element_bridge_ffi::mount(root_id, create_backend)
}

/// # Safety
///
/// When `event_len` is nonzero, `event` must point to that many readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lynx_element_bridge_dispatch_event(
    session: YewLynxSession,
    event: *const u8,
    event_len: usize,
) -> YewLynxBuffer {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_ffi::dispatch_event(session, event, event_len) }
}

/// # Safety
///
/// When `response_len` is nonzero, `response` must point to that many readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lynx_element_bridge_complete_batch(
    session: YewLynxSession,
    response: *const u8,
    response_len: usize,
) -> YewLynxBuffer {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_ffi::complete_batch(session, response, response_len) }
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_destroy_session(
    session: YewLynxSession,
) -> YewLynxDestroyResult {
    lynx_element_bridge_ffi::destroy_session(session)
}

/// # Safety
///
/// `buffer` must be empty or an unmodified, not-yet-freed buffer returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lynx_element_bridge_buffer_free(buffer: YewLynxBuffer) {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_ffi::buffer_free(buffer) }
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_backend() -> *const std::ffi::c_char {
    c"yew".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_backend_marker() -> *const std::ffi::c_char {
    c"lynx-element-bridge-backend:yew".as_ptr()
}

// Keep the original source ABI as thin aliases for existing embedders.
#[unsafe(no_mangle)]
pub extern "C" fn yew_lynx_mount(root_id: u32) -> YewLynxMountResult {
    lynx_element_bridge_mount(root_id)
}

/// # Safety
///
/// Each nonempty byte span must remain readable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yew_lynx_dispatch(
    session: YewLynxSession,
    event: *const u8,
    event_len: usize,
) -> YewLynxBuffer {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_dispatch_event(session, event, event_len) }
}

/// # Safety
///
/// Each nonempty byte span must remain readable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yew_lynx_complete(
    session: YewLynxSession,
    response: *const u8,
    response_len: usize,
) -> YewLynxBuffer {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_complete_batch(session, response, response_len) }
}

#[unsafe(no_mangle)]
pub extern "C" fn yew_lynx_destroy(session: YewLynxSession) -> YewLynxDestroyResult {
    lynx_element_bridge_destroy_session(session)
}

/// # Safety
///
/// `buffer` must be empty or an unmodified, not-yet-freed buffer returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yew_lynx_buffer_free(buffer: YewLynxBuffer) {
    // SAFETY: This forwards the caller obligations documented on this function.
    unsafe { lynx_element_bridge_buffer_free(buffer) }
}

#[cfg(test)]
mod tests_v2;
