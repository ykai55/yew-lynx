#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::thread::{self, ThreadId};

mod generated_capabilities {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../protocol/generated/rust/capabilities_generated.rs"
    ));
}

pub use generated_capabilities::{CAPABILITIES, GeneratedCapability, LYNX_REVISION};

pub const PROTOCOL_VERSION: u16 = 2;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(u32);

impl SessionId {
    pub fn new(value: u32) -> Result<Self, BridgeError> {
        nonzero_id(value, "session").map(Self)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(u32);

impl NodeId {
    pub fn new(value: u32) -> Result<Self, BridgeError> {
        nonzero_id(value, "node").map(Self)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ListenerId(u32);

impl ListenerId {
    pub fn new(value: u32) -> Result<Self, BridgeError> {
        nonzero_id(value, "listener").map(Self)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CallbackId(u32);

impl CallbackId {
    pub fn new(value: u32) -> Result<Self, BridgeError> {
        nonzero_id(value, "callback").map(Self)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ResultSlot(u32);

impl ResultSlot {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
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
pub struct CapabilityRequest {
    pub name: String,
    pub required: bool,
}

impl CapabilityRequest {
    pub fn required(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            required: true,
        }
    }

    pub fn optional(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            required: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NegotiatedCapability {
    pub name: String,
    pub available: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
    GetTag {
        node: NodeId,
    },
    InvokeCapability {
        capability: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandItem {
    pub result_slot: Option<ResultSlot>,
    pub command: Command,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandBatch {
    pub session: SessionId,
    pub sequence: u32,
    pub commands: Vec<CommandItem>,
    pub final_commit: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResultValue {
    Element(NodeId),
    Elements(Vec<NodeId>),
    String(String),
    Strings(Vec<String>),
    Boolean(bool),
    Number(f64),
    Payload {
        content_type: String,
        bytes: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommandResult {
    pub slot: Option<ResultSlot>,
    pub status: Status,
    pub message: Option<String>,
    pub value: Option<ResultValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResponseBatch {
    pub session: Option<SessionId>,
    pub sequence: u32,
    pub status: Status,
    pub message: Option<String>,
    pub results: Vec<CommandResult>,
    pub committed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
    negotiated: HashMap<String, bool>,
    nodes: HashMap<NodeId, NodeState>,
    listeners: HashMap<ListenerId, ListenerState>,
    pending: Vec<CommandItem>,
    destroyed: bool,
}

impl Session {
    pub fn create(
        id: SessionId,
        root: NodeId,
        requests: &[CapabilityRequest],
    ) -> Result<(Self, Vec<NegotiatedCapability>), BridgeError> {
        let mut negotiated = HashMap::new();
        let mut response = Vec::with_capacity(requests.len());
        for request in requests {
            if negotiated.contains_key(&request.name) {
                return Err(BridgeError::new(
                    Status::InvalidArgument,
                    format!("capability `{}` was requested more than once", request.name),
                ));
            }
            let available = CAPABILITIES
                .iter()
                .find(|capability| capability.name == request.name)
                .is_some_and(|capability| capability.available);
            if request.required && !available {
                return Err(BridgeError::new(
                    Status::Unsupported,
                    format!("required capability `{}` is unavailable", request.name),
                ));
            }
            negotiated.insert(request.name.clone(), available);
            response.push(NegotiatedCapability {
                name: request.name.clone(),
                available,
            });
        }

        let mut next_node = 1;
        if root.get() == next_node {
            next_node += 1;
        }
        Ok((
            Self {
                id,
                root,
                owner: thread::current().id(),
                next_node,
                next_listener: 1,
                sequence: 1,
                negotiated,
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
            },
            response,
        ))
    }

    pub const fn id(&self) -> SessionId {
        self.id
    }

    pub const fn root(&self) -> NodeId {
        self.root
    }

    pub fn create_element(&mut self, tag: &str) -> Result<NodeId, BridgeError> {
        self.require_capability("create_element")?;
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
        self.push(Command::CreateElement {
            node,
            tag: tag.into(),
        });
        Ok(node)
    }

    pub fn create_text(&mut self, text: &str) -> Result<NodeId, BridgeError> {
        self.require_capability("create_raw_text")?;
        let node = self.allocate_node()?;
        self.nodes.insert(
            node,
            NodeState {
                kind: NodeKind::RawText(text.into()),
                parent: None,
                children: Vec::new(),
            },
        );
        self.push(Command::CreateRawText {
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
        let capability = if reference.is_some() {
            "insert_element_before"
        } else {
            "append_element"
        };
        self.require_capability(capability)?;
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
        self.push(match reference {
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
        self.require_capability("remove_element")?;
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
        self.push(Command::RemoveElement { parent, child });
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
        self.push(Command::DestroyNode { node });
        Ok(())
    }

    pub fn set_attribute(
        &mut self,
        node: NodeId,
        name: &str,
        value: Option<&str>,
    ) -> Result<(), BridgeError> {
        self.require_capability("set_attribute")?;
        if name.is_empty() || !matches!(self.node(node)?.kind, NodeKind::Element(_)) {
            return Err(BridgeError::new(
                Status::InvalidArgument,
                "attributes require an element and a nonempty name",
            ));
        }
        self.push(Command::SetAttribute {
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
        self.require_capability("add_event_listener")?;
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
        self.push(Command::AddEventListener {
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
        self.require_capability("remove_event_listener")?;
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
        self.push(Command::RemoveEventListener {
            node,
            listener,
            callback: state.callback,
            name: state.name,
        });
        Ok(())
    }

    pub fn query_tag(&mut self, node: NodeId, slot: ResultSlot) -> Result<(), BridgeError> {
        self.require_capability("get_tag")?;
        self.node(node)?;
        self.pending.push(CommandItem {
            result_slot: Some(slot),
            command: Command::GetTag { node },
        });
        Ok(())
    }

    pub fn invoke_optional(
        &mut self,
        capability: &str,
        slot: ResultSlot,
    ) -> Result<(), BridgeError> {
        self.ensure_owner()?;
        match self.negotiated.get(capability) {
            Some(false) => {
                self.pending.push(CommandItem {
                    result_slot: Some(slot),
                    command: Command::InvokeCapability {
                        capability: capability.into(),
                    },
                });
                Ok(())
            }
            Some(true) => Err(BridgeError::new(
                Status::InvalidArgument,
                format!("capability `{capability}` is available and requires typed arguments"),
            )),
            None => Err(BridgeError::new(
                Status::InvalidArgument,
                format!("capability `{capability}` was not negotiated"),
            )),
        }
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

    fn require_capability(&self, capability: &str) -> Result<(), BridgeError> {
        self.ensure_owner()?;
        match self.negotiated.get(capability) {
            Some(true) => Ok(()),
            Some(false) => Err(BridgeError::new(
                Status::Unsupported,
                format!("capability `{capability}` is unavailable"),
            )),
            None => Err(BridgeError::new(
                Status::InvalidArgument,
                format!("capability `{capability}` was not negotiated"),
            )),
        }
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
        self.ensure_owner()?;
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
        self.ensure_owner()?;
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

    fn push(&mut self, command: Command) {
        self.pending.push(CommandItem {
            result_slot: None,
            command,
        });
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

    pub fn apply(&mut self, batch: &CommandBatch) -> ResponseBatch {
        if batch.session != self.session {
            return self.failure(batch.sequence, Status::InvalidSession, "session mismatch");
        }
        let mut results = Vec::with_capacity(batch.commands.len());
        let mut batch_status = Status::Ok;
        for item in &batch.commands {
            let outcome = self.apply_command(&item.command);
            let (status, message, value) = match outcome {
                Ok(value) => (Status::Ok, None, value),
                Err(error) => {
                    if batch_status == Status::Ok {
                        batch_status = error.status;
                    }
                    (error.status, Some(error.message), None)
                }
            };
            results.push(CommandResult {
                slot: item.result_slot,
                status,
                message,
                value,
            });
        }
        if batch.final_commit {
            self.commits += 1;
        }
        ResponseBatch {
            session: Some(self.session),
            sequence: batch.sequence,
            status: batch_status,
            message: None,
            results,
            committed: batch.final_commit,
        }
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

    fn apply_command(&mut self, command: &Command) -> Result<Option<ResultValue>, BridgeError> {
        match command {
            Command::CreateElement { node, tag } => {
                self.insert_node(*node, NodeKind::Element(tag.clone()))?;
            }
            Command::CreateRawText { node, text } => {
                self.insert_node(*node, NodeKind::RawText(text.clone()))?;
            }
            Command::AppendElement { parent, child } => {
                self.attach(*parent, *child, None)?;
            }
            Command::InsertElementBefore {
                parent,
                child,
                reference,
            } => {
                self.attach(*parent, *child, Some(*reference))?;
            }
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
            Command::GetTag { node } => {
                let tag = match &self.require_node(*node)?.kind {
                    NodeKind::Root => "page",
                    NodeKind::Element(tag) => tag,
                    NodeKind::RawText(_) => "raw-text",
                };
                return Ok(Some(ResultValue::String(tag.into())));
            }
            Command::InvokeCapability { capability } => {
                return Err(BridgeError::new(
                    Status::Unsupported,
                    format!("capability `{capability}` is unsupported"),
                ));
            }
        }
        Ok(None)
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

    fn failure(&self, sequence: u32, status: Status, message: &str) -> ResponseBatch {
        ResponseBatch {
            session: Some(self.session),
            sequence,
            status,
            message: Some(message.into()),
            results: Vec::new(),
            committed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(name: &str) -> CapabilityRequest {
        CapabilityRequest::required(name)
    }

    fn all_test_capabilities() -> Vec<CapabilityRequest> {
        [
            "create_element",
            "create_raw_text",
            "append_element",
            "insert_element_before",
            "remove_element",
            "set_attribute",
            "add_event_listener",
            "remove_event_listener",
            "get_tag",
        ]
        .into_iter()
        .map(request)
        .collect()
    }

    #[test]
    fn required_and_optional_capabilities_are_negotiated_before_mount() {
        let required = Session::create(
            SessionId::new(1).unwrap(),
            NodeId::new(1).unwrap(),
            &[CapabilityRequest::required("set_static_style")],
        )
        .unwrap_err();
        assert_eq!(required.status, Status::Unsupported);

        let (_, negotiated) = Session::create(
            SessionId::new(1).unwrap(),
            NodeId::new(1).unwrap(),
            &[
                CapabilityRequest::required("create_element"),
                CapabilityRequest::optional("set_static_style"),
            ],
        )
        .unwrap();
        assert_eq!(
            negotiated,
            vec![
                NegotiatedCapability {
                    name: "create_element".into(),
                    available: true,
                },
                NegotiatedCapability {
                    name: "set_static_style".into(),
                    available: false,
                },
            ]
        );
    }

    #[test]
    fn ordered_batch_produces_tree_query_optional_status_and_one_commit() {
        let session_id = SessionId::new(7).unwrap();
        let root = NodeId::new(1).unwrap();
        let mut capabilities = all_test_capabilities();
        capabilities.push(CapabilityRequest::optional("set_static_style"));
        let (mut session, _) = Session::create(session_id, root, &capabilities).unwrap();

        let view = session.create_element("view").unwrap();
        let text = session.create_element("text").unwrap();
        let raw = session.create_text("Count: 0").unwrap();
        session.set_attribute(view, "id", Some("counter")).unwrap();
        session.insert_before(root, view, None).unwrap();
        session.insert_before(view, text, None).unwrap();
        session.insert_before(text, raw, None).unwrap();
        session.query_tag(text, ResultSlot::new(0)).unwrap();
        session
            .invoke_optional("set_static_style", ResultSlot::new(1))
            .unwrap();

        let batch = session.take_batch().unwrap();
        let mut host = HostFake::new(session_id, root);
        let response = host.apply(&batch);

        assert_eq!(response.status, Status::Unsupported);
        assert!(response.committed);
        assert_eq!(host.commits(), 1);
        assert_eq!(
            response
                .results
                .iter()
                .find(|result| result.slot == Some(ResultSlot::new(0))),
            Some(&CommandResult {
                slot: Some(ResultSlot::new(0)),
                status: Status::Ok,
                message: None,
                value: Some(ResultValue::String("text".into())),
            })
        );
        assert_eq!(
            response
                .results
                .iter()
                .find(|result| result.slot == Some(ResultSlot::new(1)))
                .unwrap()
                .status,
            Status::Unsupported
        );
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
        let (mut session, _) = Session::create(session_id, root, &all_test_capabilities()).unwrap();
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
        host.apply(&session.take_batch().unwrap());
        assert_eq!(host.listener_count(), 1);
        let destroyed = session.destroy().unwrap();
        let response = host.apply(&destroyed);
        assert_eq!(response.status, Status::Ok);
        assert_eq!(host.listener_count(), 0);
        assert!(host.snapshot().children.is_empty());
    }

    #[test]
    fn sessions_reject_calls_from_non_owner_threads() {
        let (session, _) = Session::create(
            SessionId::new(11).unwrap(),
            NodeId::new(1).unwrap(),
            &all_test_capabilities(),
        )
        .unwrap();
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
