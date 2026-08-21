//! Yew counter static library backed by the Lynx native renderer function table.

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::rc::Rc;

use lynx_element_bridge_core::{CallbackId, CommandBatch, EventMessage, NodeId, SessionId, Status};
use lynx_element_bridge_ffi::native_host::{NativeHostHandle, NativeRendererGetApiFn};
use lynx_element_bridge_ffi::{
    BackendError, BridgeBackend, LynxElementBridgeNativeDestroyResult,
    LynxElementBridgeNativeMountResult, LynxElementBridgeSession, NativeTimerRequest,
};
use lynx_element_bridge_yew::{YewAdapter, YewAdapterError};
use yew::prelude::*;
use yew::{NativeAppHandle, NativeEvent, NativeNode, NativeRenderer};

const TIMER_CALLBACK_ID: u32 = 1;

pub struct Counter {
    count: u32,
    timer_fired: bool,
}

pub enum CounterMessage {
    Increment,
    TimerFired,
}

impl Component for Counter {
    type Message = CounterMessage;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        Self {
            count: 0,
            timer_fired: false,
        }
    }

    fn update(&mut self, _: &Context<Self>, message: Self::Message) -> bool {
        match message {
            CounterMessage::Increment => self.count += 1,
            CounterMessage::TimerFired => self.timer_fired = true,
        }
        true
    }

    fn view(&self, context: &Context<Self>) -> Html {
        let increment = context
            .link()
            .callback(|_: NativeEvent| CounterMessage::Increment);
        let timer_status = if self.timer_fired {
            "Timer: fired"
        } else {
            "Timer: pending"
        };

        html! {
            <view style="height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center;">
                <text id="counter-value" style="font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 20px;">{format!("Count: {}", self.count)}</text>
                <text id="timer-status" style="font-size: 28px; font-weight: 500; color: #5f665f; margin-bottom: 32px;">{timer_status}</text>
                <view id="counter-increment" style="height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;" ontap={increment}>
                    <text style="font-size: 28px; font-weight: 600; color: #ffffff;">{"Increment"}</text>
                </view>
            </view>
        }
    }
}

struct YewBackend {
    adapter: Rc<YewAdapter>,
    app: Option<NativeAppHandle<Counter>>,
}

impl BridgeBackend for YewBackend {
    fn initial_native_timers(&self) -> Vec<NativeTimerRequest> {
        vec![NativeTimerRequest {
            delay_millis: 1_500,
            repeating: false,
            callback: CallbackId::new(TIMER_CALLBACK_ID).expect("timer callback ID is nonzero"),
        }]
    }

    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BackendError> {
        self.adapter.dispatch_event(&event).map_err(adapter_error)?;
        self.adapter.take_batch().map_err(adapter_error)
    }

    fn dispatch_timer(&mut self, callback: CallbackId) -> Result<CommandBatch, BackendError> {
        if callback.get() != TIMER_CALLBACK_ID {
            return Err(BackendError::recoverable(
                Status::InvalidArgument,
                "Yew timer callback identity does not match",
            ));
        }
        self.app
            .as_ref()
            .ok_or_else(|| {
                BackendError::fatal(
                    Status::InternalError,
                    "Yew backend has no application handle",
                )
            })?
            .send_message(CounterMessage::TimerFired);
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
    c"yew".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_backend_marker() -> *const std::ffi::c_char {
    c"lynx-element-bridge-backend:yew".as_ptr()
}

#[cfg(test)]
#[allow(unsafe_code)]
#[path = "../../native_lifecycle_tests.rs"]
mod native_lifecycle_tests;
