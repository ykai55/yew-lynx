#![deny(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use dioxus_core::{Element, Event, VirtualDom, schedule_update};
#[cfg(any(target_arch = "wasm32", test))]
use lynx_element_bridge_core::{BridgeError, Status};
use lynx_element_bridge_core::{CommandBatch, EventMessage, NodeId, SessionId};
use lynx_element_bridge_dioxus::prelude::*;
use lynx_element_bridge_dioxus::{DioxusAdapter, DioxusAdapterError};
#[cfg(any(target_arch = "wasm32", test))]
use lynx_element_bridge_wasm_guest::{GuestApplication, MountRequest};

#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)]
mod ffi;

#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
mod wasm;

#[cfg(not(feature = "replacement-fixture"))]
const INITIAL_COUNT: u32 = 0;
#[cfg(feature = "replacement-fixture")]
const INITIAL_COUNT: u32 = 100;

#[cfg(not(target_arch = "wasm32"))]
pub use ffi::{
    lynx_element_bridge_backend, lynx_element_bridge_backend_marker,
    lynx_element_bridge_native_abandon_session, lynx_element_bridge_native_destroy_session,
    lynx_element_bridge_native_mount,
};

struct CounterModel {
    count: Cell<u32>,
    schedule: RefCell<Option<Arc<dyn Fn() + Send + Sync>>>,
}

fn counter(model: Rc<CounterModel>) -> Element {
    let schedule = schedule_update();
    model.schedule.replace(Some(Arc::clone(&schedule)));
    let listener_model = Rc::clone(&model);
    let count = model.count.get();
    rsx! {
        view {
            style: "height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center;",
            text {
                id: "counter-value",
                style: "font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 32px;",
                "Count: {count}"
            }
            view {
                id: "counter-increment",
                style: "height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;",
                ontap: move |_| {
                    listener_model.count.set(listener_model.count.get() + 1);
                    schedule();
                },
                text {
                    style: "font-size: 28px; font-weight: 600; color: #ffffff;",
                    "Increment"
                }
            }
        }
    }
}

pub struct DioxusCounter {
    dom: VirtualDom,
    adapter: DioxusAdapter,
}

impl DioxusCounter {
    pub fn mount(
        session: SessionId,
        root: NodeId,
    ) -> Result<(Self, CommandBatch), DioxusAdapterError> {
        let mut adapter = DioxusAdapter::new(session, root)?;
        let model = Rc::new(CounterModel {
            count: Cell::new(INITIAL_COUNT),
            schedule: RefCell::new(None),
        });
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
        DioxusCounter::mount(request.session, request.root).map_err(guest_error)
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

#[cfg(test)]
mod tests {
    use lynx_element_bridge_core::{CallbackId, Command, HostFake, ListenerId};

    use super::*;

    #[test]
    fn real_virtual_dom_mounts_updates_and_destroys_the_counter() {
        let session = SessionId::new(1).unwrap();
        let root = NodeId::new(1).unwrap();
        let (mut counter, mounted) = DioxusCounter::mount(session, root).unwrap();
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
        let mut host = HostFake::new(session, root);
        host.apply(&mounted).unwrap();
        {
            let snapshot = host.snapshot();
            let screen = &snapshot.children[0];
            assert_eq!(screen.tag, "view");
            assert_eq!(
                screen.attributes.get("style").map(String::as_str),
                Some(
                    "height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center;"
                )
            );
            assert_eq!(screen.children[0].tag, "text");
            assert_eq!(
                screen.children[0].attributes.get("id").map(String::as_str),
                Some("counter-value")
            );
            assert_eq!(
                screen.children[0]
                    .attributes
                    .get("style")
                    .map(String::as_str),
                Some("font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 32px;")
            );
            assert_eq!(screen.children[1].tag, "view");
            assert_eq!(
                screen.children[1].attributes.get("id").map(String::as_str),
                Some("counter-increment")
            );
            assert_eq!(
                screen.children[1]
                    .attributes
                    .get("style")
                    .map(String::as_str),
                Some(
                    "height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;"
                )
            );
            assert_eq!(screen.children[1].children[0].tag, "text");
            assert_eq!(
                screen.children[1].children[0]
                    .attributes
                    .get("style")
                    .map(String::as_str),
                Some("font-size: 28px; font-weight: 600; color: #ffffff;")
            );
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
                session,
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
            session,
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
                session,
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
        let session = SessionId::new(2).unwrap();
        let root = NodeId::new(1).unwrap();
        let (mut counter, mounted) = DioxusCounter::mount(session, root).unwrap();
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
                session,
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
