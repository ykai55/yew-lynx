#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fmt;

use dioxus_core::{
    AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, WriteMutations,
};
use lynx_element_bridge_core::{
    BridgeError, CallbackId, CommandBatch, EventMessage, ListenerId, NodeId, Session, Status,
};

pub mod dioxus_elements;

pub mod prelude {
    pub use crate::dioxus_elements;
    pub use dioxus_core_macro::rsx;
    pub use dioxus_signals;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DioxusAdapterError {
    Bridge(BridgeError),
    InvalidElement(usize),
    InvalidListener(u32),
    EventMismatch {
        listener: ListenerId,
        expected: CallbackId,
        actual: CallbackId,
    },
    InvalidStack(usize),
    InvalidTemplatePath(Vec<u8>),
    UnsupportedAttribute,
    CallbackExhausted,
}

impl fmt::Display for DioxusAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bridge(error) => error.fmt(formatter),
            Self::InvalidElement(element) => write!(formatter, "invalid Dioxus element {element}"),
            Self::InvalidListener(listener) => {
                write!(formatter, "invalid Dioxus listener {listener}")
            }
            Self::EventMismatch {
                listener,
                expected,
                actual,
            } => write!(
                formatter,
                "listener {} expects callback {}, not {}",
                listener.get(),
                expected.get(),
                actual.get()
            ),
            Self::InvalidStack(count) => {
                write!(formatter, "Dioxus stack has fewer than {count} nodes")
            }
            Self::InvalidTemplatePath(path) => write!(formatter, "invalid template path {path:?}"),
            Self::UnsupportedAttribute => {
                formatter.write_str("Dioxus Any and Listener attribute values are not serializable")
            }
            Self::CallbackExhausted => formatter.write_str("callback ID space is exhausted"),
        }
    }
}

impl std::error::Error for DioxusAdapterError {}

impl From<BridgeError> for DioxusAdapterError {
    fn from(error: BridgeError) -> Self {
        Self::Bridge(error)
    }
}

#[derive(Debug)]
struct StackEntry {
    root: NodeId,
    paths: HashMap<Vec<u8>, NodeId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Element,
    Text,
    Placeholder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ListenerState {
    element: ElementId,
    name: &'static str,
    callback: CallbackId,
}

pub struct DioxusAdapter {
    session: Session,
    root: NodeId,
    nodes: HashMap<ElementId, NodeId>,
    kinds: HashMap<NodeId, Kind>,
    parents: HashMap<NodeId, NodeId>,
    children: HashMap<NodeId, Vec<NodeId>>,
    stack: Vec<StackEntry>,
    listeners: HashMap<(ElementId, &'static str), ListenerId>,
    live_listeners: HashMap<ListenerId, ListenerState>,
    next_callback: u32,
    error: Option<DioxusAdapterError>,
}

impl fmt::Debug for DioxusAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DioxusAdapter")
            .field("session", &self.session)
            .field("nodes", &self.nodes)
            .field("stack", &self.stack)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl DioxusAdapter {
    pub fn new(root: NodeId) -> Result<Self, DioxusAdapterError> {
        let session = Session::create(root)?;
        Ok(Self {
            session,
            root,
            nodes: HashMap::from([(ElementId(0), root)]),
            kinds: HashMap::from([(root, Kind::Element)]),
            parents: HashMap::new(),
            children: HashMap::from([(root, Vec::new())]),
            stack: Vec::new(),
            listeners: HashMap::new(),
            live_listeners: HashMap::new(),
            next_callback: 1,
            error: None,
        })
    }

    pub fn take_batch(&mut self) -> Result<CommandBatch, DioxusAdapterError> {
        self.check_error()?;
        self.session.take_batch().map_err(Into::into)
    }

    pub fn discard_pending(&mut self) {
        let _ = self.session.discard_pending();
    }

    pub fn event(
        &self,
        element: ElementId,
        name: &'static str,
        content_type: &str,
        payload: Vec<u8>,
    ) -> Result<EventMessage, DioxusAdapterError> {
        self.check_error()?;
        let listener = self
            .listeners
            .get(&(element, name))
            .copied()
            .ok_or(DioxusAdapterError::InvalidElement(element.0))?;
        self.session
            .event(listener, content_type, payload)
            .map_err(Into::into)
    }

    pub fn event_for_listener(
        &self,
        listener: ListenerId,
        name: &'static str,
        content_type: &str,
        payload: Vec<u8>,
    ) -> Result<EventMessage, DioxusAdapterError> {
        self.check_error()?;
        let state = self
            .live_listeners
            .get(&listener)
            .ok_or(DioxusAdapterError::InvalidListener(listener.get()))?;
        if state.name != name {
            return Err(DioxusAdapterError::Bridge(BridgeError::new(
                Status::InvalidListener,
                format!("listener expects `{}`, not `{name}`", state.name),
            )));
        }
        self.event(state.element, state.name, content_type, payload)
    }

    pub fn resolve_event(
        &self,
        event: &EventMessage,
    ) -> Result<(ElementId, &'static str), DioxusAdapterError> {
        self.check_error()?;
        let state = self
            .live_listeners
            .get(&event.listener)
            .ok_or(DioxusAdapterError::InvalidListener(event.listener.get()))?;
        if state.callback != event.callback {
            return Err(DioxusAdapterError::EventMismatch {
                listener: event.listener,
                expected: state.callback,
                actual: event.callback,
            });
        }
        Ok((state.element, state.name))
    }

    pub fn destroy(&mut self) -> Result<CommandBatch, DioxusAdapterError> {
        self.check_error()?;
        self.nodes.retain(|element, _| *element == ElementId(0));
        self.listeners.clear();
        self.live_listeners.clear();
        self.parents.clear();
        self.children.retain(|node, _| *node == self.root);
        self.kinds.retain(|node, _| *node == self.root);
        self.stack.clear();
        self.session.destroy().map_err(Into::into)
    }

    fn lower_template_node(
        &mut self,
        node: &TemplateNode,
        path: &mut Vec<u8>,
        paths: &mut HashMap<Vec<u8>, NodeId>,
    ) -> Result<NodeId, DioxusAdapterError> {
        let bridge_node = match node {
            TemplateNode::Element {
                tag,
                namespace,
                attrs,
                children,
            } => {
                if namespace.is_some() {
                    return Err(DioxusAdapterError::Bridge(BridgeError::new(
                        Status::InvalidArgument,
                        "Dioxus adapter accepts authored Lynx tags without namespaces",
                    )));
                }
                let bridge_node = self.session.create_element(tag)?;
                self.kinds.insert(bridge_node, Kind::Element);
                self.children.insert(bridge_node, Vec::new());
                for attribute in *attrs {
                    if let TemplateAttribute::Static {
                        name,
                        value,
                        namespace,
                    } = attribute
                    {
                        if namespace.is_some() {
                            return Err(DioxusAdapterError::Bridge(BridgeError::new(
                                Status::InvalidArgument,
                                "namespaced template attributes are unsupported",
                            )));
                        }
                        self.session.set_attribute(bridge_node, name, Some(value))?;
                    }
                }
                for (index, child) in children.iter().enumerate() {
                    path.push(u8::try_from(index).map_err(|_| {
                        DioxusAdapterError::Bridge(BridgeError::new(
                            Status::ResourceExhausted,
                            "template has more than 255 children",
                        ))
                    })?);
                    let child = self.lower_template_node(child, path, paths)?;
                    path.pop();
                    self.attach(bridge_node, child, None)?;
                }
                bridge_node
            }
            TemplateNode::Text { text } => {
                let bridge_node = self.session.create_text(text)?;
                self.kinds.insert(bridge_node, Kind::Text);
                self.children.insert(bridge_node, Vec::new());
                bridge_node
            }
            TemplateNode::Dynamic { .. } => self.create_placeholder_node()?,
        };
        paths.insert(path.clone(), bridge_node);
        Ok(bridge_node)
    }

    fn create_placeholder_node(&mut self) -> Result<NodeId, DioxusAdapterError> {
        let node = self.session.create_element("view")?;
        self.session
            .set_attribute(node, "style", Some("display: none"))?;
        self.kinds.insert(node, Kind::Placeholder);
        self.children.insert(node, Vec::new());
        Ok(node)
    }

    fn attach(
        &mut self,
        parent: NodeId,
        child: NodeId,
        reference: Option<NodeId>,
    ) -> Result<(), DioxusAdapterError> {
        self.session.insert_before(parent, child, reference)?;
        let children = self.children.entry(parent).or_default();
        let index = reference
            .and_then(|reference| children.iter().position(|node| *node == reference))
            .unwrap_or(children.len());
        children.insert(index, child);
        self.parents.insert(child, parent);
        Ok(())
    }

    fn detach(&mut self, parent: NodeId, child: NodeId) -> Result<(), DioxusAdapterError> {
        self.session.remove(parent, child)?;
        self.children
            .get_mut(&parent)
            .ok_or(DioxusAdapterError::InvalidElement(parent.get() as usize))?
            .retain(|node| *node != child);
        self.parents.remove(&child);
        Ok(())
    }

    fn pop_nodes(&mut self, count: usize) -> Result<Vec<NodeId>, DioxusAdapterError> {
        if self.stack.len() < count {
            return Err(DioxusAdapterError::InvalidStack(count));
        }
        Ok(self
            .stack
            .split_off(self.stack.len() - count)
            .into_iter()
            .map(|entry| entry.root)
            .collect())
    }

    fn node(&self, element: ElementId) -> Result<NodeId, DioxusAdapterError> {
        self.nodes
            .get(&element)
            .copied()
            .ok_or(DioxusAdapterError::InvalidElement(element.0))
    }

    fn allocate_callback(&mut self) -> Result<CallbackId, DioxusAdapterError> {
        let callback = CallbackId::new(self.next_callback)?;
        self.next_callback = self
            .next_callback
            .checked_add(1)
            .ok_or(DioxusAdapterError::CallbackExhausted)?;
        Ok(callback)
    }

    fn destroy_bridge_node(&mut self, node: NodeId) -> Result<(), DioxusAdapterError> {
        let listeners = self
            .live_listeners
            .iter()
            .filter_map(|(listener, state)| {
                (self.nodes.get(&state.element) == Some(&node)).then_some(*listener)
            })
            .collect::<Vec<_>>();
        for listener in listeners {
            self.session.remove_event_listener(node, listener)?;
            if let Some(state) = self.live_listeners.remove(&listener) {
                self.listeners.remove(&(state.element, state.name));
            }
        }
        let children = self.children.get(&node).cloned().unwrap_or_default();
        for child in children {
            self.detach(node, child)?;
            self.destroy_bridge_node(child)?;
        }
        self.children.remove(&node);
        self.kinds.remove(&node);
        self.nodes.retain(|_, mapped| *mapped != node);
        self.session.destroy_node(node)?;
        Ok(())
    }

    fn remove_element(&mut self, element: ElementId) -> Result<(), DioxusAdapterError> {
        let node = self.node(element)?;
        if node == self.root {
            return Err(DioxusAdapterError::Bridge(BridgeError::new(
                Status::InvalidOwnership,
                "Dioxus cannot remove the bridge root",
            )));
        }
        if let Some(parent) = self.parents.get(&node).copied() {
            self.detach(parent, node)?;
        }
        self.destroy_bridge_node(node)
    }

    fn replace_element(
        &mut self,
        element: ElementId,
        replacements: Vec<NodeId>,
    ) -> Result<(), DioxusAdapterError> {
        let old = self.node(element)?;
        self.replace_bridge_node(old, replacements)
    }

    fn replace_bridge_node(
        &mut self,
        old: NodeId,
        replacements: Vec<NodeId>,
    ) -> Result<(), DioxusAdapterError> {
        let parent = self
            .parents
            .get(&old)
            .copied()
            .ok_or(DioxusAdapterError::InvalidElement(old.get() as usize))?;
        let siblings = self.children.get(&parent).cloned().unwrap_or_default();
        let old_index = siblings
            .iter()
            .position(|node| *node == old)
            .ok_or(DioxusAdapterError::InvalidElement(old.get() as usize))?;
        let reference = siblings.get(old_index + 1).copied();
        self.detach(parent, old)?;
        for replacement in replacements {
            self.attach(parent, replacement, reference)?;
        }
        self.destroy_bridge_node(old)
    }

    fn set_attribute_value(
        &mut self,
        element: ElementId,
        name: &'static str,
        namespace: Option<&'static str>,
        value: &AttributeValue,
    ) -> Result<(), DioxusAdapterError> {
        if namespace.is_some() {
            return Err(DioxusAdapterError::Bridge(BridgeError::new(
                Status::InvalidArgument,
                "namespaced attributes are unsupported",
            )));
        }
        let node = self.node(element)?;
        let value = match value {
            AttributeValue::Text(value) => Some(value.clone()),
            AttributeValue::Float(value) => Some(value.to_string()),
            AttributeValue::Int(value) => Some(value.to_string()),
            AttributeValue::Bool(value) => Some(value.to_string()),
            AttributeValue::None => None,
            AttributeValue::Listener(_) | AttributeValue::Any(_) => {
                return Err(DioxusAdapterError::UnsupportedAttribute);
            }
        };
        self.session.set_attribute(node, name, value.as_deref())?;
        Ok(())
    }

    fn run(&mut self, operation: impl FnOnce(&mut Self) -> Result<(), DioxusAdapterError>) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = operation(self) {
            let _ = self.session.discard_pending();
            self.error = Some(error);
        }
    }

    fn check_error(&self) -> Result<(), DioxusAdapterError> {
        self.error.clone().map_or(Ok(()), Err)
    }
}

impl WriteMutations for DioxusAdapter {
    fn append_children(&mut self, id: ElementId, m: usize) {
        self.run(|this| {
            let parent = this.node(id)?;
            for child in this.pop_nodes(m)? {
                this.attach(parent, child, None)?;
            }
            Ok(())
        });
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        self.run(|this| {
            let node = this
                .stack
                .last()
                .and_then(|entry| entry.paths.get(path))
                .copied()
                .ok_or_else(|| DioxusAdapterError::InvalidTemplatePath(path.into()))?;
            this.nodes.insert(id, node);
            Ok(())
        });
    }

    fn create_placeholder(&mut self, id: ElementId) {
        self.run(|this| {
            let node = this.create_placeholder_node()?;
            this.nodes.insert(id, node);
            this.stack.push(StackEntry {
                root: node,
                paths: HashMap::from([(Vec::new(), node)]),
            });
            Ok(())
        });
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        self.run(|this| {
            let node = this.session.create_text(value)?;
            this.nodes.insert(id, node);
            this.kinds.insert(node, Kind::Text);
            this.children.insert(node, Vec::new());
            this.stack.push(StackEntry {
                root: node,
                paths: HashMap::from([(Vec::new(), node)]),
            });
            Ok(())
        });
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        self.run(|this| {
            let template_root = template
                .roots
                .get(index)
                .ok_or(DioxusAdapterError::InvalidElement(index))?;
            let mut paths = HashMap::new();
            let root = this.lower_template_node(template_root, &mut Vec::new(), &mut paths)?;
            this.nodes.insert(id, root);
            this.stack.push(StackEntry { root, paths });
            Ok(())
        });
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        self.run(|this| {
            let replacements = this.pop_nodes(m)?;
            this.replace_element(id, replacements)
        });
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        self.run(|this| {
            let replacements = this.pop_nodes(m)?;
            let placeholder = this
                .stack
                .last()
                .and_then(|entry| entry.paths.get(path))
                .copied()
                .ok_or_else(|| DioxusAdapterError::InvalidTemplatePath(path.into()))?;
            this.replace_bridge_node(placeholder, replacements)
        });
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        self.run(|this| {
            let target = this.node(id)?;
            let parent = this
                .parents
                .get(&target)
                .copied()
                .ok_or(DioxusAdapterError::InvalidElement(id.0))?;
            let siblings = this.children.get(&parent).cloned().unwrap_or_default();
            let index = siblings
                .iter()
                .position(|node| *node == target)
                .ok_or(DioxusAdapterError::InvalidElement(id.0))?;
            let reference = siblings.get(index + 1).copied();
            for node in this.pop_nodes(m)? {
                this.attach(parent, node, reference)?;
            }
            Ok(())
        });
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        self.run(|this| {
            let target = this.node(id)?;
            let parent = this
                .parents
                .get(&target)
                .copied()
                .ok_or(DioxusAdapterError::InvalidElement(id.0))?;
            for node in this.pop_nodes(m)? {
                this.attach(parent, node, Some(target))?;
            }
            Ok(())
        });
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        self.run(|this| this.set_attribute_value(id, name, ns, value));
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        self.run(|this| {
            let old = this.node(id)?;
            if this.kinds.get(&old) != Some(&Kind::Text) {
                return Err(DioxusAdapterError::InvalidElement(id.0));
            }
            let parent = this
                .parents
                .get(&old)
                .copied()
                .ok_or(DioxusAdapterError::InvalidElement(id.0))?;
            let siblings = this.children.get(&parent).cloned().unwrap_or_default();
            let index = siblings
                .iter()
                .position(|node| *node == old)
                .ok_or(DioxusAdapterError::InvalidElement(id.0))?;
            let reference = siblings.get(index + 1).copied();
            this.detach(parent, old)?;
            this.destroy_bridge_node(old)?;
            let node = this.session.create_text(value)?;
            this.nodes.insert(id, node);
            this.kinds.insert(node, Kind::Text);
            this.children.insert(node, Vec::new());
            this.attach(parent, node, reference)
        });
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.run(|this| {
            let node = this.node(id)?;
            let callback = this.allocate_callback()?;
            let listener = this.session.add_event_listener(node, name, callback)?;
            this.listeners.insert((id, name), listener);
            this.live_listeners.insert(
                listener,
                ListenerState {
                    element: id,
                    name,
                    callback,
                },
            );
            Ok(())
        });
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.run(|this| {
            let node = this.node(id)?;
            let listener = this
                .listeners
                .remove(&(id, name))
                .ok_or(DioxusAdapterError::InvalidElement(id.0))?;
            this.session.remove_event_listener(node, listener)?;
            this.live_listeners.remove(&listener);
            Ok(())
        });
    }

    fn remove_node(&mut self, id: ElementId) {
        self.run(|this| this.remove_element(id));
    }

    fn push_root(&mut self, id: ElementId) {
        self.run(|this| {
            let root = this.node(id)?;
            this.stack.push(StackEntry {
                root,
                paths: HashMap::from([(Vec::new(), root)]),
            });
            Ok(())
        });
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use dioxus_core::{Element, Event, VirtualDom};
    use lynx_element_bridge_core::{Command, HostFake};

    use super::*;
    use crate::prelude::*;

    static VALUE_ATTRS: &[TemplateAttribute] = &[TemplateAttribute::Static {
        name: "id",
        value: "counter-value",
        namespace: None,
    }];
    static VALUE_CHILDREN: &[TemplateNode] = &[TemplateNode::Text { text: "Count: 0" }];
    static ROOT_CHILDREN: &[TemplateNode] = &[TemplateNode::Element {
        tag: "text",
        namespace: None,
        attrs: VALUE_ATTRS,
        children: VALUE_CHILDREN,
    }];
    static ROOTS: &[TemplateNode] = &[TemplateNode::Element {
        tag: "view",
        namespace: None,
        attrs: &[],
        children: ROOT_CHILDREN,
    }];
    static TEMPLATE: Template = Template {
        roots: ROOTS,
        node_paths: &[],
        attr_paths: &[],
    };

    fn lynx_rsx_fixture(received_payload: Rc<RefCell<Vec<u8>>>) -> Element {
        let count = 3;
        rsx! {
            view {
                id: "fixture-root",
                class: "fixture",
                style: "height: 100%;",
                text {
                    id: "fixture-value",
                    "Count: {count}"
                }
                view {
                    id: "fixture-tap",
                    ontap: move |event| {
                        *received_payload.borrow_mut() = event.data.as_ref().clone();
                    },
                }
            }
        }
    }

    #[test]
    fn rsx_authors_lynx_elements_attributes_dynamic_text_and_tap() {
        let root = NodeId::new(1).unwrap();
        let mut adapter = DioxusAdapter::new(root).unwrap();
        let received_payload = Rc::new(RefCell::new(Vec::new()));
        let mut dom = VirtualDom::new_with_props(lynx_rsx_fixture, Rc::clone(&received_payload));
        dom.rebuild(&mut adapter);

        let mut host = HostFake::new(root);
        let mounted = adapter.take_batch().unwrap();
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
        host.apply(&mounted).unwrap();
        let fixture = &host.snapshot().children[0];
        assert_eq!(fixture.tag, "view");
        assert_eq!(fixture.attributes.get("id"), Some(&"fixture-root".into()));
        assert_eq!(fixture.attributes.get("class"), Some(&"fixture".into()));
        assert_eq!(
            fixture.attributes.get("style"),
            Some(&"height: 100%;".into())
        );
        assert_eq!(fixture.children[0].tag, "text");
        assert_eq!(
            fixture.children[0].children[0].text.as_deref(),
            Some("Count: 3")
        );
        assert_eq!(fixture.children[1].tag, "view");
        assert_eq!(host.listener_count(), 1);

        let event = EventMessage {
            listener,
            callback,
            content_type: "application/vnd.lynx.tap".into(),
            payload: vec![0, 255],
        };
        let (target, name) = adapter.resolve_event(&event).unwrap();
        dom.runtime()
            .handle_event(name, Event::new(Rc::new(event.payload), true), target);
        assert_eq!(*received_payload.borrow(), vec![0, 255]);
    }

    #[test]
    fn write_mutations_lowers_lynx_templates_and_stack_operations() {
        let root = NodeId::new(1).unwrap();
        let mut adapter = DioxusAdapter::new(root).unwrap();
        adapter.load_template(TEMPLATE, 0, ElementId(1));
        adapter.assign_node_id(&[0], ElementId(2));
        adapter.append_children(ElementId(0), 1);
        adapter.set_attribute(
            "class",
            None,
            &AttributeValue::Text("active".into()),
            ElementId(2),
        );
        let mut host = HostFake::new(root);
        host.apply(&adapter.take_batch().unwrap()).unwrap();
        let snapshot = host.snapshot();
        assert_eq!(snapshot.children[0].tag, "view");
        assert_eq!(snapshot.children[0].children[0].tag, "text");
        assert_eq!(
            snapshot.children[0].children[0].children[0].text.as_deref(),
            Some("Count: 0")
        );
        assert_eq!(
            snapshot.children[0].children[0].attributes.get("class"),
            Some(&"active".into())
        );
    }

    #[test]
    fn dioxus_events_keep_payload_opaque_and_destroy_releases_state() {
        let root = NodeId::new(1).unwrap();
        let mut adapter = DioxusAdapter::new(root).unwrap();
        adapter.load_template(TEMPLATE, 0, ElementId(1));
        adapter.append_children(ElementId(0), 1);
        adapter.create_event_listener("tap", ElementId(1));
        let event = adapter
            .event(
                ElementId(1),
                "tap",
                "application/vnd.lynx.tap",
                vec![0, 255],
            )
            .unwrap();
        assert_eq!(
            adapter.resolve_event(&event).unwrap(),
            (ElementId(1), "tap")
        );
        assert_eq!(event.payload, vec![0, 255]);

        let mut host = HostFake::new(root);
        host.apply(&adapter.take_batch().unwrap()).unwrap();
        assert_eq!(host.listener_count(), 1);
        host.apply(&adapter.destroy().unwrap()).unwrap();
        assert_eq!(host.listener_count(), 0);
        assert!(host.snapshot().children.is_empty());
    }

    #[test]
    fn event_resolution_requires_the_exact_live_listener_and_callback() {
        let root = NodeId::new(1).unwrap();
        let mut adapter = DioxusAdapter::new(root).unwrap();
        adapter.load_template(TEMPLATE, 0, ElementId(1));
        adapter.append_children(ElementId(0), 1);
        adapter.create_event_listener("tap", ElementId(1));
        adapter.create_event_listener("longpress", ElementId(1));
        let batch = adapter.take_batch().unwrap();
        let registrations = batch
            .commands
            .iter()
            .filter_map(|command| match command {
                Command::AddEventListener {
                    listener,
                    callback,
                    name,
                    ..
                } => Some((*listener, *callback, name.as_str())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let (tap_listener, tap_callback, _) = registrations
            .iter()
            .find(|(_, _, name)| *name == "tap")
            .copied()
            .unwrap();
        let (longpress_listener, longpress_callback, _) = registrations
            .iter()
            .find(|(_, _, name)| *name == "longpress")
            .copied()
            .unwrap();
        let valid = EventMessage {
            listener: tap_listener,
            callback: tap_callback,
            content_type: "application/vnd.lynx.tap".into(),
            payload: vec![1, 2, 3],
        };
        assert_eq!(
            adapter.resolve_event(&valid).unwrap(),
            (ElementId(1), "tap")
        );

        let wrong_listener = EventMessage {
            listener: ListenerId::new(999).unwrap(),
            ..valid.clone()
        };
        assert!(matches!(
            adapter.resolve_event(&wrong_listener),
            Err(DioxusAdapterError::InvalidListener(_))
        ));
        let swapped_callback = EventMessage {
            callback: longpress_callback,
            ..valid.clone()
        };
        assert!(matches!(
            adapter.resolve_event(&swapped_callback),
            Err(DioxusAdapterError::EventMismatch { .. })
        ));
        let swapped_listener = EventMessage {
            listener: longpress_listener,
            callback: tap_callback,
            ..valid.clone()
        };
        assert!(matches!(
            adapter.resolve_event(&swapped_listener),
            Err(DioxusAdapterError::EventMismatch { .. })
        ));
    }

    #[test]
    fn removed_subtrees_and_destroyed_adapters_reject_stale_events() {
        let root = NodeId::new(1).unwrap();
        let mut adapter = DioxusAdapter::new(root).unwrap();
        adapter.load_template(TEMPLATE, 0, ElementId(1));
        adapter.append_children(ElementId(0), 1);
        adapter.create_event_listener("tap", ElementId(1));
        let event = adapter
            .event(ElementId(1), "tap", "application/vnd.lynx.tap", Vec::new())
            .unwrap();
        adapter.remove_event_listener("tap", ElementId(1));
        assert!(matches!(
            adapter.resolve_event(&event),
            Err(DioxusAdapterError::InvalidListener(_))
        ));

        adapter.create_event_listener("tap", ElementId(1));
        let subtree_event = adapter
            .event(ElementId(1), "tap", "application/vnd.lynx.tap", Vec::new())
            .unwrap();
        adapter.remove_node(ElementId(1));
        assert!(matches!(
            adapter.resolve_event(&subtree_event),
            Err(DioxusAdapterError::InvalidListener(_))
        ));

        let mut destroyed = DioxusAdapter::new(root).unwrap();
        destroyed.load_template(TEMPLATE, 0, ElementId(1));
        destroyed.append_children(ElementId(0), 1);
        destroyed.create_event_listener("tap", ElementId(1));
        let destroyed_event = destroyed
            .event(ElementId(1), "tap", "application/vnd.lynx.tap", Vec::new())
            .unwrap();
        destroyed.destroy().unwrap();
        assert!(matches!(
            destroyed.resolve_event(&destroyed_event),
            Err(DioxusAdapterError::InvalidListener(_))
        ));
    }
}
