use std::ffi::CStr;
use std::ptr;

use lynx_element_bridge_core::{
    CallbackId, Command, CommandBatch, CommandResult, EventMessage, ListenerId, ResponseBatch,
    ResultSlot, ResultValue, SessionId, Status,
};
use lynx_element_bridge_ffi::{
    LynxElementBridgeBuffer, LynxElementBridgeDestroyResult, LynxElementBridgeMountResult,
    LynxElementBridgeSession,
};
use lynx_element_bridge_wire::{
    decode_command_batch, decode_response, encode_event, encode_response,
};

use super::*;

fn copy_and_free(buffer: LynxElementBridgeBuffer) -> Vec<u8> {
    let bytes = if buffer.data.is_null() {
        Vec::new()
    } else {
        // SAFETY: The buffer came directly from this C API and remains live.
        unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) }.to_vec()
    };
    // SAFETY: The buffer is returned exactly once to its allocating API.
    unsafe { lynx_element_bridge_buffer_free(buffer) };
    bytes
}

fn success(buffer: LynxElementBridgeBuffer) -> CommandBatch {
    decode_command_batch(&copy_and_free(buffer)).expect("expected a command batch")
}

fn failure(buffer: LynxElementBridgeBuffer) -> ResponseBatch {
    decode_response(&copy_and_free(buffer)).expect("expected a response batch")
}

fn registration(batch: &CommandBatch) -> (ListenerId, CallbackId) {
    batch
        .commands
        .iter()
        .find_map(|item| match item.command {
            Command::AddEventListener {
                listener, callback, ..
            } => Some((listener, callback)),
            _ => None,
        })
        .expect("batch has no listener")
}

fn event_bytes(
    session: LynxElementBridgeSession,
    listener: ListenerId,
    callback: CallbackId,
) -> Vec<u8> {
    encode_event(&EventMessage {
        session: SessionId::new(session).unwrap(),
        listener,
        callback,
        content_type: "application/vnd.lynx.tap".into(),
        payload: vec![0, 255],
    })
    .unwrap()
}

fn destroy(session: LynxElementBridgeSession) -> LynxElementBridgeDestroyResult {
    lynx_element_bridge_destroy_session(session)
}

#[test]
fn backend_identity_and_counter_flow_use_the_public_flatbuffers_v2_abi() {
    // SAFETY: The backend identity points to static NUL-terminated storage.
    assert_eq!(
        unsafe { CStr::from_ptr(lynx_element_bridge_backend()) }
            .to_str()
            .unwrap(),
        "dioxus"
    );
    // SAFETY: The backend marker points to static NUL-terminated storage.
    assert_eq!(
        unsafe { CStr::from_ptr(lynx_element_bridge_backend_marker()) }
            .to_str()
            .unwrap(),
        "lynx-element-bridge-backend:dioxus"
    );
    let LynxElementBridgeMountResult { session, response } = lynx_element_bridge_mount(1);
    assert_ne!(session, 0);
    let initial_bytes = copy_and_free(response);
    assert_eq!(&initial_bytes[4..8], b"LEB2");
    let initial = decode_command_batch(&initial_bytes).unwrap();
    assert!(initial.commands.iter().any(|item| matches!(
        &item.command,
        Command::CreateRawText { text, .. } if text == "Count: 0"
    )));
    let (listener, callback) = registration(&initial);

    let event = event_bytes(session, listener, callback);
    // SAFETY: The encoded event remains readable for each synchronous call.
    let first = success(unsafe {
        lynx_element_bridge_dispatch_event(session, event.as_ptr(), event.len())
    });
    assert!(first.commands.iter().any(|item| matches!(
        &item.command,
        Command::CreateRawText { text, .. } if text == "Count: 1"
    )));
    // SAFETY: The listener remains live and the event remains readable.
    let second = success(unsafe {
        lynx_element_bridge_dispatch_event(session, event.as_ptr(), event.len())
    });
    assert!(second.commands.iter().any(|item| matches!(
        &item.command,
        Command::CreateRawText { text, .. } if text == "Count: 2"
    )));

    let destroyed = destroy(session);
    assert_eq!(destroyed.consumed, 1);
    let teardown = success(destroyed.response);
    assert!(teardown.commands.iter().any(|item| matches!(
        item.command,
        Command::RemoveEventListener { listener: removed, .. } if removed == listener
    )));
    assert!(teardown.commands.iter().any(|item| matches!(
        item.command,
        Command::RemoveElement { parent, .. } if parent.get() == 1
    )));
}

#[test]
fn listener_callback_mismatch_is_recoverable_and_does_not_update() {
    let mounted = lynx_element_bridge_mount(1);
    let session = mounted.session;
    let initial = success(mounted.response);
    let (listener, callback) = registration(&initial);
    let mismatch = event_bytes(
        session,
        listener,
        CallbackId::new(callback.get() + 1).unwrap(),
    );
    // SAFETY: The encoded event remains readable for this call.
    let rejected = failure(unsafe {
        lynx_element_bridge_dispatch_event(session, mismatch.as_ptr(), mismatch.len())
    });
    assert_eq!(rejected.status, Status::InvalidListener);

    let valid = event_bytes(session, listener, callback);
    // SAFETY: The encoded event remains readable for this call.
    let updated = success(unsafe {
        lynx_element_bridge_dispatch_event(session, valid.as_ptr(), valid.len())
    });
    assert!(updated.commands.iter().any(|item| matches!(
        &item.command,
        Command::CreateRawText { text, .. } if text == "Count: 1"
    )));
    success(destroy(session).response);
}

#[test]
fn complete_acknowledges_a_committed_result_and_stale_tokens_are_rejected() {
    let mounted = lynx_element_bridge_mount(1);
    let session = mounted.session;
    let sequence = success(mounted.response).sequence;
    let response = encode_response(&ResponseBatch {
        session: Some(SessionId::new(session).unwrap()),
        sequence,
        status: Status::Ok,
        message: None,
        results: vec![CommandResult {
            slot: Some(ResultSlot::new(7)),
            status: Status::Ok,
            message: None,
            value: Some(ResultValue::String("view".into())),
        }],
        committed: true,
    })
    .unwrap();
    // SAFETY: The encoded response remains readable for this call.
    let echoed =
        unsafe { lynx_element_bridge_complete_batch(session, response.as_ptr(), response.len()) };
    assert_eq!(copy_and_free(echoed), response);

    let destroyed = destroy(session);
    assert_eq!(destroyed.consumed, 1);
    success(destroyed.response);
    let stale = destroy(session);
    assert_eq!(stale.consumed, 0);
    assert_eq!(failure(stale.response).status, Status::InvalidSession);
}

#[test]
fn malformed_roots_and_spans_return_result_channel_failures() {
    let mounted = lynx_element_bridge_mount(0);
    assert_eq!(mounted.session, 0);
    assert_eq!(failure(mounted.response).status, Status::InvalidArgument);

    let mounted = lynx_element_bridge_mount(1);
    let session = mounted.session;
    success(mounted.response);
    // SAFETY: A null pointer with nonzero length is intentionally rejected before reading.
    let malformed = unsafe { lynx_element_bridge_dispatch_event(session, ptr::null(), 1) };
    assert_eq!(failure(malformed).status, Status::InvalidArgument);
    success(destroy(session).response);
}

#[test]
fn wrong_thread_destroy_does_not_consume_the_session() {
    let mounted = lynx_element_bridge_mount(1);
    let session = mounted.session;
    success(mounted.response);
    let wrong_thread = std::thread::spawn(move || {
        let destroyed = destroy(session);
        (destroyed.consumed, failure(destroyed.response).status)
    })
    .join()
    .unwrap();
    assert_eq!(wrong_thread, (0, Status::WrongThread));
    let destroyed = destroy(session);
    assert_eq!(destroyed.consumed, 1);
    success(destroyed.response);
}
