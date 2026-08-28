#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::thread::{self, ThreadId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(try_from = "u32", into = "u32"))]
pub struct SessionId(u32);

impl SessionId {
    pub fn new(value: u32) -> Result<Self, BridgeError> {
        nonzero_id(value, "session").map(Self)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for SessionId {
    type Error = BridgeError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<SessionId> for u32 {
    fn from(value: SessionId) -> Self {
        value.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(try_from = "u32", into = "u32"))]
pub struct NodeId(u32);

impl NodeId {
    pub fn new(value: u32) -> Result<Self, BridgeError> {
        nonzero_id(value, "node").map(Self)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for NodeId {
    type Error = BridgeError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<NodeId> for u32 {
    fn from(value: NodeId) -> Self {
        value.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(try_from = "u32", into = "u32"))]
pub struct ListenerId(u32);

impl ListenerId {
    pub fn new(value: u32) -> Result<Self, BridgeError> {
        nonzero_id(value, "listener").map(Self)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for ListenerId {
    type Error = BridgeError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ListenerId> for u32 {
    fn from(value: ListenerId) -> Self {
        value.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(try_from = "u32", into = "u32"))]
pub struct CallbackId(u32);

impl CallbackId {
    pub fn new(value: u32) -> Result<Self, BridgeError> {
        nonzero_id(value, "callback").map(Self)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for CallbackId {
    type Error = BridgeError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CallbackId> for u32 {
    fn from(value: CallbackId) -> Self {
        value.get()
    }
}

fn nonzero_id(value: u32, kind: &'static str) -> Result<u32, BridgeError> {
    if value == 0 {
        Err(BridgeError::new(
            Status::InvalidArgument,
            format!("{kind} ID must not be zero"),
        ))
    } else {
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Status {
    Ok,
    InvalidArgument,
    InvalidSession,
    WrongThread,
    Unsupported,
    InvalidOwnership,
    InvalidListener,
    ResourceExhausted,
    HostError,
    Panic,
    InternalError,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BridgeError {
    pub status: Status,
    pub message: String,
}

impl BridgeError {
    pub fn new(status: Status, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl fmt::Display for BridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for BridgeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Command {
    CreateElement {
        node: NodeId,
        tag: String,
    },
    CreateRawText {
        node: NodeId,
        text: String,
    },
    AppendElement {
        parent: NodeId,
        child: NodeId,
    },
    InsertElementBefore {
        parent: NodeId,
        child: NodeId,
        reference: NodeId,
    },
    RemoveElement {
        parent: NodeId,
        child: NodeId,
    },
    DestroyNode {
        node: NodeId,
    },
    SetAttribute {
        node: NodeId,
        name: String,
        value: Option<String>,
    },
    AddEventListener {
        node: NodeId,
        listener: ListenerId,
        callback: CallbackId,
        name: String,
    },
    RemoveEventListener {
        node: NodeId,
        listener: ListenerId,
        callback: CallbackId,
        name: String,
    },
}

/// An ordered, session-scoped set of in-memory renderer mutations.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CommandBatch {
    pub session: SessionId,
    pub sequence: u32,
    pub commands: Vec<Command>,
    pub final_commit: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct EventMessage {
    pub session: SessionId,
    pub listener: ListenerId,
    pub callback: CallbackId,
    pub content_type: String,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum NodeKind {
    Root,
    Element(String),
    RawText(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NodeState {
    kind: NodeKind,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ListenerState {
    node: NodeId,
    callback: CallbackId,
    name: String,
}

#[derive(Debug)]
pub struct Session {
    id: SessionId,
    root: NodeId,
    owner: ThreadId,
    next_node: u32,
    next_listener: u32,
    sequence: u32,
    nodes: HashMap<NodeId, NodeState>,
    listeners: HashMap<ListenerId, ListenerState>,
    pending: Vec<Command>,
    destroyed: bool,
}

impl Session {
    pub fn create(id: SessionId, root: NodeId) -> Result<Self, BridgeError> {
        let mut next_node = 1;
        if root.get() == next_node {
            next_node += 1;
        }
        Ok(Self {
            id,
            root,
            owner: thread::current().id(),
            next_node,
            next_listener: 1,
            sequence: 1,
            nodes: HashMap::from([(
                root,
                NodeState {
                    kind: NodeKind::Root,
                    parent: None,
                    children: Vec::new(),
                },
            )]),
            listeners: HashMap::new(),
            pending: Vec::new(),
            destroyed: false,
        })
    }

    pub const fn id(&self) -> SessionId {
        self.id
    }

    pub const fn root(&self) -> NodeId {
        self.root
    }

    pub fn create_element(&mut self, tag: &str) -> Result<NodeId, BridgeError> {
        self.ensure_owner()?;
        if tag.is_empty() {
            return Err(BridgeError::new(
                Status::InvalidArgument,
                "element tag must not be empty",
            ));
        }
        let node = self.allocate_node()?;
        self.nodes.insert(
            node,
            NodeState {
                kind: NodeKind::Element(tag.into()),
                parent: None,
                children: Vec::new(),
            },
        );
        self.pending.push(Command::CreateElement {
            node,
            tag: tag.into(),
        });
        Ok(node)
    }

    pub fn create_text(&mut self, text: &str) -> Result<NodeId, BridgeError> {
        self.ensure_owner()?;
        let node = self.allocate_node()?;
        self.nodes.insert(
            node,
            NodeState {
                kind: NodeKind::RawText(text.into()),
                parent: None,
                children: Vec::new(),
            },
        );
        self.pending.push(Command::CreateRawText {
            node,
            text: text.into(),
        });
        Ok(node)
    }

    pub fn insert_before(
        &mut self,
        parent: NodeId,
        child: NodeId,
        reference: Option<NodeId>,
    ) -> Result<(), BridgeError> {
        self.ensure_owner()?;
        let parent_state = self.node(parent)?.clone();
        let child_state = self.node(child)?.clone();
        if matches!(parent_state.kind, NodeKind::RawText(_)) {
            return Err(self.ownership("raw text cannot own children"));
        }
        if child == self.root || child_state.parent.is_some() {
            return Err(self.ownership(format!("node {} is not detached", child.get())));
        }
        if matches!(child_state.kind, NodeKind::RawText(_))
            && !matches!(parent_state.kind, NodeKind::Element(ref tag) if tag == "text")
        {
            return Err(self.ownership("raw text must be attached to a text element"));
        }
        let mut ancestor = Some(parent);
        while let Some(node) = ancestor {
            if node == child {
                return Err(self.ownership("insertion would create a cycle"));
            }
            ancestor = self.node(node)?.parent;
        }
        let index = match reference {
            Some(reference) => parent_state
                .children
                .iter()
                .position(|candidate| *candidate == reference)
                .ok_or_else(|| self.ownership("reference is not a direct child"))?,
            None => parent_state.children.len(),
        };
        self.nodes
            .get_mut(&parent)
            .expect("validated parent")
            .children
            .insert(index, child);
        self.nodes.get_mut(&child).expect("validated child").parent = Some(parent);
        self.pending.push(match reference {
            Some(reference) => Command::InsertElementBefore {
                parent,
                child,
                reference,
            },
            None => Command::AppendElement { parent, child },
        });
        Ok(())
    }

    pub fn remove(&mut self, parent: NodeId, child: NodeId) -> Result<(), BridgeError> {
        self.ensure_owner()?;
        if self.node(child)?.parent != Some(parent) {
            return Err(self.ownership("child is not attached to the specified parent"));
        }
        let parent_state = self.node_mut(parent)?;
        let index = parent_state
            .children
            .iter()
            .position(|candidate| *candidate == child)
            .expect("validated direct child");
        parent_state.children.remove(index);
        self.nodes.get_mut(&child).expect("validated child").parent = None;
        self.pending.push(Command::RemoveElement { parent, child });
        Ok(())
    }

    pub fn destroy_node(&mut self, node: NodeId) -> Result<(), BridgeError> {
        self.ensure_owner()?;
        let state = self.node(node)?;
        if node == self.root || state.parent.is_some() || !state.children.is_empty() {
            return Err(self.ownership("node must be detached and childless before destruction"));
        }
        if self
            .listeners
            .values()
            .any(|listener| listener.node == node)
        {
            return Err(self.ownership("node still owns event listeners"));
        }
        self.nodes.remove(&node);
        self.pending.push(Command::DestroyNode { node });
        Ok(())
    }

    pub fn set_attribute(
        &mut self,
        node: NodeId,
        name: &str,
        value: Option<&str>,
    ) -> Result<(), BridgeError> {
        self.ensure_owner()?;
        if name.is_empty() || !matches!(self.node(node)?.kind, NodeKind::Element(_)) {
            return Err(BridgeError::new(
                Status::InvalidArgument,
                "attributes require an element and a nonempty name",
            ));
        }
        self.pending.push(Command::SetAttribute {
            node,
            name: name.into(),
            value: value.map(Into::into),
        });
        Ok(())
    }

    pub fn add_event_listener(
        &mut self,
        node: NodeId,
        name: &str,
        callback: CallbackId,
    ) -> Result<ListenerId, BridgeError> {
        self.ensure_owner()?;
        if name.is_empty() || !matches!(self.node(node)?.kind, NodeKind::Element(_)) {
            return Err(BridgeError::new(
                Status::InvalidArgument,
                "listeners require an element and a nonempty event name",
            ));
        }
        if self
            .listeners
            .values()
            .any(|listener| listener.node == node && listener.name == name)
        {
            return Err(self.ownership("element already has this event listener"));
        }
        let listener = self.allocate_listener()?;
        self.listeners.insert(
            listener,
            ListenerState {
                node,
                callback,
                name: name.into(),
            },
        );
        self.pending.push(Command::AddEventListener {
            node,
            listener,
            callback,
            name: name.into(),
        });
        Ok(listener)
    }

    pub fn remove_event_listener(
        &mut self,
        node: NodeId,
        listener: ListenerId,
    ) -> Result<(), BridgeError> {
        self.ensure_owner()?;
        let state = self.listeners.get(&listener).cloned().ok_or_else(|| {
            BridgeError::new(
                Status::InvalidListener,
                format!("invalid or stale listener {}", listener.get()),
            )
        })?;
        if state.node != node {
            return Err(self.ownership("listener belongs to a different node"));
        }
        self.listeners.remove(&listener);
        self.pending.push(Command::RemoveEventListener {
            node,
            listener,
            callback: state.callback,
            name: state.name,
        });
        Ok(())
    }

    pub fn event(
        &self,
        listener: ListenerId,
        content_type: impl Into<String>,
        payload: Vec<u8>,
    ) -> Result<EventMessage, BridgeError> {
        self.ensure_owner()?;
        let state = self.listeners.get(&listener).ok_or_else(|| {
            BridgeError::new(
                Status::InvalidListener,
                format!("invalid or stale listener {}", listener.get()),
            )
        })?;
        let content_type = content_type.into();
        if content_type.is_empty() {
            return Err(BridgeError::new(
                Status::InvalidArgument,
                "event content type must not be empty",
            ));
        }
        Ok(EventMessage {
            session: self.id,
            listener,
            callback: state.callback,
            content_type,
            payload,
        })
    }

    pub fn take_batch(&mut self) -> Result<CommandBatch, BridgeError> {
        self.ensure_owner()?;
        self.take_batch_allow_destroyed()
    }

    pub fn discard_pending(&mut self) -> Result<(), BridgeError> {
        self.ensure_owner()?;
        self.pending.clear();
        Ok(())
    }

    pub fn destroy(&mut self) -> Result<CommandBatch, BridgeError> {
        self.ensure_owner()?;
        let children = self.node(self.root)?.children.clone();
        for child in children {
            self.destroy_subtree(self.root, child)?;
        }
        self.destroyed = true;
        self.take_batch_allow_destroyed()
    }

    fn destroy_subtree(&mut self, parent: NodeId, node: NodeId) -> Result<(), BridgeError> {
        let listeners: Vec<_> = self
            .listeners
            .iter()
            .filter_map(|(id, state)| (state.node == node).then_some(*id))
            .collect();
        for listener in listeners {
            self.remove_event_listener(node, listener)?;
        }
        let children = self.node(node)?.children.clone();
        for child in children {
            self.destroy_subtree(node, child)?;
        }
        self.remove(parent, node)?;
        self.destroy_node(node)
    }

    fn take_batch_allow_destroyed(&mut self) -> Result<CommandBatch, BridgeError> {
        if self.owner != thread::current().id() {
            return Err(BridgeError::new(
                Status::WrongThread,
                format!(
                    "session {} was called from a non-owner thread",
                    self.id.get()
                ),
            ));
        }
        let batch = CommandBatch {
            session: self.id,
            sequence: self.sequence,
            commands: std::mem::take(&mut self.pending),
            final_commit: true,
        };
        self.sequence = self.sequence.checked_add(1).ok_or_else(|| {
            BridgeError::new(Status::ResourceExhausted, "batch sequence is exhausted")
        })?;
        Ok(batch)
    }

    fn ensure_owner(&self) -> Result<(), BridgeError> {
        if self.destroyed {
            return Err(BridgeError::new(
                Status::InvalidSession,
                format!("session {} is destroyed", self.id.get()),
            ));
        }
        if self.owner != thread::current().id() {
            return Err(BridgeError::new(
                Status::WrongThread,
                format!(
                    "session {} was called from a non-owner thread",
                    self.id.get()
                ),
            ));
        }
        Ok(())
    }

    fn allocate_node(&mut self) -> Result<NodeId, BridgeError> {
        loop {
            let node = NodeId::new(self.next_node)?;
            self.next_node = self.next_node.checked_add(1).ok_or_else(|| {
                BridgeError::new(Status::ResourceExhausted, "node ID space is exhausted")
            })?;
            if node != self.root {
                return Ok(node);
            }
        }
    }

    fn allocate_listener(&mut self) -> Result<ListenerId, BridgeError> {
        let listener = ListenerId::new(self.next_listener)?;
        self.next_listener = self.next_listener.checked_add(1).ok_or_else(|| {
            BridgeError::new(Status::ResourceExhausted, "listener ID space is exhausted")
        })?;
        Ok(listener)
    }

    fn node(&self, node: NodeId) -> Result<&NodeState, BridgeError> {
        self.nodes.get(&node).ok_or_else(|| {
            BridgeError::new(
                Status::InvalidOwnership,
                format!("invalid or stale node {}", node.get()),
            )
        })
    }

    fn node_mut(&mut self, node: NodeId) -> Result<&mut NodeState, BridgeError> {
        self.nodes.get_mut(&node).ok_or_else(|| {
            BridgeError::new(
                Status::InvalidOwnership,
                format!("invalid or stale node {}", node.get()),
            )
        })
    }

    fn ownership(&self, message: impl Into<String>) -> BridgeError {
        BridgeError::new(Status::InvalidOwnership, message)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeSnapshot {
    pub tag: String,
    pub text: Option<String>,
    pub attributes: HashMap<String, String>,
    pub children: Vec<TreeSnapshot>,
}

#[derive(Debug)]
pub struct HostFake {
    session: SessionId,
    root: NodeId,
    nodes: HashMap<NodeId, NodeState>,
    attributes: HashMap<NodeId, HashMap<String, String>>,
    listeners: HashSet<ListenerId>,
    commits: u32,
}

impl HostFake {
    pub fn new(session: SessionId, root: NodeId) -> Self {
        Self {
            session,
            root,
            nodes: HashMap::from([(
                root,
                NodeState {
                    kind: NodeKind::Root,
                    parent: None,
                    children: Vec::new(),
                },
            )]),
            attributes: HashMap::new(),
            listeners: HashSet::new(),
            commits: 0,
        }
    }

    pub fn apply(&mut self, batch: &CommandBatch) -> Result<(), BridgeError> {
        if batch.session != self.session {
            return Err(BridgeError::new(Status::InvalidSession, "session mismatch"));
        }
        for command in &batch.commands {
            self.apply_command(command)?;
        }
        if batch.final_commit {
            self.commits += 1;
        }
        Ok(())
    }

    pub fn snapshot(&self) -> TreeSnapshot {
        self.snapshot_node(self.root)
    }

    pub const fn commits(&self) -> u32 {
        self.commits
    }

    pub fn listener_count(&self) -> usize {
        self.listeners.len()
    }

    fn apply_command(&mut self, command: &Command) -> Result<(), BridgeError> {
        match command {
            Command::CreateElement { node, tag } => {
                self.insert_node(*node, NodeKind::Element(tag.clone()))?;
            }
            Command::CreateRawText { node, text } => {
                self.insert_node(*node, NodeKind::RawText(text.clone()))?;
            }
            Command::AppendElement { parent, child } => self.attach(*parent, *child, None)?,
            Command::InsertElementBefore {
                parent,
                child,
                reference,
            } => self.attach(*parent, *child, Some(*reference))?,
            Command::RemoveElement { parent, child } => {
                let state = self.require_node(*child)?;
                if state.parent != Some(*parent) {
                    return Err(BridgeError::new(
                        Status::InvalidOwnership,
                        "host child-parent mismatch",
                    ));
                }
                self.nodes
                    .get_mut(parent)
                    .expect("validated parent")
                    .children
                    .retain(|candidate| candidate != child);
                self.nodes.get_mut(child).expect("validated child").parent = None;
            }
            Command::DestroyNode { node } => {
                let state = self.require_node(*node)?;
                if state.parent.is_some() || !state.children.is_empty() || *node == self.root {
                    return Err(BridgeError::new(
                        Status::InvalidOwnership,
                        "host refused to destroy an owned node",
                    ));
                }
                self.nodes.remove(node);
                self.attributes.remove(node);
            }
            Command::SetAttribute { node, name, value } => {
                self.require_node(*node)?;
                let attributes = self.attributes.entry(*node).or_default();
                if let Some(value) = value {
                    attributes.insert(name.clone(), value.clone());
                } else {
                    attributes.remove(name);
                }
            }
            Command::AddEventListener { listener, .. } => {
                if !self.listeners.insert(*listener) {
                    return Err(BridgeError::new(
                        Status::InvalidListener,
                        "host listener already exists",
                    ));
                }
            }
            Command::RemoveEventListener { listener, .. } => {
                if !self.listeners.remove(listener) {
                    return Err(BridgeError::new(
                        Status::InvalidListener,
                        "host listener does not exist",
                    ));
                }
            }
        }
        Ok(())
    }

    fn insert_node(&mut self, node: NodeId, kind: NodeKind) -> Result<(), BridgeError> {
        if self.nodes.contains_key(&node) {
            return Err(BridgeError::new(
                Status::InvalidOwnership,
                "host node already exists",
            ));
        }
        self.nodes.insert(
            node,
            NodeState {
                kind,
                parent: None,
                children: Vec::new(),
            },
        );
        Ok(())
    }

    fn attach(
        &mut self,
        parent: NodeId,
        child: NodeId,
        reference: Option<NodeId>,
    ) -> Result<(), BridgeError> {
        self.require_node(parent)?;
        if self.require_node(child)?.parent.is_some() {
            return Err(BridgeError::new(
                Status::InvalidOwnership,
                "host child is already attached",
            ));
        }
        let index = match reference {
            Some(reference) => self
                .require_node(parent)?
                .children
                .iter()
                .position(|candidate| *candidate == reference)
                .ok_or_else(|| {
                    BridgeError::new(
                        Status::InvalidOwnership,
                        "host reference is not a direct child",
                    )
                })?,
            None => self.require_node(parent)?.children.len(),
        };
        self.nodes
            .get_mut(&parent)
            .expect("validated parent")
            .children
            .insert(index, child);
        self.nodes.get_mut(&child).expect("validated child").parent = Some(parent);
        Ok(())
    }

    fn require_node(&self, node: NodeId) -> Result<&NodeState, BridgeError> {
        self.nodes.get(&node).ok_or_else(|| {
            BridgeError::new(
                Status::InvalidOwnership,
                format!("host does not own node {}", node.get()),
            )
        })
    }

    fn snapshot_node(&self, node: NodeId) -> TreeSnapshot {
        let state = self.nodes.get(&node).expect("snapshot node exists");
        let (tag, text) = match &state.kind {
            NodeKind::Root => ("page".into(), None),
            NodeKind::Element(tag) => (tag.clone(), None),
            NodeKind::RawText(text) => ("raw-text".into(), Some(text.clone())),
        };
        TreeSnapshot {
            tag,
            text,
            attributes: self.attributes.get(&node).cloned().unwrap_or_default(),
            children: state
                .children
                .iter()
                .map(|child| self.snapshot_node(*child))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_mutations_mount_and_update_a_tree() {
        let session_id = SessionId::new(7).unwrap();
        let root = NodeId::new(1).unwrap();
        let mut session = Session::create(session_id, root).unwrap();
        let view = session.create_element("view").unwrap();
        let text = session.create_element("text").unwrap();
        let raw = session.create_text("Count: 0").unwrap();
        session.set_attribute(view, "id", Some("counter")).unwrap();
        session.insert_before(root, view, None).unwrap();
        session.insert_before(view, text, None).unwrap();
        session.insert_before(text, raw, None).unwrap();

        let batch = session.take_batch().unwrap();
        assert!(batch.commands.iter().all(|command| !matches!(
            command,
            Command::DestroyNode { .. } | Command::RemoveElement { .. }
        )));
        let mut host = HostFake::new(session_id, root);
        host.apply(&batch).unwrap();

        assert_eq!(host.commits(), 1);
        assert_eq!(
            host.snapshot().children[0].attributes.get("id"),
            Some(&"counter".into())
        );
        assert_eq!(
            host.snapshot().children[0].children[0].children[0]
                .text
                .as_deref(),
            Some("Count: 0")
        );
    }

    #[test]
    fn event_payload_is_opaque_and_destroy_releases_host_state() {
        let session_id = SessionId::new(3).unwrap();
        let root = NodeId::new(1).unwrap();
        let mut session = Session::create(session_id, root).unwrap();
        let button = session.create_element("view").unwrap();
        session.insert_before(root, button, None).unwrap();
        let callback = CallbackId::new(9).unwrap();
        let listener = session.add_event_listener(button, "tap", callback).unwrap();
        let event = session
            .event(listener, "application/vnd.example.tap", vec![0, 255, 7])
            .unwrap();
        assert_eq!(event.callback, callback);
        assert_eq!(event.payload, vec![0, 255, 7]);

        let mut host = HostFake::new(session_id, root);
        host.apply(&session.take_batch().unwrap()).unwrap();
        assert_eq!(host.listener_count(), 1);
        host.apply(&session.destroy().unwrap()).unwrap();
        assert_eq!(host.listener_count(), 0);
        assert!(host.snapshot().children.is_empty());
    }

    #[test]
    fn sessions_reject_calls_from_non_owner_threads() {
        let session =
            Session::create(SessionId::new(11).unwrap(), NodeId::new(1).unwrap()).unwrap();
        let error = std::thread::spawn(move || {
            session
                .event(
                    ListenerId::new(1).unwrap(),
                    "application/octet-stream",
                    Vec::new(),
                )
                .unwrap_err()
        })
        .join()
        .unwrap();
        assert_eq!(error.status, Status::WrongThread);
    }
}
