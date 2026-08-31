use lynx_element_bridge_core::{
    CallbackId, Command, CommandBatch, EventMessage, ListenerId, NodeId, Status,
};
use lynx_element_bridge_protocol::{
    EventRequest, GuestResponse, GuestResult, MountRequest, PROTOCOL_VERSION, decode_event_request,
    decode_guest_response, decode_mount_request, encode_event_request, encode_guest_response,
    encode_mount_request,
};

fn ids() -> (NodeId, NodeId, NodeId, ListenerId, CallbackId) {
    (
        NodeId::new(1).unwrap(),
        NodeId::new(2).unwrap(),
        NodeId::new(3).unwrap(),
        ListenerId::new(4).unwrap(),
        CallbackId::new(5).unwrap(),
    )
}

#[test]
fn round_trips_mount_event_and_every_command() {
    let (root, node, child, listener, callback) = ids();
    let mount = MountRequest {
        protocol_version: PROTOCOL_VERSION,
        root,
    };
    assert_eq!(
        decode_mount_request(&encode_mount_request(&mount)).unwrap(),
        mount
    );

    let event = EventRequest {
        protocol_version: PROTOCOL_VERSION,
        event: EventMessage {
            listener,
            callback,
            content_type: "application/octet-stream".into(),
            payload: vec![0, 127, 255],
        },
    };
    assert_eq!(
        decode_event_request(&encode_event_request(&event)).unwrap(),
        event
    );

    let commands = vec![
        Command::ImportStyleSheet {
            fragment: vec![0, 127, 255],
        },
        Command::CreateElement {
            node,
            tag: "view".into(),
        },
        Command::CreateRawText {
            node: child,
            text: "hello".into(),
        },
        Command::AppendElement {
            parent: root,
            child: node,
        },
        Command::InsertElementBefore {
            parent: root,
            child,
            reference: node,
        },
        Command::RemoveElement {
            parent: root,
            child: node,
        },
        Command::DestroyNode { node },
        Command::SetAttribute {
            node,
            name: "missing".into(),
            value: None,
        },
        Command::SetAttribute {
            node,
            name: "empty".into(),
            value: Some(String::new()),
        },
        Command::AddEventListener {
            node,
            listener,
            callback,
            name: "tap".into(),
        },
        Command::RemoveEventListener {
            node,
            listener,
            callback,
            name: "tap".into(),
        },
    ];
    let response = GuestResponse {
        protocol_version: PROTOCOL_VERSION,
        result: GuestResult::Ok(CommandBatch {
            sequence: 9,
            commands,
            final_commit: true,
        }),
    };
    assert_eq!(
        decode_guest_response(&encode_guest_response(&response)).unwrap(),
        response
    );
}

#[test]
fn round_trips_empty_success_and_every_valid_error_status() {
    let success = GuestResponse {
        protocol_version: PROTOCOL_VERSION,
        result: GuestResult::Ok(CommandBatch {
            sequence: 0,
            commands: Vec::new(),
            final_commit: false,
        }),
    };
    assert_eq!(
        decode_guest_response(&encode_guest_response(&success)).unwrap(),
        success
    );

    for status in [
        Status::InvalidArgument,
        Status::InvalidSession,
        Status::WrongThread,
        Status::Unsupported,
        Status::InvalidOwnership,
        Status::InvalidListener,
        Status::ResourceExhausted,
        Status::HostError,
        Status::Panic,
        Status::InternalError,
    ] {
        let response = GuestResponse {
            protocol_version: PROTOCOL_VERSION,
            result: GuestResult::Err {
                status,
                message: String::new(),
            },
        };
        assert_eq!(
            decode_guest_response(&encode_guest_response(&response)).unwrap(),
            response
        );
    }
}

#[test]
fn rejects_identifier_version_kind_corruption_truncation_zero_ids_and_ok_error() {
    let (root, ..) = ids();
    let valid = encode_mount_request(&MountRequest {
        protocol_version: PROTOCOL_VERSION,
        root,
    });
    let mut wrong_identifier = valid.clone();
    wrong_identifier[4..8].copy_from_slice(b"NOPE");
    assert_eq!(
        decode_mount_request(&wrong_identifier).unwrap_err().status,
        Status::InvalidArgument
    );

    let wrong_version = encode_mount_request(&MountRequest {
        protocol_version: 1,
        root,
    });
    assert_eq!(
        decode_mount_request(&wrong_version).unwrap_err().status,
        Status::Unsupported
    );

    let event = encode_event_request(&EventRequest {
        protocol_version: PROTOCOL_VERSION,
        event: EventMessage {
            listener: ListenerId::new(1).unwrap(),
            callback: CallbackId::new(1).unwrap(),
            content_type: String::new(),
            payload: Vec::new(),
        },
    });
    assert_eq!(
        decode_mount_request(&event).unwrap_err().status,
        Status::InvalidArgument
    );
    assert_eq!(
        decode_mount_request(&valid[..valid.len() - 1])
            .unwrap_err()
            .status,
        Status::InvalidArgument
    );
    assert_eq!(
        decode_mount_request(&[0xff]).unwrap_err().status,
        Status::InvalidArgument
    );

    let mut zero_id = valid;
    let root_position = zero_id
        .windows(4)
        .rposition(|window| window == 1_u32.to_le_bytes())
        .unwrap();
    zero_id[root_position..root_position + 4].fill(0);
    assert_eq!(
        decode_mount_request(&zero_id).unwrap_err().status,
        Status::InvalidArgument
    );

    let ok_error = GuestResponse {
        protocol_version: PROTOCOL_VERSION,
        result: GuestResult::Err {
            status: Status::Ok,
            message: "invalid".into(),
        },
    };
    assert_eq!(
        decode_guest_response(&encode_guest_response(&ok_error))
            .unwrap_err()
            .status,
        Status::InvalidArgument
    );
}

#[test]
fn representative_encodings_match_golden_fixtures() {
    let mount = encode_mount_request(&MountRequest {
        protocol_version: PROTOCOL_VERSION,
        root: NodeId::new(1).unwrap(),
    });
    let event = encode_event_request(&EventRequest {
        protocol_version: PROTOCOL_VERSION,
        event: EventMessage {
            listener: ListenerId::new(2).unwrap(),
            callback: CallbackId::new(3).unwrap(),
            content_type: "x/test".into(),
            payload: vec![0, 255],
        },
    });
    let success = encode_guest_response(&GuestResponse {
        protocol_version: PROTOCOL_VERSION,
        result: GuestResult::Ok(CommandBatch {
            sequence: 1,
            commands: Vec::new(),
            final_commit: true,
        }),
    });
    let failure = encode_guest_response(&GuestResponse {
        protocol_version: PROTOCOL_VERSION,
        result: GuestResult::Err {
            status: Status::HostError,
            message: "failed".into(),
        },
    });
    let stylesheet = encode_guest_response(&GuestResponse {
        protocol_version: PROTOCOL_VERSION,
        result: GuestResult::Ok(CommandBatch {
            sequence: 7,
            commands: vec![Command::ImportStyleSheet {
                fragment: vec![0, 127, 255],
            }],
            final_commit: true,
        }),
    });

    assert_eq!(mount, MOUNT_FIXTURE);
    assert_eq!(event, EVENT_FIXTURE);
    assert_eq!(success, SUCCESS_FIXTURE);
    assert_eq!(failure, FAILURE_FIXTURE);
    assert_eq!(stylesheet, STYLESHEET_FIXTURE);
}

const MOUNT_FIXTURE: &[u8] = &[
    20, 0, 0, 0, 76, 69, 66, 52, 0, 0, 10, 0, 18, 0, 8, 0, 7, 0, 12, 0, 10, 0, 0, 0, 0, 0, 0, 1, 4,
    0, 0, 0, 12, 0, 0, 0, 0, 0, 6, 0, 8, 0, 4, 0, 6, 0, 0, 0, 1, 0, 0, 0,
];
const EVENT_FIXTURE: &[u8] = &[
    20, 0, 0, 0, 76, 69, 66, 52, 0, 0, 10, 0, 16, 0, 8, 0, 7, 0, 12, 0, 10, 0, 0, 0, 0, 0, 0, 2, 4,
    0, 0, 0, 16, 0, 0, 0, 12, 0, 20, 0, 4, 0, 8, 0, 12, 0, 16, 0, 12, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0,
    0, 16, 0, 0, 0, 4, 0, 0, 0, 2, 0, 0, 0, 0, 255, 0, 0, 6, 0, 0, 0, 120, 47, 116, 101, 115, 116,
    0, 0,
];
const SUCCESS_FIXTURE: &[u8] = &[
    20, 0, 0, 0, 76, 69, 66, 52, 0, 0, 10, 0, 16, 0, 8, 0, 7, 0, 12, 0, 10, 0, 0, 0, 0, 0, 0, 3, 4,
    0, 0, 0, 12, 0, 0, 0, 8, 0, 14, 0, 7, 0, 8, 0, 8, 0, 0, 0, 0, 0, 0, 1, 12, 0, 0, 0, 0, 0, 6, 0,
    10, 0, 4, 0, 6, 0, 0, 0, 16, 0, 0, 0, 0, 0, 10, 0, 16, 0, 8, 0, 12, 0, 7, 0, 10, 0, 0, 0, 0, 0,
    0, 1, 1, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
];
const FAILURE_FIXTURE: &[u8] = &[
    20, 0, 0, 0, 76, 69, 66, 52, 0, 0, 10, 0, 16, 0, 8, 0, 7, 0, 12, 0, 10, 0, 0, 0, 0, 0, 0, 3, 4,
    0, 0, 0, 4, 0, 0, 0, 244, 255, 255, 255, 0, 0, 0, 2, 12, 0, 0, 0, 8, 0, 12, 0, 7, 0, 8, 0, 8,
    0, 0, 0, 0, 0, 0, 8, 4, 0, 0, 0, 6, 0, 0, 0, 102, 97, 105, 108, 101, 100, 0, 0,
];
const STYLESHEET_FIXTURE: &[u8] = &[
    20, 0, 0, 0, 76, 69, 66, 52, 0, 0, 10, 0, 16, 0, 8, 0, 7, 0, 12, 0, 10, 0, 0, 0, 0, 0, 0, 3, 4,
    0, 0, 0, 4, 0, 0, 0, 192, 255, 255, 255, 0, 0, 0, 1, 12, 0, 0, 0, 0, 0, 6, 0, 10, 0, 4, 0, 6,
    0, 0, 0, 16, 0, 0, 0, 0, 0, 10, 0, 16, 0, 8, 0, 12, 0, 7, 0, 10, 0, 0, 0, 0, 0, 0, 1, 7, 0, 0,
    0, 4, 0, 0, 0, 1, 0, 0, 0, 12, 0, 0, 0, 8, 0, 14, 0, 7, 0, 8, 0, 8, 0, 0, 0, 0, 0, 0, 10, 12,
    0, 0, 0, 0, 0, 6, 0, 8, 0, 4, 0, 6, 0, 0, 0, 4, 0, 0, 0, 3, 0, 0, 0, 0, 127, 255, 0,
];
