use lynx_element_bridge_core::{
    CallbackId, Command, CommandBatch, EventMessage, ListenerId, NodeId, SessionId, Status,
};
use lynx_element_bridge_wasm_guest::{
    EventRequest, GuestResponse, GuestResult, MountRequest, PROTOCOL_VERSION_V1,
    decode_event_request, decode_guest_response, decode_mount_request, encode_event_request,
    encode_guest_response, encode_mount_request,
};

fn ids() -> (SessionId, NodeId, NodeId, NodeId, ListenerId, CallbackId) {
    (
        SessionId::new(7).unwrap(),
        NodeId::new(1).unwrap(),
        NodeId::new(2).unwrap(),
        NodeId::new(3).unwrap(),
        ListenerId::new(4).unwrap(),
        CallbackId::new(5).unwrap(),
    )
}

#[test]
fn protocol_v1_round_trips_mount_event_and_every_command() {
    let (session, root, node, child, listener, callback) = ids();
    let mount = MountRequest {
        protocol_version: PROTOCOL_VERSION_V1,
        session,
        root,
    };
    assert_eq!(
        decode_mount_request(&encode_mount_request(&mount).unwrap()).unwrap(),
        mount
    );

    let event = EventRequest {
        protocol_version: PROTOCOL_VERSION_V1,
        event: EventMessage {
            session,
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
        protocol_version: PROTOCOL_VERSION_V1,
        result: GuestResult::Ok(CommandBatch {
            session,
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
        protocol_version: PROTOCOL_VERSION_V1,
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
    let (session, root, ..) = ids();
    let wrong_version = encode_mount_request(&MountRequest {
        protocol_version: 2,
        session,
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

    // A valid postcard shape with a zero SessionId must still preserve the core ID invariant.
    let zero_id = postcard::to_allocvec(&(PROTOCOL_VERSION_V1, 0_u32, 1_u32)).unwrap();
    assert_eq!(
        decode_mount_request(&zero_id).unwrap_err().status,
        Status::InvalidArgument
    );
}
