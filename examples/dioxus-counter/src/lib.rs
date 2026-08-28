#![deny(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use dioxus_core::{Element, Event, VirtualDom, schedule_update};
use lynx_element_bridge_core::{
    BridgeError, CallbackId, CommandBatch, EventMessage, NodeId, SessionId, Status,
};
use lynx_element_bridge_dioxus::prelude::*;
use lynx_element_bridge_dioxus::{DioxusAdapter, DioxusAdapterError};

#[allow(unsafe_code)]
mod ffi;

pub use ffi::{
    lynx_element_bridge_backend, lynx_element_bridge_backend_marker,
    lynx_element_bridge_native_abandon_session, lynx_element_bridge_native_destroy_session,
    lynx_element_bridge_native_mount,
};

pub(crate) const TIMER_CALLBACK_ID: u32 = 1;

struct CounterModel {
    count: Cell<u32>,
    timer_fired: Cell<bool>,
    schedule: RefCell<Option<Arc<dyn Fn() + Send + Sync>>>,
}

fn counter(model: Rc<CounterModel>) -> Element {
    let schedule = schedule_update();
    model.schedule.replace(Some(Arc::clone(&schedule)));
    let listener_model = Rc::clone(&model);
    let count = model.count.get();
    let timer_status = if model.timer_fired.get() {
        "fired"
    } else {
        "pending"
    };

    rsx! {
        view {
            style: "height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center;",
            text {
                id: "counter-value",
                style: "font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 32px;",
                "Count: {count}"
            }
            text {
                id: "timer-status",
                style: "font-size: 28px; font-weight: 500; color: #5f665f; margin-bottom: 32px;",
                "Timer: {timer_status}"
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
    model: Rc<CounterModel>,
}

impl DioxusCounter {
    pub fn mount(
        session: SessionId,
        root: NodeId,
    ) -> Result<(Self, CommandBatch), DioxusAdapterError> {
        let mut adapter = DioxusAdapter::new(session, root)?;
        let model = Rc::new(CounterModel {
            count: Cell::new(0),
            timer_fired: Cell::new(false),
            schedule: RefCell::new(None),
        });
        let mut dom = VirtualDom::new_with_props(counter, Rc::clone(&model));
        dom.rebuild(&mut adapter);
        let batch = adapter.take_batch()?;
        Ok((
            Self {
                dom,
                adapter,
                model,
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

    pub fn dispatch_timer(
        &mut self,
        callback: CallbackId,
    ) -> Result<CommandBatch, DioxusAdapterError> {
        if callback.get() != TIMER_CALLBACK_ID {
            return Err(BridgeError::new(
                Status::InvalidArgument,
                "Dioxus timer callback identity does not match",
            )
            .into());
        }
        self.model.timer_fired.set(true);
        let schedule = self
            .model
            .schedule
            .borrow()
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                BridgeError::new(Status::InternalError, "Dioxus counter has no scheduler")
            })?;
        schedule();
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
            assert_eq!(screen.children[1].tag, "text");
            assert_eq!(
                screen.children[1].attributes.get("id").map(String::as_str),
                Some("timer-status")
            );
            assert_eq!(
                screen.children[1]
                    .attributes
                    .get("style")
                    .map(String::as_str),
                Some("font-size: 28px; font-weight: 500; color: #5f665f; margin-bottom: 32px;")
            );
            assert_eq!(screen.children[2].tag, "view");
            assert_eq!(
                screen.children[2].attributes.get("id").map(String::as_str),
                Some("counter-increment")
            );
            assert_eq!(
                screen.children[2]
                    .attributes
                    .get("style")
                    .map(String::as_str),
                Some(
                    "height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;"
                )
            );
            assert_eq!(screen.children[2].children[0].tag, "text");
            assert_eq!(
                screen.children[2].children[0]
                    .attributes
                    .get("style")
                    .map(String::as_str),
                Some("font-size: 28px; font-weight: 600; color: #ffffff;")
            );
            assert_eq!(
                screen.children[2].children[0].children[0].text.as_deref(),
                Some("Increment")
            );
        }
        assert_eq!(
            host.snapshot().children[0].children[0].children[0]
                .text
                .as_deref(),
            Some("Count: 0")
        );
        assert_eq!(
            host.snapshot().children[0].children[1].children[0]
                .text
                .as_deref(),
            Some("Timer: pending")
        );

        let timer_update = counter.dispatch_timer(CallbackId::new(1).unwrap()).unwrap();
        host.apply(&timer_update).unwrap();
        assert_eq!(
            host.snapshot().children[0].children[0].children[0]
                .text
                .as_deref(),
            Some("Count: 0")
        );
        assert_eq!(
            host.snapshot().children[0].children[1].children[0]
                .text
                .as_deref(),
            Some("Timer: fired")
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
            Some("Count: 1")
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
            Some("Count: 2")
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

#[cfg(test)]
#[allow(unsafe_code)]
#[path = "../../native_lifecycle_tests.rs"]
mod native_lifecycle_tests;
