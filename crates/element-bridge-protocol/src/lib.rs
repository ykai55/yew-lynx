use flatbuffers::{FlatBufferBuilder, WIPOffset};
use lynx_element_bridge_core::{
    BridgeError, CallbackId, Command, CommandBatch, EventMessage, ListenerId, NodeId, Status,
};

#[allow(unsafe_code, clippy::all, warnings)]
mod generated {
    include!("guest_protocol_generated.rs");
}

use generated::lynx::element_bridge::protocol as fb;

pub const PROTOCOL_VERSION: u32 = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MountRequest {
    pub protocol_version: u32,
    pub root: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventRequest {
    pub protocol_version: u32,
    pub event: EventMessage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestResponse {
    pub protocol_version: u32,
    pub result: GuestResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuestResult {
    Ok(CommandBatch),
    Err { status: Status, message: String },
}

impl GuestResponse {
    pub fn from_result(result: Result<CommandBatch, BridgeError>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            result: match result {
                Ok(batch) => GuestResult::Ok(batch),
                Err(error) => GuestResult::Err {
                    status: error.status,
                    message: error.message,
                },
            },
        }
    }
}

pub fn encode_mount_request(request: &MountRequest) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let message = fb::MountRequest::create(
        &mut builder,
        &fb::MountRequestArgs {
            root: request.root.get(),
        },
    );
    finish_envelope(
        builder,
        request.protocol_version,
        fb::Message::MountRequest,
        message.as_union_value(),
    )
}

pub fn decode_mount_request(bytes: &[u8]) -> Result<MountRequest, BridgeError> {
    let envelope = decode_envelope(bytes, fb::Message::MountRequest)?;
    let request = envelope
        .message_as_mount_request()
        .ok_or_else(|| invalid("mount request is missing its union payload"))?;
    Ok(MountRequest {
        protocol_version: envelope.protocol_version(),
        root: NodeId::new(request.root())?,
    })
}

pub fn encode_event_request(request: &EventRequest) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let content_type = builder.create_string(&request.event.content_type);
    let payload = builder.create_vector(&request.event.payload);
    let message = fb::EventRequest::create(
        &mut builder,
        &fb::EventRequestArgs {
            listener: request.event.listener.get(),
            callback: request.event.callback.get(),
            content_type: Some(content_type),
            payload: Some(payload),
        },
    );
    finish_envelope(
        builder,
        request.protocol_version,
        fb::Message::EventRequest,
        message.as_union_value(),
    )
}

pub fn decode_event_request(bytes: &[u8]) -> Result<EventRequest, BridgeError> {
    let envelope = decode_envelope(bytes, fb::Message::EventRequest)?;
    let request = envelope
        .message_as_event_request()
        .ok_or_else(|| invalid("event request is missing its union payload"))?;
    Ok(EventRequest {
        protocol_version: envelope.protocol_version(),
        event: EventMessage {
            listener: ListenerId::new(request.listener())?,
            callback: CallbackId::new(request.callback())?,
            content_type: request.content_type().into(),
            payload: request.payload().bytes().to_vec(),
        },
    })
}

pub fn encode_guest_response(response: &GuestResponse) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    let (result_type, result) = match &response.result {
        GuestResult::Ok(batch) => {
            let batch = encode_batch(&mut builder, batch);
            let success =
                fb::Success::create(&mut builder, &fb::SuccessArgs { batch: Some(batch) });
            (fb::GuestResult::Success, success.as_union_value())
        }
        GuestResult::Err { status, message } => {
            let message = builder.create_string(message);
            let failure = fb::Failure::create(
                &mut builder,
                &fb::FailureArgs {
                    status: encode_status(*status),
                    message: Some(message),
                },
            );
            (fb::GuestResult::Failure, failure.as_union_value())
        }
    };
    let response_message = fb::GuestResponse::create(
        &mut builder,
        &fb::GuestResponseArgs {
            result_type,
            result: Some(result),
        },
    );
    finish_envelope(
        builder,
        response.protocol_version,
        fb::Message::GuestResponse,
        response_message.as_union_value(),
    )
}

pub fn decode_guest_response(bytes: &[u8]) -> Result<GuestResponse, BridgeError> {
    let envelope = decode_envelope(bytes, fb::Message::GuestResponse)?;
    let response = envelope
        .message_as_guest_response()
        .ok_or_else(|| invalid("guest response is missing its union payload"))?;
    let result = match response.result_type() {
        fb::GuestResult::Success => {
            let success = response
                .result_as_success()
                .ok_or_else(|| invalid("success result is missing its union payload"))?;
            GuestResult::Ok(decode_batch(success.batch())?)
        }
        fb::GuestResult::Failure => {
            let failure = response
                .result_as_failure()
                .ok_or_else(|| invalid("failure result is missing its union payload"))?;
            let status = decode_status(failure.status())?;
            if status == Status::Ok {
                return Err(invalid("guest error response used an OK status"));
            }
            GuestResult::Err {
                status,
                message: failure.message().into(),
            }
        }
        kind if kind == fb::GuestResult::NONE => {
            return Err(invalid("guest response has no result kind"));
        }
        kind => return Err(invalid(format!("unknown guest result kind {}", kind.0))),
    };
    Ok(GuestResponse {
        protocol_version: envelope.protocol_version(),
        result,
    })
}

fn finish_envelope(
    mut builder: FlatBufferBuilder<'static>,
    protocol_version: u32,
    message_type: fb::Message,
    message: WIPOffset<flatbuffers::UnionWIPOffset>,
) -> Vec<u8> {
    let envelope = fb::Envelope::create(
        &mut builder,
        &fb::EnvelopeArgs {
            protocol_version,
            message_type,
            message: Some(message),
        },
    );
    fb::finish_envelope_buffer(&mut builder, envelope);
    builder.finished_data().to_vec()
}

fn decode_envelope(bytes: &[u8], expected: fb::Message) -> Result<fb::Envelope<'_>, BridgeError> {
    if bytes.len() < flatbuffers::SIZE_UOFFSET + flatbuffers::FILE_IDENTIFIER_LENGTH
        || !fb::envelope_buffer_has_identifier(bytes)
    {
        return Err(invalid(
            "invalid FlatBuffers file identifier; expected LEB4",
        ));
    }
    let envelope = fb::root_as_envelope(bytes)
        .map_err(|error| invalid(format!("invalid FlatBuffers message: {error}")))?;
    if envelope.protocol_version() != PROTOCOL_VERSION {
        return Err(BridgeError::new(
            Status::Unsupported,
            format!(
                "unsupported protocol version {}",
                envelope.protocol_version()
            ),
        ));
    }
    let actual = envelope.message_type();
    if actual.variant_name().is_none() {
        return Err(invalid(format!(
            "unknown envelope message kind {}",
            actual.0
        )));
    }
    if actual != expected {
        return Err(invalid(format!(
            "unexpected envelope message kind {}; expected {}",
            actual.variant_name().unwrap_or("unknown"),
            expected.variant_name().unwrap_or("unknown")
        )));
    }
    if envelope.message().is_none() {
        return Err(invalid("envelope is missing its union payload"));
    }
    Ok(envelope)
}

fn encode_batch<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    batch: &CommandBatch,
) -> WIPOffset<fb::CommandBatch<'a>> {
    let commands = batch
        .commands
        .iter()
        .map(|command| encode_command(builder, command))
        .collect::<Vec<_>>();
    let commands = builder.create_vector(&commands);
    fb::CommandBatch::create(
        builder,
        &fb::CommandBatchArgs {
            sequence: batch.sequence,
            commands: Some(commands),
            final_commit: batch.final_commit,
        },
    )
}

fn encode_command<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    command: &Command,
) -> WIPOffset<fb::Command<'a>> {
    let (payload_type, payload) = match command {
        Command::ImportStyleSheet { fragment } => {
            let fragment = builder.create_vector(fragment);
            let value = fb::ImportStyleSheet::create(
                builder,
                &fb::ImportStyleSheetArgs {
                    fragment: Some(fragment),
                },
            );
            (fb::CommandPayload::ImportStyleSheet, value.as_union_value())
        }
        Command::CreateElement { node, tag } => {
            let tag = builder.create_string(tag);
            let value = fb::CreateElement::create(
                builder,
                &fb::CreateElementArgs {
                    node: node.get(),
                    tag: Some(tag),
                },
            );
            (fb::CommandPayload::CreateElement, value.as_union_value())
        }
        Command::CreateRawText { node, text } => {
            let text = builder.create_string(text);
            let value = fb::CreateRawText::create(
                builder,
                &fb::CreateRawTextArgs {
                    node: node.get(),
                    text: Some(text),
                },
            );
            (fb::CommandPayload::CreateRawText, value.as_union_value())
        }
        Command::AppendElement { parent, child } => {
            let value = fb::AppendElement::create(
                builder,
                &fb::AppendElementArgs {
                    parent: parent.get(),
                    child: child.get(),
                },
            );
            (fb::CommandPayload::AppendElement, value.as_union_value())
        }
        Command::InsertElementBefore {
            parent,
            child,
            reference,
        } => {
            let value = fb::InsertElementBefore::create(
                builder,
                &fb::InsertElementBeforeArgs {
                    parent: parent.get(),
                    child: child.get(),
                    reference: reference.get(),
                },
            );
            (
                fb::CommandPayload::InsertElementBefore,
                value.as_union_value(),
            )
        }
        Command::RemoveElement { parent, child } => {
            let value = fb::RemoveElement::create(
                builder,
                &fb::RemoveElementArgs {
                    parent: parent.get(),
                    child: child.get(),
                },
            );
            (fb::CommandPayload::RemoveElement, value.as_union_value())
        }
        Command::DestroyNode { node } => {
            let value = fb::DestroyNode::create(builder, &fb::DestroyNodeArgs { node: node.get() });
            (fb::CommandPayload::DestroyNode, value.as_union_value())
        }
        Command::SetAttribute { node, name, value } => {
            let name = builder.create_string(name);
            let value = value.as_ref().map(|value| builder.create_string(value));
            let value = fb::SetAttribute::create(
                builder,
                &fb::SetAttributeArgs {
                    node: node.get(),
                    name: Some(name),
                    value,
                },
            );
            (fb::CommandPayload::SetAttribute, value.as_union_value())
        }
        Command::AddEventListener {
            node,
            listener,
            callback,
            name,
        } => {
            let name = builder.create_string(name);
            let value = fb::AddEventListener::create(
                builder,
                &fb::AddEventListenerArgs {
                    node: node.get(),
                    listener: listener.get(),
                    callback: callback.get(),
                    name: Some(name),
                },
            );
            (fb::CommandPayload::AddEventListener, value.as_union_value())
        }
        Command::RemoveEventListener {
            node,
            listener,
            callback,
            name,
        } => {
            let name = builder.create_string(name);
            let value = fb::RemoveEventListener::create(
                builder,
                &fb::RemoveEventListenerArgs {
                    node: node.get(),
                    listener: listener.get(),
                    callback: callback.get(),
                    name: Some(name),
                },
            );
            (
                fb::CommandPayload::RemoveEventListener,
                value.as_union_value(),
            )
        }
    };
    fb::Command::create(
        builder,
        &fb::CommandArgs {
            payload_type,
            payload: Some(payload),
        },
    )
}

fn decode_batch(batch: fb::CommandBatch<'_>) -> Result<CommandBatch, BridgeError> {
    let commands = batch
        .commands()
        .iter()
        .map(decode_command)
        .collect::<Result<_, _>>()?;
    Ok(CommandBatch {
        sequence: batch.sequence(),
        commands,
        final_commit: batch.final_commit(),
    })
}

fn decode_command(command: fb::Command<'_>) -> Result<Command, BridgeError> {
    macro_rules! payload {
        ($accessor:ident, $name:literal) => {
            command
                .$accessor()
                .ok_or_else(|| invalid(concat!($name, " command is missing its union payload")))?
        };
    }
    Ok(match command.payload_type() {
        fb::CommandPayload::ImportStyleSheet => {
            let value = payload!(payload_as_import_style_sheet, "ImportStyleSheet");
            Command::ImportStyleSheet {
                fragment: value.fragment().bytes().to_vec(),
            }
        }
        fb::CommandPayload::CreateElement => {
            let value = payload!(payload_as_create_element, "CreateElement");
            Command::CreateElement {
                node: NodeId::new(value.node())?,
                tag: value.tag().into(),
            }
        }
        fb::CommandPayload::CreateRawText => {
            let value = payload!(payload_as_create_raw_text, "CreateRawText");
            Command::CreateRawText {
                node: NodeId::new(value.node())?,
                text: value.text().into(),
            }
        }
        fb::CommandPayload::AppendElement => {
            let value = payload!(payload_as_append_element, "AppendElement");
            Command::AppendElement {
                parent: NodeId::new(value.parent())?,
                child: NodeId::new(value.child())?,
            }
        }
        fb::CommandPayload::InsertElementBefore => {
            let value = payload!(payload_as_insert_element_before, "InsertElementBefore");
            Command::InsertElementBefore {
                parent: NodeId::new(value.parent())?,
                child: NodeId::new(value.child())?,
                reference: NodeId::new(value.reference())?,
            }
        }
        fb::CommandPayload::RemoveElement => {
            let value = payload!(payload_as_remove_element, "RemoveElement");
            Command::RemoveElement {
                parent: NodeId::new(value.parent())?,
                child: NodeId::new(value.child())?,
            }
        }
        fb::CommandPayload::DestroyNode => {
            let value = payload!(payload_as_destroy_node, "DestroyNode");
            Command::DestroyNode {
                node: NodeId::new(value.node())?,
            }
        }
        fb::CommandPayload::SetAttribute => {
            let value = payload!(payload_as_set_attribute, "SetAttribute");
            Command::SetAttribute {
                node: NodeId::new(value.node())?,
                name: value.name().into(),
                value: value.value().map(Into::into),
            }
        }
        fb::CommandPayload::AddEventListener => {
            let value = payload!(payload_as_add_event_listener, "AddEventListener");
            Command::AddEventListener {
                node: NodeId::new(value.node())?,
                listener: ListenerId::new(value.listener())?,
                callback: CallbackId::new(value.callback())?,
                name: value.name().into(),
            }
        }
        fb::CommandPayload::RemoveEventListener => {
            let value = payload!(payload_as_remove_event_listener, "RemoveEventListener");
            Command::RemoveEventListener {
                node: NodeId::new(value.node())?,
                listener: ListenerId::new(value.listener())?,
                callback: CallbackId::new(value.callback())?,
                name: value.name().into(),
            }
        }
        kind if kind == fb::CommandPayload::NONE => {
            return Err(invalid("command has no payload kind"));
        }
        kind => return Err(invalid(format!("unknown command payload kind {}", kind.0))),
    })
}

fn encode_status(status: Status) -> fb::Status {
    match status {
        Status::Ok => fb::Status::Ok,
        Status::InvalidArgument => fb::Status::InvalidArgument,
        Status::InvalidSession => fb::Status::InvalidSession,
        Status::WrongThread => fb::Status::WrongThread,
        Status::Unsupported => fb::Status::Unsupported,
        Status::InvalidOwnership => fb::Status::InvalidOwnership,
        Status::InvalidListener => fb::Status::InvalidListener,
        Status::ResourceExhausted => fb::Status::ResourceExhausted,
        Status::HostError => fb::Status::HostError,
        Status::Panic => fb::Status::Panic,
        Status::InternalError => fb::Status::InternalError,
    }
}

fn decode_status(status: fb::Status) -> Result<Status, BridgeError> {
    match status {
        fb::Status::Ok => Ok(Status::Ok),
        fb::Status::InvalidArgument => Ok(Status::InvalidArgument),
        fb::Status::InvalidSession => Ok(Status::InvalidSession),
        fb::Status::WrongThread => Ok(Status::WrongThread),
        fb::Status::Unsupported => Ok(Status::Unsupported),
        fb::Status::InvalidOwnership => Ok(Status::InvalidOwnership),
        fb::Status::InvalidListener => Ok(Status::InvalidListener),
        fb::Status::ResourceExhausted => Ok(Status::ResourceExhausted),
        fb::Status::HostError => Ok(Status::HostError),
        fb::Status::Panic => Ok(Status::Panic),
        fb::Status::InternalError => Ok(Status::InternalError),
        value => Err(invalid(format!("unknown status value {}", value.0))),
    }
}

fn invalid(message: impl Into<String>) -> BridgeError {
    BridgeError::new(Status::InvalidArgument, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_and_unknown_envelope_message_unions() {
        let mut builder = FlatBufferBuilder::new();
        let envelope = fb::Envelope::create(
            &mut builder,
            &fb::EnvelopeArgs {
                protocol_version: PROTOCOL_VERSION,
                message_type: fb::Message::MountRequest,
                message: None,
            },
        );
        fb::finish_envelope_buffer(&mut builder, envelope);
        assert_eq!(
            decode_mount_request(builder.finished_data())
                .unwrap_err()
                .status,
            Status::InvalidArgument
        );

        let mut builder = FlatBufferBuilder::new();
        let mount = fb::MountRequest::create(&mut builder, &fb::MountRequestArgs { root: 1 });
        let envelope = fb::Envelope::create(
            &mut builder,
            &fb::EnvelopeArgs {
                protocol_version: PROTOCOL_VERSION,
                message_type: fb::Message(99),
                message: Some(mount.as_union_value()),
            },
        );
        fb::finish_envelope_buffer(&mut builder, envelope);
        let error = decode_mount_request(builder.finished_data()).unwrap_err();
        assert_eq!(error.status, Status::InvalidArgument);
        assert!(error.message.contains("unknown"), "{}", error.message);
    }

    #[test]
    fn rejects_missing_and_unknown_result_unions_and_unknown_status() {
        for result_type in [fb::GuestResult::Success, fb::GuestResult(99)] {
            let mut builder = FlatBufferBuilder::new();
            let response = fb::GuestResponse::create(
                &mut builder,
                &fb::GuestResponseArgs {
                    result_type,
                    result: None,
                },
            );
            let bytes = finish_test_envelope(builder, response.as_union_value());
            assert_eq!(
                decode_guest_response(&bytes).unwrap_err().status,
                Status::InvalidArgument
            );
        }

        let mut builder = FlatBufferBuilder::new();
        let message = builder.create_string("unknown status");
        let failure = fb::Failure::create(
            &mut builder,
            &fb::FailureArgs {
                status: fb::Status(99),
                message: Some(message),
            },
        );
        let response = fb::GuestResponse::create(
            &mut builder,
            &fb::GuestResponseArgs {
                result_type: fb::GuestResult::Failure,
                result: Some(failure.as_union_value()),
            },
        );
        let bytes = finish_test_envelope(builder, response.as_union_value());
        assert_eq!(
            decode_guest_response(&bytes).unwrap_err().status,
            Status::InvalidArgument
        );
    }

    #[test]
    fn rejects_missing_and_unknown_command_unions() {
        for payload_type in [fb::CommandPayload::CreateElement, fb::CommandPayload(99)] {
            let mut builder = FlatBufferBuilder::new();
            let command = fb::Command::create(
                &mut builder,
                &fb::CommandArgs {
                    payload_type,
                    payload: None,
                },
            );
            let commands = builder.create_vector(&[command]);
            let batch = fb::CommandBatch::create(
                &mut builder,
                &fb::CommandBatchArgs {
                    sequence: 1,
                    commands: Some(commands),
                    final_commit: true,
                },
            );
            let success =
                fb::Success::create(&mut builder, &fb::SuccessArgs { batch: Some(batch) });
            let response = fb::GuestResponse::create(
                &mut builder,
                &fb::GuestResponseArgs {
                    result_type: fb::GuestResult::Success,
                    result: Some(success.as_union_value()),
                },
            );
            let bytes = finish_test_envelope(builder, response.as_union_value());
            assert_eq!(
                decode_guest_response(&bytes).unwrap_err().status,
                Status::InvalidArgument
            );
        }
    }

    fn finish_test_envelope(
        mut builder: FlatBufferBuilder<'static>,
        response: WIPOffset<flatbuffers::UnionWIPOffset>,
    ) -> Vec<u8> {
        let envelope = fb::Envelope::create(
            &mut builder,
            &fb::EnvelopeArgs {
                protocol_version: PROTOCOL_VERSION,
                message_type: fb::Message::GuestResponse,
                message: Some(response),
            },
        );
        fb::finish_envelope_buffer(&mut builder, envelope);
        builder.finished_data().to_vec()
    }
}
