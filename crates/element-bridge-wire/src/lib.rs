#![deny(unsafe_code)]

use std::fmt;

use flatbuffers::{FlatBufferBuilder, WIPOffset};
use lynx_element_bridge_core::{
    BridgeError, CapabilityRequest, Command as CoreCommand, CommandBatch as CoreCommandBatch,
    CommandItem as CoreCommandItem, CommandResult as CoreCommandResult, EventMessage as CoreEvent,
    NodeId, PROTOCOL_VERSION, ResponseBatch as CoreResponse, ResultSlot,
    ResultValue as CoreResultValue, SessionId, Status as CoreStatus,
};

#[allow(
    clippy::all,
    dead_code,
    deprecated,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    unused_imports
)]
pub mod generated {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../protocol/generated/rust/element_bridge_v2_generated.rs"
    ));
}

use generated::lynx::element_bridge::v2 as fb;

pub const FILE_IDENTIFIER: &str = fb::ENVELOPE_IDENTIFIER;
const NO_RESULT_SLOT: u32 = u32::MAX;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WireError {
    InvalidBuffer(String),
    UnsupportedVersion(u16),
    ChannelMismatch,
    InvalidId(&'static str),
    UnsupportedCommand(String),
}

impl fmt::Display for WireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBuffer(message) => {
                write!(formatter, "invalid FlatBuffers v2 payload: {message}")
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported protocol version {version}")
            }
            Self::ChannelMismatch => {
                formatter.write_str("envelope channel and message do not match")
            }
            Self::InvalidId(kind) => write!(formatter, "{kind} ID must not be zero"),
            Self::UnsupportedCommand(command) => {
                write!(formatter, "command `{command}` has no typed v2 encoding")
            }
        }
    }
}

impl std::error::Error for WireError {}

pub fn encode_create_session(
    root: NodeId,
    capabilities: &[CapabilityRequest],
) -> Result<Vec<u8>, WireError> {
    let mut builder = FlatBufferBuilder::new();
    let requests = capabilities
        .iter()
        .map(|capability| {
            let name = builder.create_string(&capability.name);
            fb::CapabilityRequest::create(
                &mut builder,
                &fb::CapabilityRequestArgs {
                    name: Some(name),
                    required: capability.required,
                },
            )
        })
        .collect::<Vec<_>>();
    let requests = builder.create_vector(&requests);
    let request = fb::CreateSessionRequest::create(
        &mut builder,
        &fb::CreateSessionRequestArgs {
            root_id: root.get(),
            capabilities: Some(requests),
        },
    );
    finish_envelope(
        &mut builder,
        fb::Channel::COMMAND,
        fb::Message::CreateSessionRequest,
        request.as_union_value(),
    );
    Ok(builder.finished_data().to_vec())
}

pub fn encode_command_batch(batch: &CoreCommandBatch) -> Result<Vec<u8>, WireError> {
    let mut builder = FlatBufferBuilder::new();
    let commands = batch
        .commands
        .iter()
        .map(|command| encode_command(&mut builder, command))
        .collect::<Result<Vec<_>, _>>()?;
    let commands = builder.create_vector(&commands);
    let command_batch = fb::CommandBatch::create(
        &mut builder,
        &fb::CommandBatchArgs {
            session_id: batch.session.get(),
            sequence: batch.sequence,
            commands: Some(commands),
            final_commit: batch.final_commit,
        },
    );
    finish_envelope(
        &mut builder,
        fb::Channel::COMMAND,
        fb::Message::CommandBatch,
        command_batch.as_union_value(),
    );
    Ok(builder.finished_data().to_vec())
}

pub fn encode_response(response: &CoreResponse) -> Result<Vec<u8>, WireError> {
    let mut builder = FlatBufferBuilder::new();
    let results = response
        .results
        .iter()
        .map(|result| encode_result(&mut builder, result))
        .collect::<Vec<_>>();
    let results = builder.create_vector(&results);
    let message = response
        .message
        .as_ref()
        .map(|message| builder.create_string(message));
    let response = fb::ResponseBatch::create(
        &mut builder,
        &fb::ResponseBatchArgs {
            session_id: response.session.map_or(0, SessionId::get),
            sequence: response.sequence,
            status: encode_status(response.status),
            message,
            results: Some(results),
            committed: response.committed,
        },
    );
    finish_envelope(
        &mut builder,
        fb::Channel::RESULT,
        fb::Message::ResponseBatch,
        response.as_union_value(),
    );
    Ok(builder.finished_data().to_vec())
}

pub fn encode_failure(session: u32, sequence: u32, status: CoreStatus, message: &str) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let message = builder.create_string(message);
    let response = fb::ResponseBatch::create(
        &mut builder,
        &fb::ResponseBatchArgs {
            session_id: session,
            sequence,
            status: encode_status(status),
            message: Some(message),
            results: None,
            committed: false,
        },
    );
    finish_envelope(
        &mut builder,
        fb::Channel::RESULT,
        fb::Message::ResponseBatch,
        response.as_union_value(),
    );
    builder.finished_data().to_vec()
}

pub fn encode_event(event: &CoreEvent) -> Result<Vec<u8>, WireError> {
    let mut builder = FlatBufferBuilder::new();
    let content_type = builder.create_string(&event.content_type);
    let payload = builder.create_vector(&event.payload);
    let event = fb::EventMessage::create(
        &mut builder,
        &fb::EventMessageArgs {
            session_id: event.session.get(),
            listener_id: event.listener.get(),
            callback_id: event.callback.get(),
            content_type: Some(content_type),
            payload: Some(payload),
        },
    );
    finish_envelope(
        &mut builder,
        fb::Channel::EVENT,
        fb::Message::EventMessage,
        event.as_union_value(),
    );
    Ok(builder.finished_data().to_vec())
}

pub fn decode_response(bytes: &[u8]) -> Result<CoreResponse, WireError> {
    let envelope = verified_envelope(bytes)?;
    if envelope.channel() != fb::Channel::RESULT
        || envelope.message_type() != fb::Message::ResponseBatch
    {
        return Err(WireError::ChannelMismatch);
    }
    let response = envelope
        .message_as_response_batch()
        .ok_or(WireError::ChannelMismatch)?;
    let results = response
        .results()
        .map(|results| {
            results
                .iter()
                .map(|result| {
                    Ok(CoreCommandResult {
                        slot: (result.slot() != NO_RESULT_SLOT)
                            .then(|| ResultSlot::new(result.slot())),
                        status: decode_status(result.status())?,
                        message: result.message().map(Into::into),
                        value: decode_result_value(result)?,
                    })
                })
                .collect::<Result<Vec<_>, WireError>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(CoreResponse {
        session: (response.session_id() != 0)
            .then(|| SessionId::new(response.session_id()))
            .transpose()
            .map_err(|_| WireError::InvalidId("session"))?,
        sequence: response.sequence(),
        status: decode_status(response.status())?,
        message: response.message().map(Into::into),
        results,
        committed: response.committed(),
    })
}

pub fn decode_command_batch(bytes: &[u8]) -> Result<CoreCommandBatch, WireError> {
    let envelope = verified_envelope(bytes)?;
    if envelope.channel() != fb::Channel::COMMAND
        || envelope.message_type() != fb::Message::CommandBatch
    {
        return Err(WireError::ChannelMismatch);
    }
    let batch = envelope
        .message_as_command_batch()
        .ok_or(WireError::ChannelMismatch)?;
    let commands = batch
        .commands()
        .map(|commands| commands.iter().map(decode_command).collect())
        .transpose()?
        .unwrap_or_default();
    Ok(CoreCommandBatch {
        session: SessionId::new(batch.session_id()).map_err(|_| WireError::InvalidId("session"))?,
        sequence: batch.sequence(),
        commands,
        final_commit: batch.final_commit(),
    })
}

pub fn decode_event(bytes: &[u8]) -> Result<CoreEvent, WireError> {
    let envelope = verified_envelope(bytes)?;
    if envelope.channel() != fb::Channel::EVENT
        || envelope.message_type() != fb::Message::EventMessage
    {
        return Err(WireError::ChannelMismatch);
    }
    let event = envelope
        .message_as_event_message()
        .ok_or(WireError::ChannelMismatch)?;
    Ok(CoreEvent {
        session: SessionId::new(event.session_id()).map_err(|_| WireError::InvalidId("session"))?,
        listener: lynx_element_bridge_core::ListenerId::new(event.listener_id())
            .map_err(|_| WireError::InvalidId("listener"))?,
        callback: lynx_element_bridge_core::CallbackId::new(event.callback_id())
            .map_err(|_| WireError::InvalidId("callback"))?,
        content_type: event.content_type().into(),
        payload: event
            .payload()
            .map(|payload| payload.iter().collect())
            .unwrap_or_default(),
    })
}

pub fn verify(bytes: &[u8]) -> Result<(), WireError> {
    verified_envelope(bytes).map(|_| ())
}

fn decode_command(command: fb::Command<'_>) -> Result<CoreCommandItem, WireError> {
    let node = |value| NodeId::new(value).map_err(|_| WireError::InvalidId("node"));
    let listener = |value| {
        lynx_element_bridge_core::ListenerId::new(value)
            .map_err(|_| WireError::InvalidId("listener"))
    };
    let callback = |value| {
        lynx_element_bridge_core::CallbackId::new(value)
            .map_err(|_| WireError::InvalidId("callback"))
    };
    let operation = match command.operation_type() {
        fb::ElementCommand::CreateElementCommand => {
            let value = command
                .operation_as_create_element_command()
                .ok_or_else(|| WireError::InvalidBuffer("missing CreateElement payload".into()))?;
            CoreCommand::CreateElement {
                node: node(command.result_node_id())?,
                tag: value
                    .tag()
                    .ok_or_else(|| WireError::InvalidBuffer("missing CreateElement tag".into()))?
                    .into(),
            }
        }
        fb::ElementCommand::CreateRawTextCommand => {
            let value = command
                .operation_as_create_raw_text_command()
                .ok_or_else(|| WireError::InvalidBuffer("missing CreateRawText payload".into()))?;
            CoreCommand::CreateRawText {
                node: node(command.result_node_id())?,
                text: value
                    .text()
                    .ok_or_else(|| WireError::InvalidBuffer("missing CreateRawText text".into()))?
                    .into(),
            }
        }
        fb::ElementCommand::AppendElementCommand => {
            let value = command
                .operation_as_append_element_command()
                .ok_or_else(|| WireError::InvalidBuffer("missing AppendElement payload".into()))?;
            CoreCommand::AppendElement {
                parent: node(value.parent())?,
                child: node(value.current())?,
            }
        }
        fb::ElementCommand::InsertElementBeforeCommand => {
            let value = command
                .operation_as_insert_element_before_command()
                .ok_or_else(|| {
                    WireError::InvalidBuffer("missing InsertElementBefore payload".into())
                })?;
            CoreCommand::InsertElementBefore {
                parent: node(value.parent())?,
                child: node(value.current())?,
                reference: node(value.marker().ok_or_else(|| {
                    WireError::InvalidBuffer("missing InsertElementBefore marker".into())
                })?)?,
            }
        }
        fb::ElementCommand::RemoveElementCommand => {
            let value = command
                .operation_as_remove_element_command()
                .ok_or_else(|| WireError::InvalidBuffer("missing RemoveElement payload".into()))?;
            CoreCommand::RemoveElement {
                parent: node(value.parent())?,
                child: node(value.current())?,
            }
        }
        fb::ElementCommand::ReleaseElementCommand => {
            let value = command
                .operation_as_release_element_command()
                .ok_or_else(|| WireError::InvalidBuffer("missing ReleaseElement payload".into()))?;
            CoreCommand::DestroyNode {
                node: node(value.node())?,
            }
        }
        fb::ElementCommand::SetAttributeCommand => {
            let value = command
                .operation_as_set_attribute_command()
                .ok_or_else(|| WireError::InvalidBuffer("missing SetAttribute payload".into()))?;
            let payload = value
                .value()
                .ok_or_else(|| WireError::InvalidBuffer("missing SetAttribute value".into()))?;
            let content_type = payload.content_type();
            let attribute = match content_type {
                "application/vnd.lynx-element-bridge.null" => None,
                "text/plain;charset=utf-8" => Some(
                    std::str::from_utf8(
                        payload
                            .bytes()
                            .map(|bytes| bytes.bytes())
                            .unwrap_or_default(),
                    )
                    .map_err(|error| WireError::InvalidBuffer(error.to_string()))?
                    .into(),
                ),
                other => {
                    return Err(WireError::InvalidBuffer(format!(
                        "unsupported attribute content type {other}"
                    )));
                }
            };
            CoreCommand::SetAttribute {
                node: node(value.current())?,
                name: value
                    .attr_name()
                    .ok_or_else(|| WireError::InvalidBuffer("missing attribute name".into()))?
                    .into(),
                value: attribute,
            }
        }
        fb::ElementCommand::AddEventListenerCommand => {
            let value = command
                .operation_as_add_event_listener_command()
                .ok_or_else(|| {
                    WireError::InvalidBuffer("missing AddEventListener payload".into())
                })?;
            CoreCommand::AddEventListener {
                node: node(value.node())?,
                listener: listener(command.listener_id())?,
                callback: callback(value.callback())?,
                name: value
                    .name()
                    .ok_or_else(|| WireError::InvalidBuffer("missing listener name".into()))?
                    .into(),
            }
        }
        fb::ElementCommand::RemoveEventListenerCommand => {
            let value = command
                .operation_as_remove_event_listener_command()
                .ok_or_else(|| {
                    WireError::InvalidBuffer("missing RemoveEventListener payload".into())
                })?;
            CoreCommand::RemoveEventListener {
                node: node(value.node())?,
                listener: listener(command.listener_id())?,
                callback: callback(value.callback())?,
                name: value
                    .name()
                    .ok_or_else(|| WireError::InvalidBuffer("missing listener name".into()))?
                    .into(),
            }
        }
        fb::ElementCommand::GetTagCommand => {
            let value = command
                .operation_as_get_tag_command()
                .ok_or_else(|| WireError::InvalidBuffer("missing GetTag payload".into()))?;
            CoreCommand::GetTag {
                node: node(value.node())?,
            }
        }
        operation => {
            return Err(WireError::UnsupportedCommand(format!("{operation:?}")));
        }
    };
    Ok(CoreCommandItem {
        result_slot: (command.result_slot() != NO_RESULT_SLOT)
            .then(|| ResultSlot::new(command.result_slot())),
        command: operation,
    })
}

fn encode_command<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    item: &CoreCommandItem,
) -> Result<WIPOffset<fb::Command<'a>>, WireError> {
    let result_slot = item.result_slot.map_or(NO_RESULT_SLOT, ResultSlot::get);
    let mut result_node_id = 0;
    let mut listener_id = 0;
    let (operation_type, operation) = match &item.command {
        CoreCommand::CreateElement { node, tag } => {
            result_node_id = node.get();
            let tag = builder.create_string(tag);
            let operation = fb::CreateElementCommand::create(
                builder,
                &fb::CreateElementCommandArgs {
                    tag: Some(tag),
                    com_parent_uni_id: 0,
                    info: None,
                },
            );
            (
                fb::ElementCommand::CreateElementCommand,
                operation.as_union_value(),
            )
        }
        CoreCommand::CreateRawText { node, text } => {
            result_node_id = node.get();
            let text = builder.create_string(text);
            let operation = fb::CreateRawTextCommand::create(
                builder,
                &fb::CreateRawTextCommandArgs {
                    text: Some(text),
                    info: None,
                },
            );
            (
                fb::ElementCommand::CreateRawTextCommand,
                operation.as_union_value(),
            )
        }
        CoreCommand::AppendElement { parent, child } => {
            let operation = fb::AppendElementCommand::create(
                builder,
                &fb::AppendElementCommandArgs {
                    parent: parent.get(),
                    current: child.get(),
                },
            );
            (
                fb::ElementCommand::AppendElementCommand,
                operation.as_union_value(),
            )
        }
        CoreCommand::InsertElementBefore {
            parent,
            child,
            reference,
        } => {
            let operation = fb::InsertElementBeforeCommand::create(
                builder,
                &fb::InsertElementBeforeCommandArgs {
                    parent: parent.get(),
                    current: child.get(),
                    marker: Some(reference.get()),
                },
            );
            (
                fb::ElementCommand::InsertElementBeforeCommand,
                operation.as_union_value(),
            )
        }
        CoreCommand::RemoveElement { parent, child } => {
            let operation = fb::RemoveElementCommand::create(
                builder,
                &fb::RemoveElementCommandArgs {
                    parent: parent.get(),
                    current: child.get(),
                },
            );
            (
                fb::ElementCommand::RemoveElementCommand,
                operation.as_union_value(),
            )
        }
        CoreCommand::DestroyNode { node } => {
            let operation = fb::ReleaseElementCommand::create(
                builder,
                &fb::ReleaseElementCommandArgs { node: node.get() },
            );
            (
                fb::ElementCommand::ReleaseElementCommand,
                operation.as_union_value(),
            )
        }
        CoreCommand::SetAttribute { node, name, value } => {
            let name = builder.create_string(name);
            let content_type = builder.create_string(if value.is_some() {
                "text/plain;charset=utf-8"
            } else {
                "application/vnd.lynx-element-bridge.null"
            });
            let bytes = value
                .as_ref()
                .map(|value| builder.create_vector(value.as_bytes()));
            let value = fb::Payload::create(
                builder,
                &fb::PayloadArgs {
                    content_type: Some(content_type),
                    bytes,
                },
            );
            let operation = fb::SetAttributeCommand::create(
                builder,
                &fb::SetAttributeCommandArgs {
                    current: node.get(),
                    attr_name: Some(name),
                    value: Some(value),
                },
            );
            (
                fb::ElementCommand::SetAttributeCommand,
                operation.as_union_value(),
            )
        }
        CoreCommand::AddEventListener {
            node,
            listener,
            callback,
            name,
        } => {
            listener_id = listener.get();
            let name = builder.create_string(name);
            let content_type = builder.create_string("application/json");
            let bytes = builder.create_vector(b"{}".as_slice());
            let options = fb::Payload::create(
                builder,
                &fb::PayloadArgs {
                    content_type: Some(content_type),
                    bytes: Some(bytes),
                },
            );
            let operation = fb::AddEventListenerCommand::create(
                builder,
                &fb::AddEventListenerCommandArgs {
                    node: node.get(),
                    name: Some(name),
                    callback: callback.get(),
                    options: Some(options),
                },
            );
            (
                fb::ElementCommand::AddEventListenerCommand,
                operation.as_union_value(),
            )
        }
        CoreCommand::RemoveEventListener {
            node,
            listener,
            callback,
            name,
        } => {
            listener_id = listener.get();
            let name = builder.create_string(name);
            let content_type = builder.create_string("application/json");
            let bytes = builder.create_vector(b"{}".as_slice());
            let options = fb::Payload::create(
                builder,
                &fb::PayloadArgs {
                    content_type: Some(content_type),
                    bytes: Some(bytes),
                },
            );
            let operation = fb::RemoveEventListenerCommand::create(
                builder,
                &fb::RemoveEventListenerCommandArgs {
                    node: node.get(),
                    name: Some(name),
                    callback: callback.get(),
                    options: Some(options),
                },
            );
            (
                fb::ElementCommand::RemoveEventListenerCommand,
                operation.as_union_value(),
            )
        }
        CoreCommand::GetTag { node } => {
            let operation =
                fb::GetTagCommand::create(builder, &fb::GetTagCommandArgs { node: node.get() });
            (
                fb::ElementCommand::GetTagCommand,
                operation.as_union_value(),
            )
        }
        CoreCommand::InvokeCapability { capability } => {
            return Err(WireError::UnsupportedCommand(capability.clone()));
        }
    };

    Ok(fb::Command::create(
        builder,
        &fb::CommandArgs {
            result_slot,
            result_node_id,
            result_node_ids: None,
            listener_id,
            operation_type,
            operation: Some(operation),
        },
    ))
}

fn encode_result<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    result: &CoreCommandResult,
) -> WIPOffset<fb::ResultItem<'a>> {
    let message = result
        .message
        .as_ref()
        .map(|message| builder.create_string(message));
    let (kind, value_type, value) = match &result.value {
        None => (fb::ResultValueKind::NONE, fb::ResultValue::NONE, None),
        Some(CoreResultValue::Element(node)) => {
            let value = fb::ElementIdResult::create(
                builder,
                &fb::ElementIdResultArgs { value: node.get() },
            );
            (
                fb::ResultValueKind::ELEMENT_ID,
                fb::ResultValue::ElementIdResult,
                Some(value.as_union_value()),
            )
        }
        Some(CoreResultValue::Elements(nodes)) => {
            let nodes = nodes.iter().map(|node| node.get()).collect::<Vec<_>>();
            let nodes = builder.create_vector(&nodes);
            let value = fb::ElementIdsResult::create(
                builder,
                &fb::ElementIdsResultArgs {
                    values: Some(nodes),
                },
            );
            (
                fb::ResultValueKind::ELEMENT_IDS,
                fb::ResultValue::ElementIdsResult,
                Some(value.as_union_value()),
            )
        }
        Some(CoreResultValue::String(string)) => {
            let string = builder.create_string(string);
            let value = fb::StringResult::create(
                builder,
                &fb::StringResultArgs {
                    value: Some(string),
                },
            );
            (
                fb::ResultValueKind::STRING,
                fb::ResultValue::StringResult,
                Some(value.as_union_value()),
            )
        }
        Some(CoreResultValue::Strings(strings)) => {
            let strings = strings
                .iter()
                .map(|string| builder.create_string(string))
                .collect::<Vec<_>>();
            let strings = builder.create_vector(&strings);
            let value = fb::StringsResult::create(
                builder,
                &fb::StringsResultArgs {
                    values: Some(strings),
                },
            );
            (
                fb::ResultValueKind::STRINGS,
                fb::ResultValue::StringsResult,
                Some(value.as_union_value()),
            )
        }
        Some(CoreResultValue::Boolean(boolean)) => {
            let value =
                fb::BooleanResult::create(builder, &fb::BooleanResultArgs { value: *boolean });
            (
                fb::ResultValueKind::BOOLEAN,
                fb::ResultValue::BooleanResult,
                Some(value.as_union_value()),
            )
        }
        Some(CoreResultValue::Number(number)) => {
            let value = fb::NumberResult::create(builder, &fb::NumberResultArgs { value: *number });
            (
                fb::ResultValueKind::NUMBER,
                fb::ResultValue::NumberResult,
                Some(value.as_union_value()),
            )
        }
        Some(CoreResultValue::Payload {
            content_type,
            bytes,
        }) => {
            let content_type = builder.create_string(content_type);
            let bytes = builder.create_vector(bytes);
            let value = fb::Payload::create(
                builder,
                &fb::PayloadArgs {
                    content_type: Some(content_type),
                    bytes: Some(bytes),
                },
            );
            (
                fb::ResultValueKind::PAYLOAD,
                fb::ResultValue::Payload,
                Some(value.as_union_value()),
            )
        }
    };
    fb::ResultItem::create(
        builder,
        &fb::ResultItemArgs {
            slot: result.slot.map_or(NO_RESULT_SLOT, ResultSlot::get),
            status: encode_status(result.status),
            message,
            value_kind: kind,
            value_type,
            value,
        },
    )
}

fn decode_result_value(result: fb::ResultItem<'_>) -> Result<Option<CoreResultValue>, WireError> {
    let value = match result.value_kind() {
        fb::ResultValueKind::NONE => None,
        fb::ResultValueKind::ELEMENT_ID => Some(CoreResultValue::Element(
            NodeId::new(
                result
                    .value_as_element_id_result()
                    .ok_or(WireError::ChannelMismatch)?
                    .value(),
            )
            .map_err(|_| WireError::InvalidId("node"))?,
        )),
        fb::ResultValueKind::ELEMENT_IDS => {
            let values = result
                .value_as_element_ids_result()
                .ok_or(WireError::ChannelMismatch)?
                .values()
                .map(|values| {
                    values
                        .iter()
                        .map(|value| NodeId::new(value).map_err(|_| WireError::InvalidId("node")))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?
                .unwrap_or_default();
            Some(CoreResultValue::Elements(values))
        }
        fb::ResultValueKind::STRING => Some(CoreResultValue::String(
            result
                .value_as_string_result()
                .and_then(|value| value.value())
                .ok_or(WireError::ChannelMismatch)?
                .into(),
        )),
        fb::ResultValueKind::STRINGS => {
            let values = result
                .value_as_strings_result()
                .and_then(|value| value.values())
                .map(|values| values.iter().map(Into::into).collect())
                .unwrap_or_default();
            Some(CoreResultValue::Strings(values))
        }
        fb::ResultValueKind::BOOLEAN => Some(CoreResultValue::Boolean(
            result
                .value_as_boolean_result()
                .ok_or(WireError::ChannelMismatch)?
                .value(),
        )),
        fb::ResultValueKind::NUMBER => Some(CoreResultValue::Number(
            result
                .value_as_number_result()
                .ok_or(WireError::ChannelMismatch)?
                .value(),
        )),
        fb::ResultValueKind::PAYLOAD => {
            let value = result
                .value_as_payload()
                .ok_or(WireError::ChannelMismatch)?;
            Some(CoreResultValue::Payload {
                content_type: value.content_type().into(),
                bytes: value
                    .bytes()
                    .map(|bytes| bytes.iter().collect())
                    .unwrap_or_default(),
            })
        }
        _ => return Err(WireError::ChannelMismatch),
    };
    Ok(value)
}

fn finish_envelope(
    builder: &mut FlatBufferBuilder<'_>,
    channel: fb::Channel,
    message_type: fb::Message,
    message: WIPOffset<flatbuffers::UnionWIPOffset>,
) {
    let envelope = fb::Envelope::create(
        builder,
        &fb::EnvelopeArgs {
            version: PROTOCOL_VERSION,
            channel,
            message_type,
            message: Some(message),
        },
    );
    fb::finish_envelope_buffer(builder, envelope);
}

fn verified_envelope(bytes: &[u8]) -> Result<fb::Envelope<'_>, WireError> {
    if !fb::envelope_buffer_has_identifier(bytes) {
        return Err(WireError::InvalidBuffer("missing LEB2 identifier".into()));
    }
    let envelope =
        fb::root_as_envelope(bytes).map_err(|error| WireError::InvalidBuffer(error.to_string()))?;
    if envelope.version() != PROTOCOL_VERSION {
        return Err(WireError::UnsupportedVersion(envelope.version()));
    }
    Ok(envelope)
}

fn encode_status(status: CoreStatus) -> fb::Status {
    match status {
        CoreStatus::Ok => fb::Status::OK,
        CoreStatus::InvalidArgument => fb::Status::INVALID_ARGUMENT,
        CoreStatus::InvalidSession => fb::Status::INVALID_SESSION,
        CoreStatus::WrongThread => fb::Status::WRONG_THREAD,
        CoreStatus::Unsupported => fb::Status::UNSUPPORTED,
        CoreStatus::InvalidOwnership => fb::Status::INVALID_OWNERSHIP,
        CoreStatus::InvalidListener => fb::Status::INVALID_LISTENER,
        CoreStatus::ResourceExhausted => fb::Status::RESOURCE_EXHAUSTED,
        CoreStatus::HostError => fb::Status::HOST_ERROR,
        CoreStatus::Panic => fb::Status::PANIC,
        CoreStatus::InternalError => fb::Status::INTERNAL_ERROR,
    }
}

fn decode_status(status: fb::Status) -> Result<CoreStatus, WireError> {
    match status {
        fb::Status::OK => Ok(CoreStatus::Ok),
        fb::Status::INVALID_ARGUMENT => Ok(CoreStatus::InvalidArgument),
        fb::Status::INVALID_SESSION => Ok(CoreStatus::InvalidSession),
        fb::Status::WRONG_THREAD => Ok(CoreStatus::WrongThread),
        fb::Status::UNSUPPORTED => Ok(CoreStatus::Unsupported),
        fb::Status::INVALID_OWNERSHIP => Ok(CoreStatus::InvalidOwnership),
        fb::Status::INVALID_LISTENER => Ok(CoreStatus::InvalidListener),
        fb::Status::RESOURCE_EXHAUSTED => Ok(CoreStatus::ResourceExhausted),
        fb::Status::HOST_ERROR => Ok(CoreStatus::HostError),
        fb::Status::PANIC => Ok(CoreStatus::Panic),
        fb::Status::INTERNAL_ERROR => Ok(CoreStatus::InternalError),
        _ => Err(WireError::InvalidBuffer("unknown status".into())),
    }
}

impl From<BridgeError> for WireError {
    fn from(error: BridgeError) -> Self {
        Self::InvalidBuffer(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use lynx_element_bridge_core::{
        CallbackId, CapabilityRequest, CommandResult, EventMessage, HostFake, ListenerId, Session,
    };

    use super::*;

    fn capabilities() -> Vec<CapabilityRequest> {
        [
            "create_element",
            "create_raw_text",
            "append_element",
            "remove_element",
            "set_attribute",
            "add_event_listener",
            "remove_event_listener",
            "get_tag",
        ]
        .into_iter()
        .map(CapabilityRequest::required)
        .collect()
    }

    #[test]
    fn command_batches_use_the_v2_identifier_and_typed_operations() {
        let session_id = SessionId::new(1).unwrap();
        let root = NodeId::new(1).unwrap();
        let (mut session, _) = Session::create(session_id, root, &capabilities()).unwrap();
        let text = session.create_element("text").unwrap();
        let raw = session.create_text("A\0B").unwrap();
        session.insert_before(root, text, None).unwrap();
        session.insert_before(text, raw, None).unwrap();
        session.query_tag(text, ResultSlot::new(0)).unwrap();

        let bytes = encode_command_batch(&session.take_batch().unwrap()).unwrap();
        assert_eq!(&bytes[4..8], b"LEB2");
        verify(&bytes).unwrap();
        let envelope = verified_envelope(&bytes).unwrap();
        assert_eq!(envelope.channel(), fb::Channel::COMMAND);
        let batch = envelope.message_as_command_batch().unwrap();
        assert_eq!(batch.commands().unwrap().len(), 5);
        assert_eq!(
            batch.commands().unwrap().get(0).operation_type(),
            fb::ElementCommand::CreateElementCommand
        );
        assert_eq!(
            batch.commands().unwrap().get(0).result_node_id(),
            text.get()
        );
        assert_eq!(decode_command_batch(&bytes).unwrap().commands.len(), 5);
    }

    #[test]
    fn result_and_event_channels_round_trip_binary_values() {
        let session = SessionId::new(9).unwrap();
        let response = CoreResponse {
            session: Some(session),
            sequence: 4,
            status: CoreStatus::Ok,
            message: None,
            results: vec![CommandResult {
                slot: Some(ResultSlot::new(2)),
                status: CoreStatus::Ok,
                message: None,
                value: Some(CoreResultValue::Payload {
                    content_type: "application/octet-stream".into(),
                    bytes: vec![0, 255, 1],
                }),
            }],
            committed: true,
        };
        assert_eq!(
            decode_response(&encode_response(&response).unwrap()).unwrap(),
            response
        );

        let event = EventMessage {
            session,
            listener: ListenerId::new(3).unwrap(),
            callback: CallbackId::new(5).unwrap(),
            content_type: "application/octet-stream".into(),
            payload: vec![0, 255, 1],
        };
        assert_eq!(decode_event(&encode_event(&event).unwrap()).unwrap(), event);

        let failure = decode_response(&encode_failure(
            0,
            0,
            CoreStatus::InvalidArgument,
            "invalid argument",
        ))
        .unwrap();
        assert_eq!(failure.session, None);
        assert_eq!(failure.status, CoreStatus::InvalidArgument);
    }

    #[test]
    fn host_fake_response_is_byte_identical_after_wire_round_trip() {
        let session_id = SessionId::new(2).unwrap();
        let root = NodeId::new(1).unwrap();
        let (mut session, _) = Session::create(session_id, root, &capabilities()).unwrap();
        let node = session.create_element("view").unwrap();
        session.insert_before(root, node, None).unwrap();
        let batch = session.take_batch().unwrap();
        let response = HostFake::new(session_id, root).apply(&batch);
        let decoded = decode_response(&encode_response(&response).unwrap()).unwrap();
        assert_eq!(decoded, response);
    }

    #[test]
    fn verifier_rejects_text_and_truncated_buffers() {
        assert!(verify(b"not flatbuffers").is_err());
        let request = encode_create_session(
            NodeId::new(1).unwrap(),
            &[CapabilityRequest::required("create_element")],
        )
        .unwrap();
        assert!(verify(&request[..request.len() / 2]).is_err());
    }
}
