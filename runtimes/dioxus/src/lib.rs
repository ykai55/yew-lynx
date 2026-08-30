#![deny(unsafe_op_in_unsafe_fn)]

use std::marker::PhantomData;
use std::rc::Rc;

use dioxus_core::{Element, Event, VirtualDom};
use lynx_element_bridge_core::{BridgeError, CommandBatch, EventMessage, NodeId, Status};
use lynx_element_bridge_dioxus::{DioxusAdapter, DioxusAdapterError};
#[cfg(not(target_arch = "wasm32"))]
use lynx_element_bridge_ffi::native_host::{NativeHostHandle, NativeRendererGetApiFn};
#[cfg(not(target_arch = "wasm32"))]
use lynx_element_bridge_ffi::{
    BackendError, BridgeBackend, LynxElementBridgeNativeDestroyResult,
    LynxElementBridgeNativeMountResult, LynxElementBridgeSession,
};
use lynx_element_bridge_wasm_guest::{GuestApplication, MountRequest};

pub mod prelude {
    pub use dioxus_core;
    pub use dioxus_core::{Element, use_hook};
    pub use lynx_element_bridge_dioxus::prelude::*;
}

#[doc(hidden)]
pub trait RootComponent: 'static {
    fn render() -> Element;
}

pub struct Runtime<R>
where
    R: RootComponent,
{
    dom: VirtualDom,
    adapter: DioxusAdapter,
    root: PhantomData<R>,
}

impl<R> Runtime<R>
where
    R: RootComponent,
{
    pub fn mount(root: NodeId) -> Result<(Self, CommandBatch), DioxusAdapterError> {
        let mut adapter = DioxusAdapter::new(root)?;
        let mut dom = VirtualDom::new(R::render);
        dom.rebuild(&mut adapter);
        let batch = adapter.take_batch()?;
        Ok((
            Self {
                dom,
                adapter,
                root: PhantomData,
            },
            batch,
        ))
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

impl<R> GuestApplication for Runtime<R>
where
    R: RootComponent,
{
    fn mount(request: MountRequest) -> Result<(Self, CommandBatch), BridgeError> {
        Self::mount(request.root).map_err(guest_error)
    }

    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BridgeError> {
        self.dispatch(event).map_err(guest_error)
    }

    fn destroy(self) -> Result<CommandBatch, BridgeError> {
        self.destroy().map_err(guest_error)
    }
}

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
struct DioxusBackend<R: RootComponent>(Option<Runtime<R>>);

#[cfg(not(target_arch = "wasm32"))]
impl<R> BridgeBackend for DioxusBackend<R>
where
    R: RootComponent,
{
    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BackendError> {
        self.0
            .as_mut()
            .ok_or_else(|| BackendError::fatal(Status::InternalError, "Dioxus runtime is empty"))?
            .dispatch(event)
            .map_err(adapter_error)
    }

    fn destroy(mut self: Box<Self>, _: bool) -> Result<CommandBatch, BackendError> {
        self.0
            .take()
            .ok_or_else(|| BackendError::fatal(Status::InternalError, "Dioxus runtime is empty"))?
            .destroy()
            .map_err(adapter_error)
    }

    fn discard_pending(&mut self) {
        if let Some(runtime) = self.0.as_mut() {
            runtime.discard_pending();
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
/// Mounts a Dioxus root through the native renderer function table.
///
/// # Safety
///
/// `get_api` and `host` must obey the native renderer C ABI for the mounted session lifetime.
pub unsafe fn native_mount<R>(
    get_api: Option<NativeRendererGetApiFn>,
    host: NativeHostHandle,
) -> LynxElementBridgeNativeMountResult
where
    R: RootComponent,
{
    unsafe {
        lynx_element_bridge_ffi::native_mount(get_api, host, |root| {
            let (runtime, batch) = Runtime::<R>::mount(root).map_err(adapter_error)?;
            Ok((Box::new(DioxusBackend(Some(runtime))), batch))
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
    ($app:path) => {
        struct __LynxDioxusRoot;

        impl $crate::RootComponent for __LynxDioxusRoot {
            fn render() -> $crate::__private::Element {
                $app()
            }
        }

        #[cfg(target_arch = "wasm32")]
        $crate::__private::export_guest!($crate::Runtime<__LynxDioxusRoot>);

        #[unsafe(no_mangle)]
        #[cfg(not(target_arch = "wasm32"))]
        #[allow(unsafe_code)]
        unsafe extern "C" fn lynx_element_bridge_native_mount(
            get_api: Option<$crate::__private::NativeRendererGetApiFn>,
            host: $crate::__private::NativeHostHandle,
        ) -> $crate::__private::LynxElementBridgeNativeMountResult {
            unsafe { $crate::native_mount::<__LynxDioxusRoot>(get_api, host) }
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
            c"dioxus".as_ptr()
        }

        #[unsafe(no_mangle)]
        #[cfg(not(target_arch = "wasm32"))]
        extern "C" fn lynx_element_bridge_backend_marker() -> *const ::std::ffi::c_char {
            c"lynx-element-bridge-backend:dioxus".as_ptr()
        }
    };
}

#[doc(hidden)]
pub mod __private {
    pub use dioxus_core::Element;
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
    use std::cell::Cell;
    use std::rc::Rc;

    use dioxus_core::{schedule_update, use_hook};
    use lynx_element_bridge_core::{Command, HostFake};
    use lynx_element_bridge_dioxus::prelude::*;

    use super::*;

    struct TestRoot;

    impl RootComponent for TestRoot {
        fn render() -> Element {
            let count = use_hook(|| Rc::new(Cell::new(0)));
            let listener_count = Rc::clone(&count);
            let displayed_count = count.get();
            rsx! {
                view {
                    text { "Count: {displayed_count}" }
                    view {
                        ontap: move |_| {
                            listener_count.set(listener_count.get() + 1);
                            schedule_update()();
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn runtime_mount_event_and_destroy_apply_real_dioxus_updates() {
        let root = NodeId::new(1).unwrap();
        let (runtime, mounted) = Runtime::<TestRoot>::mount(root).unwrap();
        let mut backend = DioxusBackend(Some(runtime));
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
            .expect("Dioxus runtime should register the tap listener");
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
            &mut backend,
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

        host.apply(&BridgeBackend::destroy(Box::new(backend), false).unwrap())
            .unwrap();
        assert!(host.snapshot().children.is_empty());
        assert_eq!(host.listener_count(), 0);
    }
}
