use lynx_element_bridge_core::{Command, EventMessage, HostFake, NodeId};
use lynx_element_bridge_wasm_guest::{
    EventRequest, GuestResponse, GuestResult, GuestRuntime, MountRequest, PROTOCOL_VERSION_V2,
    encode_event_request, encode_mount_request,
};

use super::YewCounter;
use crate::INITIAL_COUNT;

fn batch(response: GuestResponse) -> lynx_element_bridge_core::CommandBatch {
    match response.result {
        GuestResult::Ok(batch) => batch,
        GuestResult::Err { status, message } => panic!("guest failed with {status:?}: {message}"),
    }
}

#[test]
fn real_yew_guest_mount_event_and_destroy_conform_to_the_host_command_lifecycle() {
    let root = NodeId::new(1).unwrap();
    let mut runtime = GuestRuntime::<YewCounter>::new();
    let mut host = HostFake::new(root);

    let mounted = batch(
        runtime.mount(
            &encode_mount_request(&MountRequest {
                protocol_version: PROTOCOL_VERSION_V2,
                root,
            })
            .unwrap(),
        ),
    );
    let (listener, callback) = mounted
        .commands
        .iter()
        .find_map(|command| match command {
            Command::AddEventListener {
                listener, callback, ..
            } => Some((*listener, *callback)),
            _ => None,
        })
        .expect("the real Yew counter should register its tap listener");
    host.apply(&mounted).unwrap();
    assert_eq!(
        host.snapshot().children[0].children[0].children[0]
            .text
            .as_deref(),
        Some("Yew ❎ Lynx")
    );
    assert_eq!(host.listener_count(), 1);

    let updated = batch(
        runtime.dispatch_event(
            &encode_event_request(&EventRequest {
                protocol_version: PROTOCOL_VERSION_V2,
                event: EventMessage {
                    listener,
                    callback,
                    content_type: "application/vnd.lynx.tap".into(),
                    payload: vec![0, 255],
                },
            })
            .unwrap(),
        ),
    );
    host.apply(&updated).unwrap();
    assert_eq!(
        host.snapshot().children[0].children[0].children[0]
            .text
            .as_deref(),
        Some(format!("Count: {}", INITIAL_COUNT + 1).as_str())
    );

    host.apply(&batch(runtime.destroy())).unwrap();
    assert!(host.snapshot().children.is_empty());
    assert_eq!(host.listener_count(), 0);
}
