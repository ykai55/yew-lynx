use lynx_element_bridge_core::{
    CallbackId, Command, CommandBatch, EventMessage, ListenerId, NodeId, Status,
};
use lynx_element_bridge_wasm_guest::{
    EventRequest, GuestResponse, GuestResult, MountRequest, PROTOCOL_VERSION_V2,
    decode_event_request, decode_guest_response, decode_mount_request, encode_event_request,
    encode_guest_response, encode_mount_request,
};
use serde::Serialize;

#[derive(Serialize)]
struct V1MountRequest {
    protocol_version: u32,
    session: u32,
    root: u32,
}

#[derive(Serialize)]
struct V1EventRequest {
    protocol_version: u32,
    event: V1EventMessage,
}

#[derive(Serialize)]
struct V1EventMessage {
    session: u32,
    listener: u32,
    callback: u32,
    content_type: String,
    payload: Vec<u8>,
}

#[derive(Serialize)]
struct V1GuestResponse {
    protocol_version: u32,
    result: V1GuestResult,
}

#[derive(Serialize)]
#[allow(dead_code)]
enum V1GuestResult {
    Ok(()),
    Err { status: Status, message: String },
}

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
fn protocol_v2_round_trips_mount_event_and_every_command() {
    let (root, node, child, listener, callback) = ids();
    let mount = MountRequest {
        protocol_version: PROTOCOL_VERSION_V2,
        root,
    };
    assert_eq!(
        decode_mount_request(&encode_mount_request(&mount).unwrap()).unwrap(),
        mount
    );

    let event = EventRequest {
        protocol_version: PROTOCOL_VERSION_V2,
        event: EventMessage {
            listener,
            callback,
            content_type: "application/octet-stream".into(),
            payload: vec![0, 127, 255],
        },
    };
    assert_eq!(
        decode_event_request(&encode_event_request(&event).unwrap()).unwrap(),
        event
    );

    let commands = vec![
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
            name: "id".into(),
            value: Some("counter".into()),
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
        protocol_version: PROTOCOL_VERSION_V2,
        result: GuestResult::Ok(CommandBatch {
            sequence: 9,
            commands,
            final_commit: true,
        }),
    };
    assert_eq!(
        decode_guest_response(&encode_guest_response(&response).unwrap()).unwrap(),
        response
    );

    let error = GuestResponse {
        protocol_version: PROTOCOL_VERSION_V2,
        result: GuestResult::Err {
            status: Status::HostError,
            message: "failed".into(),
        },
    };
    assert_eq!(
        decode_guest_response(&encode_guest_response(&error).unwrap()).unwrap(),
        error
    );
}

#[test]
fn decoding_rejects_wrong_versions_corruption_and_zero_ids() {
    let (root, ..) = ids();
    let wrong_version = encode_mount_request(&MountRequest {
        protocol_version: 1,
        root,
    })
    .unwrap();
    assert_eq!(
        decode_mount_request(&wrong_version).unwrap_err().status,
        Status::Unsupported
    );
    assert_eq!(
        decode_mount_request(&[0xff]).unwrap_err().status,
        Status::InvalidArgument
    );

    // A valid postcard shape with a zero NodeId must still preserve the core ID invariant.
    let zero_id = postcard::to_allocvec(&(PROTOCOL_VERSION_V2, 0_u32)).unwrap();
    assert_eq!(
        decode_mount_request(&zero_id).unwrap_err().status,
        Status::InvalidArgument
    );
}

#[test]
fn decoding_rejects_real_v1_message_layouts() {
    let mount = postcard::to_allocvec(&V1MountRequest {
        protocol_version: 1,
        session: 7,
        root: 1,
    })
    .unwrap();
    assert!(decode_mount_request(&mount).is_err());

    let event = postcard::to_allocvec(&V1EventRequest {
        protocol_version: 1,
        event: V1EventMessage {
            session: 7,
            listener: 4,
            callback: 5,
            content_type: "application/octet-stream".into(),
            payload: vec![0, 255],
        },
    })
    .unwrap();
    assert!(decode_event_request(&event).is_err());

    let response = postcard::to_allocvec(&V1GuestResponse {
        protocol_version: 1,
        result: V1GuestResult::Err {
            status: Status::HostError,
            message: "v1".into(),
        },
    })
    .unwrap();
    assert!(decode_guest_response(&response).is_err());
}
