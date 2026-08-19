#![forbid(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use yew::{NativeEvent, NativeListener, NativeNode, NativeRendererBackend};

pub const PROTOCOL_VERSION: u32 = 1;
pub const JS_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SuccessResponse {
    pub version: u32,
    pub ok: bool,
    pub operations: Vec<FiberMutation>,
}

impl SuccessResponse {
    pub fn new(operations: Vec<FiberMutation>) -> Result<Self, ProtocolError> {
        let response = Self {
            version: PROTOCOL_VERSION,
            ok: true,
            operations,
        };
        response.validate()?;
        Ok(response)
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_version(self.version)?;
        if !self.ok {
            return Err(ProtocolError::InvalidSuccessFlag);
        }
        validate_operations(&self.operations, false)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FailureResponse {
    pub version: u32,
    pub ok: bool,
    pub status: u32,
    pub error: String,
    pub operations: Vec<FiberMutation>,
}

impl FailureResponse {
    pub fn new(
        status: u32,
        error: impl Into<String>,
        operations: Vec<FiberMutation>,
    ) -> Result<Self, ProtocolError> {
        let response = Self {
            version: PROTOCOL_VERSION,
            ok: false,
            status,
            error: error.into(),
            operations,
        };
        response.validate()?;
        Ok(response)
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_version(self.version)?;
        if self.ok {
            return Err(ProtocolError::InvalidFailureFlag);
        }
        if self.status == 0 {
            return Err(ProtocolError::ZeroFailureStatus);
        }
        validate_operations(&self.operations, true)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ProtocolResponse {
    Success(SuccessResponse),
    Failure(FailureResponse),
}

impl ProtocolResponse {
    pub fn success(operations: Vec<FiberMutation>) -> Result<Self, ProtocolError> {
        SuccessResponse::new(operations).map(Self::Success)
    }

    pub fn failure(
        status: u32,
        error: impl Into<String>,
        operations: Vec<FiberMutation>,
    ) -> Result<Self, ProtocolError> {
        FailureResponse::new(status, error, operations).map(Self::Failure)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| ProtocolError::Json(error.to_string()))
    }

    pub fn from_json(json: &[u8]) -> Result<Self, ProtocolError> {
        let response: Self =
            serde_json::from_slice(json).map_err(|error| ProtocolError::Json(error.to_string()))?;
        response.validate()?;
        Ok(response)
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::Success(response) => response.validate(),
            Self::Failure(response) => response.validate(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum FiberMutation {
    CreateElement {
        node: u64,
        tag: String,
    },
    CreateText {
        node: u64,
        text: String,
    },
    InsertBefore {
        parent: u64,
        child: u64,
        reference: Option<u64>,
    },
    Remove {
        parent: u64,
        child: u64,
    },
    DestroyNode {
        node: u64,
    },
    SetAttribute {
        node: u64,
        name: String,
        value: Option<String>,
    },
    AddEventListener {
        node: u64,
        listener: u64,
        name: String,
    },
    RemoveEventListener {
        node: u64,
        listener: u64,
    },
    Flush {
        root: u64,
    },
}

impl FiberMutation {
    fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::CreateElement { node, tag } => {
                validate_id(*node, "node")?;
                if !valid_tag(tag) {
                    return Err(ProtocolError::InvalidName("tag"));
                }
            }
            Self::CreateText { node, .. } | Self::DestroyNode { node } => {
                validate_id(*node, "node")?;
            }
            Self::InsertBefore {
                parent,
                child,
                reference,
            } => {
                validate_id(*parent, "parent")?;
                validate_id(*child, "child")?;
                if let Some(reference) = reference {
                    validate_id(*reference, "reference")?;
                }
            }
            Self::Remove { parent, child } => {
                validate_id(*parent, "parent")?;
                validate_id(*child, "child")?;
            }
            Self::SetAttribute { node, name, .. } => {
                validate_id(*node, "node")?;
                if !valid_attribute_name(name) {
                    return Err(ProtocolError::InvalidName("attribute"));
                }
            }
            Self::AddEventListener {
                node,
                listener,
                name,
            } => {
                validate_id(*node, "node")?;
                validate_id(*listener, "listener")?;
                if name != "tap" {
                    return Err(ProtocolError::InvalidEvent(name.clone()));
                }
            }
            Self::RemoveEventListener { node, listener } => {
                validate_id(*node, "node")?;
                validate_id(*listener, "listener")?;
            }
            Self::Flush { root } => validate_id(*root, "root")?,
        }
        Ok(())
    }
}

fn validate_version(version: u32) -> Result<(), ProtocolError> {
    if version == PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(ProtocolError::UnsupportedVersion(version))
    }
}

fn validate_operations(
    operations: &[FiberMutation],
    allow_empty: bool,
) -> Result<(), ProtocolError> {
    if operations.is_empty() {
        return if allow_empty {
            Ok(())
        } else {
            Err(ProtocolError::MissingFlush)
        };
    }

    let last = operations.len() - 1;
    let mut flush_count = 0;
    for (index, operation) in operations.iter().enumerate() {
        operation.validate()?;
        if matches!(operation, FiberMutation::Flush { .. }) {
            flush_count += 1;
            if index != last {
                return Err(ProtocolError::FlushNotFinal);
            }
        }
    }

    match flush_count {
        0 => Err(ProtocolError::MissingFlush),
        1 => Ok(()),
        _ => Err(ProtocolError::MultipleFlushes),
    }
}

fn validate_id(value: u64, field: &'static str) -> Result<(), ProtocolError> {
    if value == 0 {
        Err(ProtocolError::ZeroId(field))
    } else if value > JS_MAX_SAFE_INTEGER {
        Err(ProtocolError::UnsafeId(field, value))
    } else {
        Ok(())
    }
}

fn valid_tag(name: &str) -> bool {
    name.len() <= 64
        && !matches!(
            name,
            "block"
                | "component"
                | "for"
                | "if"
                | "list"
                | "list-container"
                | "list-item"
                | "none"
                | "page"
                | "raw-text"
                | "waterfall"
                | "wrapper"
        )
        && !name.is_empty()
        && name.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                byte.is_ascii_lowercase()
            } else {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
            }
        })
}

fn valid_attribute_name(name: &str) -> bool {
    name.len() <= 128
        && !matches!(name, "constructor" | "prototype")
        && !name.is_empty()
        && name.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                byte.is_ascii_lowercase()
            } else {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'_' | b'.' | b':' | b'-')
            }
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    Json(String),
    UnsupportedVersion(u32),
    InvalidSuccessFlag,
    InvalidFailureFlag,
    ZeroFailureStatus,
    ZeroId(&'static str),
    UnsafeId(&'static str, u64),
    InvalidName(&'static str),
    InvalidEvent(String),
    MissingFlush,
    MultipleFlushes,
    FlushNotFinal,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message) => write!(formatter, "invalid protocol JSON: {message}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported protocol version {version}")
            }
            Self::InvalidSuccessFlag => formatter.write_str("success response must have ok=true"),
            Self::InvalidFailureFlag => formatter.write_str("failure response must have ok=false"),
            Self::ZeroFailureStatus => formatter.write_str("failure status must not be zero"),
            Self::ZeroId(field) => write!(formatter, "{field} ID must not be zero"),
            Self::UnsafeId(field, value) => write!(
                formatter,
                "{field} ID {value} exceeds JavaScript Number.MAX_SAFE_INTEGER"
            ),
            Self::InvalidName(field) => write!(formatter, "{field} name is invalid"),
            Self::InvalidEvent(event) => {
                write!(formatter, "event `{event}` is not supported by protocol v1")
            }
            Self::MissingFlush => formatter.write_str("mutation response has no flush boundary"),
            Self::MultipleFlushes => {
                formatter.write_str("mutation response has multiple flush boundaries")
            }
            Self::FlushNotFinal => formatter.write_str("flush must be the final operation"),
        }
    }
}

impl std::error::Error for ProtocolError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendError {
    InvalidRoot(u64),
    InvalidNode(u64),
    InvalidListener(u64),
    EventMismatch {
        listener: u64,
        expected: String,
        actual: String,
    },
    InvalidMutation(String),
    IdExhausted(&'static str),
    Protocol(ProtocolError),
}

impl fmt::Display for BackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRoot(root) => write!(formatter, "invalid root ID {root}"),
            Self::InvalidNode(node) => write!(formatter, "invalid or stale node ID {node}"),
            Self::InvalidListener(listener) => {
                write!(formatter, "invalid or stale listener ID {listener}")
            }
            Self::EventMismatch {
                listener,
                expected,
                actual,
            } => write!(
                formatter,
                "listener {listener} expects event `{expected}`, not `{actual}`"
            ),
            Self::InvalidMutation(message) => formatter.write_str(message),
            Self::IdExhausted(kind) => write!(formatter, "{kind} ID space is exhausted"),
            Self::Protocol(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for BackendError {}

impl From<ProtocolError> for BackendError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum NodeKind {
    Root,
    Element(String),
    RawText,
}

#[derive(Debug)]
struct NodeState {
    kind: NodeKind,
    parent: Option<NativeNode>,
    children: Vec<NativeNode>,
}

struct ListenerState {
    node: NativeNode,
    name: String,
    callback: Rc<dyn Fn(NativeEvent)>,
}

pub struct RecordingBackend {
    root: NativeNode,
    next_node: Cell<u64>,
    next_listener: Cell<u64>,
    nodes: RefCell<HashMap<NativeNode, NodeState>>,
    listeners: RefCell<HashMap<NativeListener, ListenerState>>,
    pending: RefCell<Vec<FiberMutation>>,
    poisoned: Cell<bool>,
    error: RefCell<Option<BackendError>>,
}

impl fmt::Debug for RecordingBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RecordingBackend")
            .field("root", &self.root)
            .field("poisoned", &self.poisoned)
            .finish_non_exhaustive()
    }
}

impl RecordingBackend {
    pub fn new(root: NativeNode) -> Result<Rc<Self>, BackendError> {
        if root.0 == 0 || root.0 > JS_MAX_SAFE_INTEGER {
            return Err(BackendError::InvalidRoot(root.0));
        }
        Ok(Rc::new(Self {
            root,
            next_node: Cell::new(1),
            next_listener: Cell::new(1),
            nodes: RefCell::new(HashMap::from([(
                root,
                NodeState {
                    kind: NodeKind::Root,
                    parent: None,
                    children: Vec::new(),
                },
            )])),
            listeners: RefCell::new(HashMap::new()),
            pending: RefCell::new(Vec::new()),
            poisoned: Cell::new(false),
            error: RefCell::new(None),
        }))
    }

    pub fn root(&self) -> NativeNode {
        self.root
    }

    pub fn dispatch(&self, listener: NativeListener, event: &str) -> Result<(), BackendError> {
        if let Some(error) = self.current_error() {
            return Err(error);
        }
        let (expected, callback) = {
            let listeners = match self.listeners.try_borrow() {
                Ok(listeners) => listeners,
                Err(_) => {
                    let error = BackendError::InvalidMutation(
                        "listener registry is already borrowed".into(),
                    );
                    self.record_error(error.clone());
                    return Err(error);
                }
            };
            let state = listeners
                .get(&listener)
                .ok_or(BackendError::InvalidListener(listener.0))?;
            (state.name.clone(), Rc::clone(&state.callback))
        };
        if event != expected {
            return Err(BackendError::EventMismatch {
                listener: listener.0,
                expected,
                actual: event.into(),
            });
        }
        callback(NativeEvent::new(event));
        Ok(())
    }

    pub fn take_response(&self) -> Result<SuccessResponse, BackendError> {
        if let Some(error) = self.current_error() {
            return Err(error);
        }
        let mut operations = match self.pending.try_borrow() {
            Ok(pending) => pending.clone(),
            Err(_) => {
                let error = BackendError::InvalidMutation(
                    "pending mutation list is already borrowed".into(),
                );
                self.record_error(error.clone());
                return Err(error);
            }
        };
        operations.push(FiberMutation::Flush { root: self.root.0 });
        let response = match SuccessResponse::new(operations) {
            Ok(response) => response,
            Err(error) => {
                let error = BackendError::Protocol(error);
                self.record_error(error.clone());
                return Err(error);
            }
        };
        match self.pending.try_borrow_mut() {
            Ok(mut pending) => pending.clear(),
            Err(_) => {
                let error = BackendError::InvalidMutation(
                    "pending mutation list is already borrowed".into(),
                );
                self.record_error(error.clone());
                return Err(error);
            }
        }
        Ok(response)
    }

    pub fn discard_pending(&self) {
        if let Ok(mut pending) = self.pending.try_borrow_mut() {
            pending.clear();
        }
    }

    fn current_error(&self) -> Option<BackendError> {
        if !self.poisoned.get() {
            return None;
        }
        self.error
            .try_borrow()
            .ok()
            .and_then(|error| error.clone())
            .or_else(|| {
                Some(BackendError::InvalidMutation(
                    "backend is permanently poisoned".into(),
                ))
            })
    }

    fn record_error(&self, error: BackendError) {
        if self.poisoned.replace(true) {
            return;
        }
        if let Ok(mut current) = self.error.try_borrow_mut() {
            *current = Some(error);
        }
        if let Ok(mut pending) = self.pending.try_borrow_mut() {
            pending.clear();
        }
    }

    fn record(&self, operation: FiberMutation) {
        if self.poisoned.get() {
            return;
        }
        if let Ok(mut pending) = self.pending.try_borrow_mut() {
            pending.push(operation);
        } else {
            self.record_error(BackendError::InvalidMutation(
                "pending mutation list is already borrowed".into(),
            ));
        }
    }

    fn allocate_node(&self) -> Option<NativeNode> {
        loop {
            let id = self.next_node.get();
            if id == 0 || id > JS_MAX_SAFE_INTEGER {
                self.record_error(BackendError::IdExhausted("node"));
                return None;
            }
            self.next_node
                .set(if id == JS_MAX_SAFE_INTEGER { 0 } else { id + 1 });
            if id != self.root.0 {
                return Some(NativeNode(id));
            }
        }
    }

    fn allocate_listener(&self) -> Option<NativeListener> {
        let id = self.next_listener.get();
        if id == 0 || id > JS_MAX_SAFE_INTEGER {
            self.record_error(BackendError::IdExhausted("listener"));
            return None;
        }
        self.next_listener
            .set(if id == JS_MAX_SAFE_INTEGER { 0 } else { id + 1 });
        Some(NativeListener(id))
    }

    fn create_node(
        &self,
        kind: NodeKind,
        operation: impl FnOnce(u64) -> FiberMutation,
    ) -> NativeNode {
        if self.poisoned.get() {
            return NativeNode(0);
        }
        let Some(node) = self.allocate_node() else {
            return NativeNode(0);
        };
        let Ok(mut nodes) = self.nodes.try_borrow_mut() else {
            self.record_error(BackendError::InvalidMutation(
                "node registry is already borrowed".into(),
            ));
            return NativeNode(0);
        };
        if nodes.contains_key(&node) {
            drop(nodes);
            self.record_error(BackendError::InvalidMutation(format!(
                "duplicate node ID {}",
                node.0
            )));
            return NativeNode(0);
        }
        nodes.insert(
            node,
            NodeState {
                kind,
                parent: None,
                children: Vec::new(),
            },
        );
        drop(nodes);
        self.record(operation(node.0));
        node
    }
}

impl NativeRendererBackend for RecordingBackend {
    fn create_element(&self, tag: &str) -> NativeNode {
        if !valid_tag(tag) {
            self.record_error(BackendError::InvalidMutation(format!(
                "element tag `{tag}` is not a supported authored tag"
            )));
            return NativeNode(0);
        }
        self.create_node(NodeKind::Element(tag.into()), |node| {
            FiberMutation::CreateElement {
                node,
                tag: tag.into(),
            }
        })
    }

    fn create_text(&self, text: &str) -> NativeNode {
        self.create_node(NodeKind::RawText, |node| FiberMutation::CreateText {
            node,
            text: text.into(),
        })
    }

    fn insert_before(&self, parent: NativeNode, child: NativeNode, reference: Option<NativeNode>) {
        if self.poisoned.get() {
            return;
        }
        let Ok(mut nodes) = self.nodes.try_borrow_mut() else {
            self.record_error(BackendError::InvalidMutation(
                "node registry is already borrowed".into(),
            ));
            return;
        };
        let Some(parent_state) = nodes.get(&parent) else {
            drop(nodes);
            self.record_error(BackendError::InvalidNode(parent.0));
            return;
        };
        let Some(child_state) = nodes.get(&child) else {
            drop(nodes);
            self.record_error(BackendError::InvalidNode(child.0));
            return;
        };
        if matches!(parent_state.kind, NodeKind::RawText) {
            drop(nodes);
            self.record_error(BackendError::InvalidMutation(format!(
                "raw-text node {} cannot be a parent",
                parent.0
            )));
            return;
        }
        if matches!(child_state.kind, NodeKind::RawText)
            && !matches!(&parent_state.kind, NodeKind::Element(tag) if tag == "text")
        {
            drop(nodes);
            self.record_error(BackendError::InvalidMutation(format!(
                "raw-text node {} must be attached to a text element",
                child.0
            )));
            return;
        }
        if child == self.root || child_state.parent.is_some() {
            drop(nodes);
            self.record_error(BackendError::InvalidMutation(format!(
                "node {} is not detached",
                child.0
            )));
            return;
        }

        let mut ancestor = Some(parent);
        while let Some(node) = ancestor {
            if node == child {
                drop(nodes);
                self.record_error(BackendError::InvalidMutation(format!(
                    "inserting node {} under {} would create a cycle",
                    child.0, parent.0
                )));
                return;
            }
            ancestor = nodes.get(&node).and_then(|state| state.parent);
        }

        let index = match reference {
            Some(reference) => {
                let Some(index) = parent_state
                    .children
                    .iter()
                    .position(|candidate| *candidate == reference)
                else {
                    drop(nodes);
                    self.record_error(BackendError::InvalidMutation(format!(
                        "reference node {} is not a child of {}",
                        reference.0, parent.0
                    )));
                    return;
                };
                index
            }
            None => parent_state.children.len(),
        };
        let Some(parent_state) = nodes.get_mut(&parent) else {
            drop(nodes);
            self.record_error(BackendError::InvalidNode(parent.0));
            return;
        };
        parent_state.children.insert(index, child);
        let Some(child_state) = nodes.get_mut(&child) else {
            drop(nodes);
            self.record_error(BackendError::InvalidNode(child.0));
            return;
        };
        child_state.parent = Some(parent);
        drop(nodes);
        self.record(FiberMutation::InsertBefore {
            parent: parent.0,
            child: child.0,
            reference: reference.map(|node| node.0),
        });
    }

    fn remove(&self, parent: NativeNode, child: NativeNode) {
        if self.poisoned.get() {
            return;
        }
        let Ok(mut nodes) = self.nodes.try_borrow_mut() else {
            self.record_error(BackendError::InvalidMutation(
                "node registry is already borrowed".into(),
            ));
            return;
        };
        let Some(child_state) = nodes.get(&child) else {
            drop(nodes);
            self.record_error(BackendError::InvalidNode(child.0));
            return;
        };
        if child_state.parent != Some(parent) {
            drop(nodes);
            self.record_error(BackendError::InvalidMutation(format!(
                "node {} is not a child of {}",
                child.0, parent.0
            )));
            return;
        }
        let Some(parent_state) = nodes.get_mut(&parent) else {
            drop(nodes);
            self.record_error(BackendError::InvalidNode(parent.0));
            return;
        };
        let Some(index) = parent_state
            .children
            .iter()
            .position(|candidate| *candidate == child)
        else {
            drop(nodes);
            self.record_error(BackendError::InvalidMutation(format!(
                "parent {} does not contain child {}",
                parent.0, child.0
            )));
            return;
        };
        parent_state.children.remove(index);
        let Some(child_state) = nodes.get_mut(&child) else {
            drop(nodes);
            self.record_error(BackendError::InvalidNode(child.0));
            return;
        };
        child_state.parent = None;
        drop(nodes);
        self.record(FiberMutation::Remove {
            parent: parent.0,
            child: child.0,
        });
    }

    fn destroy_node(&self, node: NativeNode) {
        if self.poisoned.get() {
            return;
        }
        let Ok(listeners) = self.listeners.try_borrow() else {
            self.record_error(BackendError::InvalidMutation(
                "listener registry is already borrowed".into(),
            ));
            return;
        };
        if listeners.values().any(|listener| listener.node == node) {
            drop(listeners);
            self.record_error(BackendError::InvalidMutation(format!(
                "node {} still has listeners",
                node.0
            )));
            return;
        }
        drop(listeners);

        let Ok(mut nodes) = self.nodes.try_borrow_mut() else {
            self.record_error(BackendError::InvalidMutation(
                "node registry is already borrowed".into(),
            ));
            return;
        };
        let Some(state) = nodes.get(&node) else {
            drop(nodes);
            self.record_error(BackendError::InvalidNode(node.0));
            return;
        };
        if node == self.root || state.parent.is_some() || !state.children.is_empty() {
            drop(nodes);
            self.record_error(BackendError::InvalidMutation(format!(
                "node {} is not detached and empty",
                node.0
            )));
            return;
        }
        nodes.remove(&node);
        drop(nodes);
        self.record(FiberMutation::DestroyNode { node: node.0 });
    }

    fn set_attribute(&self, node: NativeNode, name: &str, value: Option<&str>) {
        if self.poisoned.get() {
            return;
        }
        let Ok(nodes) = self.nodes.try_borrow() else {
            self.record_error(BackendError::InvalidMutation(
                "node registry is already borrowed".into(),
            ));
            return;
        };
        let Some(state) = nodes.get(&node) else {
            drop(nodes);
            self.record_error(BackendError::InvalidNode(node.0));
            return;
        };
        if !matches!(state.kind, NodeKind::Element(_)) || !valid_attribute_name(name) {
            drop(nodes);
            self.record_error(BackendError::InvalidMutation(format!(
                "invalid attribute `{name}` for node {}",
                node.0
            )));
            return;
        }
        drop(nodes);
        self.record(FiberMutation::SetAttribute {
            node: node.0,
            name: name.into(),
            value: value.map(Into::into),
        });
    }

    fn add_event_listener(
        &self,
        node: NativeNode,
        name: &str,
        callback: Box<dyn Fn(NativeEvent)>,
    ) -> NativeListener {
        if self.poisoned.get() {
            return NativeListener(0);
        }
        let Ok(nodes) = self.nodes.try_borrow() else {
            self.record_error(BackendError::InvalidMutation(
                "node registry is already borrowed".into(),
            ));
            return NativeListener(0);
        };
        let valid_node = nodes
            .get(&node)
            .is_some_and(|state| matches!(state.kind, NodeKind::Element(_)));
        drop(nodes);
        if !valid_node {
            self.record_error(BackendError::InvalidNode(node.0));
            return NativeListener(0);
        }
        if name != "tap" {
            self.record_error(BackendError::InvalidMutation(format!(
                "event `{name}` is not supported by protocol v1"
            )));
            return NativeListener(0);
        }

        let Ok(mut listeners) = self.listeners.try_borrow_mut() else {
            self.record_error(BackendError::InvalidMutation(
                "listener registry is already borrowed".into(),
            ));
            return NativeListener(0);
        };
        if listeners
            .values()
            .any(|listener| listener.node == node && listener.name == name)
        {
            drop(listeners);
            self.record_error(BackendError::InvalidMutation(format!(
                "node {} already has a `{name}` listener",
                node.0
            )));
            return NativeListener(0);
        }
        let Some(listener) = self.allocate_listener() else {
            return NativeListener(0);
        };
        listeners.insert(
            listener,
            ListenerState {
                node,
                name: name.into(),
                callback: Rc::from(callback),
            },
        );
        drop(listeners);
        self.record(FiberMutation::AddEventListener {
            node: node.0,
            listener: listener.0,
            name: name.into(),
        });
        listener
    }

    fn remove_event_listener(&self, node: NativeNode, listener: NativeListener) {
        if self.poisoned.get() {
            return;
        }
        let Ok(mut listeners) = self.listeners.try_borrow_mut() else {
            self.record_error(BackendError::InvalidMutation(
                "listener registry is already borrowed".into(),
            ));
            return;
        };
        let Some(state) = listeners.get(&listener) else {
            drop(listeners);
            self.record_error(BackendError::InvalidListener(listener.0));
            return;
        };
        if state.node != node {
            drop(listeners);
            self.record_error(BackendError::InvalidMutation(format!(
                "listener {} is not attached to node {}",
                listener.0, node.0
            )));
            return;
        }
        listeners.remove(&listener);
        drop(listeners);
        self.record(FiberMutation::RemoveEventListener {
            node: node.0,
            listener: listener.0,
        });
    }

    fn flush(&self, root: NativeNode) {
        if self.poisoned.get() {
            return;
        }
        if root != self.root {
            self.record_error(BackendError::InvalidRoot(root.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;

    #[test]
    fn protocol_envelopes_are_exact_and_reject_unknown_fields() {
        let success = ProtocolResponse::success(vec![FiberMutation::Flush { root: 1 }]).unwrap();
        let json = success.to_json().unwrap();
        assert_eq!(ProtocolResponse::from_json(&json).unwrap(), success);
        assert_eq!(
            String::from_utf8(json).unwrap(),
            r#"{"version":1,"ok":true,"operations":[{"op":"flush","root":1}]}"#
        );

        let failure = ProtocolResponse::failure(8, "backend failed", Vec::new()).unwrap();
        let json = failure.to_json().unwrap();
        assert_eq!(ProtocolResponse::from_json(&json).unwrap(), failure);
        assert_eq!(
            String::from_utf8(json).unwrap(),
            r#"{"version":1,"ok":false,"status":8,"error":"backend failed","operations":[]}"#
        );

        assert!(
            ProtocolResponse::from_json(
                br#"{"version":1,"ok":true,"operations":[{"op":"flush","root":1}],"extra":0}"#
            )
            .is_err()
        );
        assert!(
            ProtocolResponse::from_json(
                br#"{"version":1,"ok":true,"operations":[{"op":"flush","root":1,"extra":0}]}"#
            )
            .is_err()
        );
    }

    #[test]
    fn ids_are_positive_javascript_safe_integers() {
        let at_max = ProtocolResponse::success(vec![FiberMutation::Flush {
            root: JS_MAX_SAFE_INTEGER,
        }])
        .unwrap();
        assert!(at_max.validate().is_ok());
        assert!(
            ProtocolResponse::success(vec![FiberMutation::Flush {
                root: JS_MAX_SAFE_INTEGER + 1,
            }])
            .is_err()
        );
        assert!(
            ProtocolResponse::success(vec![
                FiberMutation::AddEventListener {
                    node: 1,
                    listener: JS_MAX_SAFE_INTEGER,
                    name: "tap".into(),
                },
                FiberMutation::Flush { root: 1 },
            ])
            .is_ok()
        );
        assert!(
            ProtocolResponse::success(vec![
                FiberMutation::AddEventListener {
                    node: 1,
                    listener: JS_MAX_SAFE_INTEGER + 1,
                    name: "tap".into(),
                },
                FiberMutation::Flush { root: 1 },
            ])
            .is_err()
        );
        assert!(RecordingBackend::new(NativeNode(JS_MAX_SAFE_INTEGER)).is_ok());
        assert_eq!(
            RecordingBackend::new(NativeNode(JS_MAX_SAFE_INTEGER + 1)).unwrap_err(),
            BackendError::InvalidRoot(JS_MAX_SAFE_INTEGER + 1)
        );

        let backend = RecordingBackend::new(NativeNode(1)).unwrap();
        backend.next_node.set(JS_MAX_SAFE_INTEGER);
        assert_eq!(backend.create_element("view").0, JS_MAX_SAFE_INTEGER);
        assert_eq!(backend.create_element("view"), NativeNode(0));
        assert_eq!(
            backend.take_response(),
            Err(BackendError::IdExhausted("node"))
        );

        let backend = RecordingBackend::new(NativeNode(1)).unwrap();
        let first = backend.create_element("view");
        let second = backend.create_element("view");
        backend.next_listener.set(JS_MAX_SAFE_INTEGER);
        assert_eq!(
            backend.add_event_listener(first, "tap", Box::new(|_| {})).0,
            JS_MAX_SAFE_INTEGER
        );
        assert_eq!(
            backend.add_event_listener(second, "tap", Box::new(|_| {})),
            NativeListener(0)
        );
        assert_eq!(
            backend.take_response(),
            Err(BackendError::IdExhausted("listener"))
        );
    }

    #[test]
    fn allocation_is_independent_of_root_and_flushes_are_coalesced() {
        let backend = RecordingBackend::new(NativeNode(JS_MAX_SAFE_INTEGER)).unwrap();
        assert_eq!(backend.create_element("view"), NativeNode(1));
        backend.flush(backend.root());
        backend.flush(backend.root());

        let response = backend.take_response().unwrap();
        assert_eq!(
            response
                .operations
                .iter()
                .filter(|operation| matches!(operation, FiberMutation::Flush { .. }))
                .count(),
            1
        );
        assert_eq!(
            response.operations.last(),
            Some(&FiberMutation::Flush {
                root: JS_MAX_SAFE_INTEGER
            })
        );

        backend.flush(backend.root());
        assert_eq!(
            backend.take_response().unwrap().operations,
            vec![FiberMutation::Flush {
                root: JS_MAX_SAFE_INTEGER
            }]
        );
    }

    #[test]
    fn raw_text_cycles_invalid_names_and_duplicate_events_poison_permanently() {
        let raw_backend = RecordingBackend::new(NativeNode(1)).unwrap();
        let raw = raw_backend.create_text("raw");
        raw_backend.insert_before(NativeNode(1), raw, None);
        let first_error = raw_backend.take_response().unwrap_err();
        assert_eq!(raw_backend.take_response(), Err(first_error));

        let raw_parent_backend = RecordingBackend::new(NativeNode(1)).unwrap();
        let raw_parent = raw_parent_backend.create_text("raw");
        let child = raw_parent_backend.create_element("view");
        raw_parent_backend.insert_before(raw_parent, child, None);
        assert!(raw_parent_backend.take_response().is_err());

        let cycle_backend = RecordingBackend::new(NativeNode(1)).unwrap();
        let outer = cycle_backend.create_element("view");
        let inner = cycle_backend.create_element("view");
        cycle_backend.insert_before(NativeNode(1), outer, None);
        cycle_backend.insert_before(outer, inner, None);
        cycle_backend.remove(NativeNode(1), outer);
        cycle_backend.insert_before(inner, outer, None);
        let first_error = cycle_backend.take_response().unwrap_err();
        cycle_backend.flush(NativeNode(1));
        assert_eq!(cycle_backend.take_response(), Err(first_error));

        let tag_backend = RecordingBackend::new(NativeNode(1)).unwrap();
        assert_eq!(tag_backend.create_element("page"), NativeNode(0));
        assert!(tag_backend.take_response().is_err());

        let structural_tag_backend = RecordingBackend::new(NativeNode(1)).unwrap();
        assert_eq!(structural_tag_backend.create_element("list"), NativeNode(0));
        assert!(structural_tag_backend.take_response().is_err());

        let attribute_backend = RecordingBackend::new(NativeNode(1)).unwrap();
        let node = attribute_backend.create_element("view");
        attribute_backend.set_attribute(node, "prototype", Some("value"));
        assert!(attribute_backend.take_response().is_err());

        let event_backend = RecordingBackend::new(NativeNode(1)).unwrap();
        let node = event_backend.create_element("view");
        assert_eq!(
            event_backend.add_event_listener(node, "click", Box::new(|_| {})),
            NativeListener(0)
        );
        assert!(event_backend.take_response().is_err());

        let duplicate_backend = RecordingBackend::new(NativeNode(1)).unwrap();
        let node = duplicate_backend.create_element("view");
        duplicate_backend.add_event_listener(node, "tap", Box::new(|_| {}));
        assert_eq!(
            duplicate_backend.add_event_listener(node, "tap", Box::new(|_| {})),
            NativeListener(0)
        );
        assert!(duplicate_backend.take_response().is_err());
    }

    #[test]
    fn no_op_listener_still_returns_one_final_flush() {
        let backend = RecordingBackend::new(NativeNode(1)).unwrap();
        let node = backend.create_element("view");
        let listener = backend.add_event_listener(node, "tap", Box::new(|_| {}));
        backend.take_response().unwrap();

        backend.dispatch(listener, "tap").unwrap();

        assert_eq!(
            backend.take_response().unwrap().operations,
            vec![FiberMutation::Flush { root: 1 }]
        );
    }

    #[test]
    fn stale_listener_is_recoverable_but_backend_mutation_errors_are_not() {
        let backend = RecordingBackend::new(NativeNode(1)).unwrap();
        let node = backend.create_element("view");
        let listener = backend.add_event_listener(node, "tap", Box::new(|_| {}));
        backend.remove_event_listener(node, listener);
        assert_eq!(
            backend.dispatch(listener, "tap"),
            Err(BackendError::InvalidListener(listener.0))
        );
        assert!(backend.take_response().is_ok());

        backend.insert_before(NativeNode(1), NativeNode(99), None);
        let error = backend.take_response().unwrap_err();
        assert_eq!(backend.take_response(), Err(error));
    }

    #[test]
    fn callback_panics_can_be_contained_by_the_caller() {
        let backend = RecordingBackend::new(NativeNode(1)).unwrap();
        let node = backend.create_element("view");
        let listener =
            backend.add_event_listener(node, "tap", Box::new(|_| panic!("callback panic")));

        let result = catch_unwind(AssertUnwindSafe(|| backend.dispatch(listener, "tap")));

        assert!(result.is_err());
    }
}
