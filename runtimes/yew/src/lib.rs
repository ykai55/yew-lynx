#![deny(unsafe_op_in_unsafe_fn)]

use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::rc::Rc;

use lynx_element_bridge_core::{BridgeError, CommandBatch, EventMessage, NodeId, Status};
#[cfg(not(target_arch = "wasm32"))]
use lynx_element_bridge_ffi::native_host::{NativeHostHandle, NativeRendererGetApiFn};
#[cfg(not(target_arch = "wasm32"))]
use lynx_element_bridge_ffi::{
    BackendError, BridgeBackend, LynxElementBridgeNativeDestroyResult,
    LynxElementBridgeNativeMountResult, LynxElementBridgeSession,
};
use lynx_element_bridge_wasm_guest::{GuestApplication, MountRequest};
use lynx_element_bridge_yew::{YewAdapter, YewAdapterError};
use yew::{BaseComponent, NativeAppHandle, NativeNode, NativeRenderer};

pub mod prelude {
    pub use yew::NativeEvent;
    pub use yew::prelude::*;
}

pub struct Runtime<C>
where
    C: BaseComponent<Properties = ()>,
{
    adapter: Rc<YewAdapter>,
    app: Option<NativeAppHandle<C>>,
    component: PhantomData<C>,
}

impl<C> Runtime<C>
where
    C: BaseComponent<Properties = ()>,
{
    pub fn mount(root: NodeId) -> Result<(Self, CommandBatch), YewAdapterError> {
        let adapter = YewAdapter::new(root)?;
        let rendered = catch_unwind(AssertUnwindSafe({
            let adapter = Rc::clone(&adapter);
            move || NativeRenderer::<C>::new(adapter, NativeNode(root.get().into())).render()
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
                component: PhantomData,
            },
            batch,
        ))
    }

    pub fn dispatch(&mut self, event: EventMessage) -> Result<CommandBatch, YewAdapterError> {
        self.adapter.dispatch_event(&event)?;
        self.adapter.take_batch()
    }

    pub fn discard_pending(&mut self) {
        self.adapter.discard_pending();
    }

    pub fn abandon(&mut self) {
        if let Some(mut app) = self.app.take() {
            app.abandon();
        }
    }

    fn destroy(mut self) -> Result<CommandBatch, BridgeError> {
        let mut app = self.app.take().ok_or_else(|| {
            BridgeError::new(
                Status::InternalError,
                "Yew runtime has no application handle",
            )
        })?;
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

impl<C> GuestApplication for Runtime<C>
where
    C: BaseComponent<Properties = ()>,
{
    fn mount(request: MountRequest) -> Result<(Self, CommandBatch), BridgeError> {
        Self::mount(request.root).map_err(guest_error)
    }

    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BridgeError> {
        self.dispatch(event).map_err(guest_error)
    }

    fn destroy(self) -> Result<CommandBatch, BridgeError> {
        self.destroy()
    }
}

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
impl<C> BridgeBackend for Runtime<C>
where
    C: BaseComponent<Properties = ()>,
{
    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BackendError> {
        self.dispatch(event).map_err(adapter_error)
    }

    fn destroy(mut self: Box<Self>, poisoned: bool) -> Result<CommandBatch, BackendError> {
        let mut app = self.app.take().ok_or_else(|| {
            BackendError::fatal(
                Status::InternalError,
                "Yew runtime has no application handle",
            )
        })?;
        let destroyed = catch_unwind(AssertUnwindSafe(|| app.destroy()));
        if poisoned {
            self.adapter.discard_pending();
            return Err(BackendError::recoverable(
                Status::HostError,
                "Yew runtime was destroyed after becoming permanently poisoned",
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
        self.discard_pending();
    }

    fn abandon(&mut self) {
        self.abandon();
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

#[cfg(not(target_arch = "wasm32"))]
/// Mounts a Yew root through the native renderer function table.
///
/// # Safety
///
/// `get_api` and `host` must obey the native renderer C ABI for the mounted session lifetime.
pub unsafe fn native_mount<C>(
    get_api: Option<NativeRendererGetApiFn>,
    host: NativeHostHandle,
) -> LynxElementBridgeNativeMountResult
where
    C: BaseComponent<Properties = ()>,
{
    unsafe {
        lynx_element_bridge_ffi::native_mount(get_api, host, |root| {
            let (runtime, batch) = Runtime::<C>::mount(root).map_err(adapter_error)?;
            Ok((Box::new(runtime), batch))
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn native_destroy_session(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    lynx_element_bridge_ffi::native_destroy_session(session)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn native_abandon_session(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    lynx_element_bridge_ffi::native_abandon_session(session)
}

#[macro_export]
macro_rules! launch {
    ($app:ty) => {
        #[cfg(target_arch = "wasm32")]
        $crate::__private::export_guest!($crate::Runtime<$app>);

        #[unsafe(no_mangle)]
        #[cfg(not(target_arch = "wasm32"))]
        #[allow(unsafe_code)]
        unsafe extern "C" fn lynx_element_bridge_native_mount(
            get_api: Option<$crate::__private::NativeRendererGetApiFn>,
            host: $crate::__private::NativeHostHandle,
        ) -> $crate::__private::LynxElementBridgeNativeMountResult {
            unsafe { $crate::native_mount::<$app>(get_api, host) }
        }

        #[unsafe(no_mangle)]
        #[cfg(not(target_arch = "wasm32"))]
        extern "C" fn lynx_element_bridge_native_destroy_session(
            session: $crate::__private::LynxElementBridgeSession,
        ) -> $crate::__private::LynxElementBridgeNativeDestroyResult {
            $crate::native_destroy_session(session)
        }

        #[unsafe(no_mangle)]
        #[cfg(not(target_arch = "wasm32"))]
        extern "C" fn lynx_element_bridge_native_abandon_session(
            session: $crate::__private::LynxElementBridgeSession,
        ) -> $crate::__private::LynxElementBridgeNativeDestroyResult {
            $crate::native_abandon_session(session)
        }

        #[unsafe(no_mangle)]
        #[cfg(not(target_arch = "wasm32"))]
        extern "C" fn lynx_element_bridge_backend() -> *const ::std::ffi::c_char {
            c"yew".as_ptr()
        }

        #[unsafe(no_mangle)]
        #[cfg(not(target_arch = "wasm32"))]
        extern "C" fn lynx_element_bridge_backend_marker() -> *const ::std::ffi::c_char {
            c"lynx-element-bridge-backend:yew".as_ptr()
        }
    };
}

#[doc(hidden)]
pub mod __private {
    #[cfg(not(target_arch = "wasm32"))]
    pub use lynx_element_bridge_ffi::native_host::{NativeHostHandle, NativeRendererGetApiFn};
    #[cfg(not(target_arch = "wasm32"))]
    pub use lynx_element_bridge_ffi::{
        LynxElementBridgeNativeDestroyResult, LynxElementBridgeNativeMountResult,
        LynxElementBridgeSession,
    };
    pub use lynx_element_bridge_wasm_guest::export_guest;
}

#[cfg(test)]
mod tests {
    use lynx_element_bridge_core::{CallbackId, Command, HostFake, ListenerId};

    use super::*;
    use crate::prelude::*;

    #[function_component(TestApp)]
    fn test_app() -> Html {
        let count = use_state(|| 0);
        let increment = {
            let count = count.clone();
            Callback::from(move |_: NativeEvent| count.set(*count + 1))
        };
        html! {
            <view>
                <text>{format!("Count: {}", *count)}</text>
                <view ontap={increment} />
            </view>
        }
    }

    #[test]
    fn runtime_mount_event_and_destroy_apply_real_yew_updates() {
        let root = NodeId::new(1).unwrap();
        let (mut runtime, mounted) = Runtime::<TestApp>::mount(root).unwrap();
        let (listener, callback) = mounted
            .commands
            .iter()
            .find_map(|command| match command {
                Command::AddEventListener {
                    listener,
                    callback,
                    name,
                    ..
                } if name == "tap" => Some((*listener, *callback)),
                _ => None,
            })
            .expect("Yew runtime should register the tap listener");
        let mut host = HostFake::new(root);
        host.apply(&mounted).unwrap();
        assert_eq!(
            host.snapshot().children[0].children[0].children[0]
                .text
                .as_deref(),
            Some("Count: 0")
        );
        assert_eq!(host.listener_count(), 1);

        let updated = BridgeBackend::dispatch_event(
            &mut runtime,
            EventMessage {
                listener,
                callback,
                content_type: "application/vnd.lynx.tap".into(),
                payload: vec![0, 255],
            },
        )
        .unwrap();
        host.apply(&updated).unwrap();
        assert_eq!(
            host.snapshot().children[0].children[0].children[0]
                .text
                .as_deref(),
            Some("Count: 1")
        );

        host.apply(&BridgeBackend::destroy(Box::new(runtime), false).unwrap())
            .unwrap();
        assert!(host.snapshot().children.is_empty());
        assert_eq!(host.listener_count(), 0);
    }

    #[test]
    fn guest_runtime_maps_stale_events_and_destroys_cleanly() {
        let root = NodeId::new(1).unwrap();
        let (mut runtime, mounted) = <Runtime<TestApp> as GuestApplication>::mount(MountRequest {
            protocol_version: lynx_element_bridge_wasm_guest::PROTOCOL_VERSION,
            root,
        })
        .unwrap();
        let mut host = HostFake::new(root);
        host.apply(&mounted).unwrap();

        let error = GuestApplication::dispatch_event(
            &mut runtime,
            EventMessage {
                listener: ListenerId::new(999).unwrap(),
                callback: CallbackId::new(999).unwrap(),
                content_type: "application/vnd.lynx.tap".into(),
                payload: Vec::new(),
            },
        )
        .unwrap_err();
        assert_eq!(error.status, Status::InvalidListener);

        host.apply(&GuestApplication::destroy(runtime).unwrap())
            .unwrap();
        assert!(host.snapshot().children.is_empty());
    }

    #[test]
    fn native_backend_handles_poison_discard_and_abandon() {
        let root = NodeId::new(1).unwrap();
        let (mut poisoned, _) = Runtime::<TestApp>::mount(root).unwrap();
        BridgeBackend::discard_pending(&mut poisoned);
        let error = BridgeBackend::destroy(Box::new(poisoned), true).unwrap_err();
        assert_eq!(error.status, Status::HostError);

        let (mut abandoned, _) = Runtime::<TestApp>::mount(root).unwrap();
        BridgeBackend::abandon(&mut abandoned);
        BridgeBackend::abandon(&mut abandoned);
    }

    #[test]
    fn error_mappings_preserve_public_status_semantics() {
        let bridge = BridgeError::new(Status::InvalidArgument, "bridge");
        assert_eq!(
            guest_error(YewAdapterError::Bridge(bridge.clone())).status,
            Status::InvalidArgument
        );
        assert_eq!(
            guest_error(YewAdapterError::InvalidListener(1)).status,
            Status::InvalidListener
        );
        assert_eq!(
            guest_error(YewAdapterError::CallbackExhausted).status,
            Status::ResourceExhausted
        );
        assert_eq!(
            guest_error(YewAdapterError::Borrowed("listeners")).status,
            Status::HostError
        );

        assert_eq!(
            adapter_error(YewAdapterError::EventMismatch {
                expected: "tap".into(),
                actual: "click".into(),
            })
            .status,
            Status::InvalidListener
        );
        assert_eq!(
            adapter_error(YewAdapterError::Bridge(bridge)).status,
            Status::InvalidArgument
        );
        assert_eq!(
            adapter_error(YewAdapterError::CallbackExhausted).status,
            Status::ResourceExhausted
        );
        assert_eq!(
            adapter_error(YewAdapterError::InvalidNode(1)).status,
            Status::HostError
        );
    }

    #[test]
    fn native_wrappers_reject_missing_api_and_stale_sessions() {
        // SAFETY: A missing API resolver is rejected before native renderer access.
        let mounted = unsafe { native_mount::<TestApp>(None, 1) };
        assert_eq!(
            mounted.status,
            lynx_element_bridge_ffi::native_host::NATIVE_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(mounted.session, 0);
        assert_eq!(native_destroy_session(0).consumed, 0);
        assert_eq!(native_abandon_session(0).consumed, 0);
    }
}
