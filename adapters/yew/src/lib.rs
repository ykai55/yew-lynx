#![forbid(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use lynx_element_bridge_core::{
    BridgeError, CallbackId, CommandBatch, EventMessage, ListenerId, NodeId, Session, Status,
};
use yew::{NativeEvent, NativeListener, NativeNode, NativeRendererBackend};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum YewAdapterError {
    Bridge(BridgeError),
    InvalidNode(u64),
    InvalidListener(u64),
    EventMismatch { expected: String, actual: String },
    CallbackExhausted,
    Borrowed(&'static str),
}

impl fmt::Display for YewAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bridge(error) => error.fmt(formatter),
            Self::InvalidNode(node) => write!(formatter, "invalid Yew node {node}"),
            Self::InvalidListener(listener) => write!(formatter, "invalid Yew listener {listener}"),
            Self::EventMismatch { expected, actual } => {
                write!(formatter, "listener expects `{expected}`, not `{actual}`")
            }
            Self::CallbackExhausted => formatter.write_str("callback ID space is exhausted"),
            Self::Borrowed(registry) => write!(formatter, "{registry} is already borrowed"),
        }
    }
}

impl std::error::Error for YewAdapterError {}

impl From<BridgeError> for YewAdapterError {
    fn from(error: BridgeError) -> Self {
        Self::Bridge(error)
    }
}

struct ListenerState {
    callback: CallbackId,
    name: String,
    handler: Rc<dyn Fn(NativeEvent)>,
}

pub struct YewAdapter {
    session: RefCell<Session>,
    next_callback: Cell<u32>,
    listeners: RefCell<HashMap<ListenerId, ListenerState>>,
    error: RefCell<Option<YewAdapterError>>,
}

impl fmt::Debug for YewAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YewAdapter")
            .field("session", &self.session)
            .field("poisoned", &self.error.borrow().is_some())
            .finish_non_exhaustive()
    }
}

impl YewAdapter {
    pub fn new(root: NodeId) -> Result<Rc<Self>, YewAdapterError> {
        let session = Session::create(root)?;
        Ok(Rc::new(Self {
            session: RefCell::new(session),
            next_callback: Cell::new(1),
            listeners: RefCell::new(HashMap::new()),
            error: RefCell::new(None),
        }))
    }

    pub fn take_batch(&self) -> Result<CommandBatch, YewAdapterError> {
        self.check_error()?;
        self.session
            .try_borrow_mut()
            .map_err(|_| YewAdapterError::Borrowed("session"))?
            .take_batch()
            .map_err(Into::into)
    }

    pub fn import_style_sheet(&self, fragment: &[u8]) -> Result<(), YewAdapterError> {
        self.with_session(|session| session.import_style_sheet(fragment))
    }

    pub fn dispatch_event(&self, event: &EventMessage) -> Result<(), YewAdapterError> {
        self.check_error()?;
        let (name, handler) = {
            let listeners = self
                .listeners
                .try_borrow()
                .map_err(|_| YewAdapterError::Borrowed("listener registry"))?;
            let listener =
                listeners
                    .get(&event.listener)
                    .ok_or(YewAdapterError::InvalidListener(
                        event.listener.get().into(),
                    ))?;
            if listener.callback != event.callback {
                return Err(YewAdapterError::InvalidListener(
                    event.listener.get().into(),
                ));
            }
            (listener.name.clone(), Rc::clone(&listener.handler))
        };
        handler(NativeEvent::new(name));
        Ok(())
    }

    pub fn event(
        &self,
        listener: NativeListener,
        name: &str,
        content_type: &str,
        payload: Vec<u8>,
    ) -> Result<EventMessage, YewAdapterError> {
        let listener = listener_id(listener)?;
        let listeners = self
            .listeners
            .try_borrow()
            .map_err(|_| YewAdapterError::Borrowed("listener registry"))?;
        let state = listeners
            .get(&listener)
            .ok_or(YewAdapterError::InvalidListener(listener.get().into()))?;
        if state.name != name {
            return Err(YewAdapterError::EventMismatch {
                expected: state.name.clone(),
                actual: name.into(),
            });
        }
        drop(listeners);
        self.session
            .try_borrow()
            .map_err(|_| YewAdapterError::Borrowed("session"))?
            .event(listener, content_type, payload)
            .map_err(Into::into)
    }

    pub fn destroy(&self) -> Result<CommandBatch, YewAdapterError> {
        self.check_error()?;
        self.listeners
            .try_borrow_mut()
            .map_err(|_| YewAdapterError::Borrowed("listener registry"))?
            .clear();
        self.session
            .try_borrow_mut()
            .map_err(|_| YewAdapterError::Borrowed("session"))?
            .destroy()
            .map_err(Into::into)
    }

    pub fn discard_pending(&self) {
        if let Ok(mut session) = self.session.try_borrow_mut() {
            let _ = session.discard_pending();
        }
    }

    fn allocate_callback(&self) -> Result<CallbackId, YewAdapterError> {
        let current = self.next_callback.get();
        let callback = CallbackId::new(current).map_err(YewAdapterError::Bridge)?;
        self.next_callback.set(
            current
                .checked_add(1)
                .ok_or(YewAdapterError::CallbackExhausted)?,
        );
        Ok(callback)
    }

    fn check_error(&self) -> Result<(), YewAdapterError> {
        self.error
            .try_borrow()
            .map_err(|_| YewAdapterError::Borrowed("error state"))?
            .clone()
            .map_or(Ok(()), Err)
    }

    fn record_error(&self, error: YewAdapterError) {
        if let Ok(mut current) = self.error.try_borrow_mut() {
            if current.is_none() {
                *current = Some(error);
            }
        }
        self.discard_pending();
    }

    fn with_session<T>(
        &self,
        operation: impl FnOnce(&mut Session) -> Result<T, BridgeError>,
    ) -> Result<T, YewAdapterError> {
        self.check_error()?;
        let mut session = self
            .session
            .try_borrow_mut()
            .map_err(|_| YewAdapterError::Borrowed("session"))?;
        operation(&mut session).map_err(Into::into)
    }
}

impl NativeRendererBackend for YewAdapter {
    fn create_element(&self, tag: &str) -> NativeNode {
        match self.with_session(|session| session.create_element(tag)) {
            Ok(node) => NativeNode(node.get().into()),
            Err(error) => {
                self.record_error(error);
                NativeNode(0)
            }
        }
    }

    fn create_text(&self, text: &str) -> NativeNode {
        match self.with_session(|session| session.create_text(text)) {
            Ok(node) => NativeNode(node.get().into()),
            Err(error) => {
                self.record_error(error);
                NativeNode(0)
            }
        }
    }

    fn insert_before(&self, parent: NativeNode, child: NativeNode, reference: Option<NativeNode>) {
        let result = (|| {
            let parent = node_id(parent)?;
            let child = node_id(child)?;
            let reference = reference.map(node_id).transpose()?;
            self.with_session(|session| session.insert_before(parent, child, reference))
        })();
        if let Err(error) = result {
            self.record_error(error);
        }
    }

    fn remove(&self, parent: NativeNode, child: NativeNode) {
        let result = (|| {
            let parent = node_id(parent)?;
            let child = node_id(child)?;
            self.with_session(|session| session.remove(parent, child))
        })();
        if let Err(error) = result {
            self.record_error(error);
        }
    }

    fn destroy_node(&self, node: NativeNode) {
        let result =
            node_id(node).and_then(|node| self.with_session(|session| session.destroy_node(node)));
        if let Err(error) = result {
            self.record_error(error);
        }
    }

    fn set_attribute(&self, node: NativeNode, name: &str, value: Option<&str>) {
        let result = node_id(node)
            .and_then(|node| self.with_session(|session| session.set_attribute(node, name, value)));
        if let Err(error) = result {
            self.record_error(error);
        }
    }

    fn add_event_listener(
        &self,
        node: NativeNode,
        name: &str,
        callback: Box<dyn Fn(NativeEvent)>,
    ) -> NativeListener {
        let result = (|| {
            let node = node_id(node)?;
            let callback_id = self.allocate_callback()?;
            let listener =
                self.with_session(|session| session.add_event_listener(node, name, callback_id))?;
            self.listeners
                .try_borrow_mut()
                .map_err(|_| YewAdapterError::Borrowed("listener registry"))?
                .insert(
                    listener,
                    ListenerState {
                        callback: callback_id,
                        name: name.into(),
                        handler: Rc::from(callback),
                    },
                );
            Ok::<_, YewAdapterError>(NativeListener(listener.get().into()))
        })();
        match result {
            Ok(listener) => listener,
            Err(error) => {
                self.record_error(error);
                NativeListener(0)
            }
        }
    }

    fn remove_event_listener(&self, node: NativeNode, listener: NativeListener) {
        let result = (|| {
            let node = node_id(node)?;
            let listener = listener_id(listener)?;
            self.with_session(|session| session.remove_event_listener(node, listener))?;
            self.listeners
                .try_borrow_mut()
                .map_err(|_| YewAdapterError::Borrowed("listener registry"))?
                .remove(&listener)
                .ok_or(YewAdapterError::InvalidListener(listener.get().into()))?;
            Ok::<_, YewAdapterError>(())
        })();
        if let Err(error) = result {
            self.record_error(error);
        }
    }

    fn flush(&self, root: NativeNode) {
        let result = node_id(root).and_then(|root| {
            let session = self
                .session
                .try_borrow()
                .map_err(|_| YewAdapterError::Borrowed("session"))?;
            if session.root() == root {
                Ok(())
            } else {
                Err(YewAdapterError::Bridge(BridgeError::new(
                    Status::InvalidArgument,
                    "flush root does not belong to this session",
                )))
            }
        });
        if let Err(error) = result {
            self.record_error(error);
        }
    }
}

fn node_id(node: NativeNode) -> Result<NodeId, YewAdapterError> {
    u32::try_from(node.0)
        .ok()
        .and_then(|node| NodeId::new(node).ok())
        .ok_or(YewAdapterError::InvalidNode(node.0))
}

fn listener_id(listener: NativeListener) -> Result<ListenerId, YewAdapterError> {
    u32::try_from(listener.0)
        .ok()
        .and_then(|listener| ListenerId::new(listener).ok())
        .ok_or(YewAdapterError::InvalidListener(listener.0))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use lynx_element_bridge_core::{Command, HostFake};

    use super::*;

    #[test]
    fn yew_backend_records_framework_neutral_batches_and_opaque_events() {
        let root = NodeId::new(1).unwrap();
        let adapter = YewAdapter::new(root).unwrap();
        let view = adapter.create_element("view");
        adapter.set_attribute(view, "id", Some("button"));
        adapter.insert_before(NativeNode(1), view, None);
        let called = Rc::new(Cell::new(false));
        let listener = adapter.add_event_listener(view, "tap", {
            let called = Rc::clone(&called);
            Box::new(move |_| called.set(true))
        });
        adapter.flush(NativeNode(1));

        let batch = adapter.take_batch().unwrap();
        assert!(batch.commands.iter().any(|command| matches!(
            command,
            Command::CreateElement { tag, .. } if tag == "view"
        )));
        let mut host = HostFake::new(root);
        host.apply(&batch).unwrap();
        let event = adapter
            .event(listener, "tap", "application/vnd.lynx.tap", vec![0, 255])
            .unwrap();
        assert_eq!(event.payload, vec![0, 255]);
        adapter.dispatch_event(&event).unwrap();
        assert!(called.get());
    }

    #[test]
    fn removed_callbacks_are_stale_and_destroy_cleans_the_tree() {
        let root = NodeId::new(1).unwrap();
        let adapter = YewAdapter::new(root).unwrap();
        let view = adapter.create_element("view");
        adapter.insert_before(NativeNode(1), view, None);
        let listener = adapter.add_event_listener(view, "tap", Box::new(|_| {}));
        let event = adapter
            .event(listener, "tap", "application/octet-stream", Vec::new())
            .unwrap();
        adapter.remove_event_listener(view, listener);
        assert!(matches!(
            adapter.dispatch_event(&event),
            Err(YewAdapterError::InvalidListener(_))
        ));
        adapter.remove(NativeNode(1), view);
        adapter.destroy_node(view);
        let mut host = HostFake::new(root);
        host.apply(&adapter.take_batch().unwrap()).unwrap();
        assert!(host.snapshot().children.is_empty());
    }
}
