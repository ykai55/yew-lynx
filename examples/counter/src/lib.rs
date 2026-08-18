//! A host-independent counter fixture for Yew's generic Clay backend.

use yew::ClayEvent;
use yew::prelude::*;

#[function_component(Counter)]
pub fn counter() -> Html {
    let count = use_state(|| 0);
    let increment = {
        let count = count.clone();
        Callback::from(move |_: ClayEvent| count.set(*count + 1))
    };

    html! {
        <view>
            <text id="counter-value">{format!("Count: {}", *count)}</text>
            <view id="counter-increment" ontap={increment}>
                <text>{"Increment"}</text>
            </view>
        </view>
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::fmt;
    use std::rc::Rc;

    use yew::{ClayBackend, ClayEvent, ClayListener, ClayNode, ClayRenderer};

    use super::Counter;

    const ROOT: ClayNode = ClayNode(1);

    struct ListenerState {
        node: ClayNode,
        name: String,
        callback: Rc<dyn Fn(ClayEvent)>,
    }

    struct RecordingBackend {
        next_node: Cell<u64>,
        next_listener: Cell<u64>,
        listeners: RefCell<HashMap<ClayListener, ListenerState>>,
        pending_text: RefCell<Vec<String>>,
        commits: RefCell<Vec<Vec<String>>>,
    }

    impl fmt::Debug for RecordingBackend {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter
                .debug_struct("RecordingBackend")
                .finish_non_exhaustive()
        }
    }

    impl RecordingBackend {
        fn new() -> Rc<Self> {
            Rc::new(Self {
                next_node: Cell::new(ROOT.0 + 1),
                next_listener: Cell::new(1),
                listeners: RefCell::new(HashMap::new()),
                pending_text: RefCell::new(Vec::new()),
                commits: RefCell::new(Vec::new()),
            })
        }

        fn allocate_node(&self) -> ClayNode {
            let node = ClayNode(self.next_node.get());
            self.next_node.set(node.0 + 1);
            node
        }

        fn tap(&self) {
            let callback = self
                .listeners
                .borrow()
                .values()
                .find(|listener| listener.name == "tap")
                .map(|listener| Rc::clone(&listener.callback))
                .expect("tap listener not found");
            callback(ClayEvent::new("tap"));
        }
    }

    impl ClayBackend for RecordingBackend {
        fn create_element(&self, _tag: &str) -> ClayNode {
            self.allocate_node()
        }

        fn create_text(&self, text: &str) -> ClayNode {
            self.pending_text.borrow_mut().push(text.into());
            self.allocate_node()
        }

        fn insert_before(&self, _parent: ClayNode, _child: ClayNode, _reference: Option<ClayNode>) {
        }

        fn remove(&self, _parent: ClayNode, _child: ClayNode) {}

        fn destroy_node(&self, _node: ClayNode) {}

        fn set_attribute(&self, _node: ClayNode, _name: &str, _value: Option<&str>) {}

        fn add_event_listener(
            &self,
            node: ClayNode,
            name: &str,
            callback: Box<dyn Fn(ClayEvent)>,
        ) -> ClayListener {
            let listener = ClayListener(self.next_listener.get());
            self.next_listener.set(listener.0 + 1);
            self.listeners.borrow_mut().insert(
                listener,
                ListenerState {
                    node,
                    name: name.into(),
                    callback: Rc::from(callback),
                },
            );
            listener
        }

        fn remove_event_listener(&self, node: ClayNode, listener: ClayListener) {
            let removed = self.listeners.borrow_mut().remove(&listener).unwrap();
            assert_eq!(removed.node, node);
        }

        fn flush(&self, _root: ClayNode) {
            self.commits
                .borrow_mut()
                .push(std::mem::take(&mut *self.pending_text.borrow_mut()));
        }
    }

    #[test]
    fn renders_and_updates_counter_through_generic_backend() {
        let backend = RecordingBackend::new();
        let app = ClayRenderer::<Counter>::new(backend.clone(), ROOT).render();

        assert_eq!(backend.commits.borrow()[0], ["Count: 0", "Increment"]);

        backend.tap();

        assert_eq!(backend.commits.borrow()[1], ["Count: 1", "Increment"]);

        app.destroy();
        assert!(backend.listeners.borrow().is_empty());
        assert!(backend.commits.borrow()[2].is_empty());
    }
}
