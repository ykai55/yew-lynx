use std::ptr;
use std::sync::{Arc, Barrier};

use lynx_element_bridge_core::{
    CallbackId, Command, CommandResult, EventMessage, ListenerId, ResponseBatch, ResultSlot,
    ResultValue, SessionId, Status,
};
use lynx_element_bridge_wire::{
    decode_command_batch, decode_response, encode_event, encode_response,
};

use super::*;

fn copy_and_free(buffer: YewLynxBuffer) -> Vec<u8> {
    let bytes = if buffer.data.is_null() {
        Vec::new()
    } else {
        // SAFETY: The buffer came directly from the Rust C API and is still live.
        unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) }.to_vec()
    };
    // SAFETY: The buffer is returned exactly once to its allocating API.
    unsafe { yew_lynx_buffer_free(buffer) };
    bytes
}

fn success(buffer: YewLynxBuffer) -> CommandBatch {
    decode_command_batch(&copy_and_free(buffer)).expect("expected a command batch")
}

fn failure(buffer: YewLynxBuffer) -> lynx_element_bridge_core::ResponseBatch {
    decode_response(&copy_and_free(buffer)).expect("expected a response batch")
}

fn listener_registration(batch: &CommandBatch) -> (ListenerId, CallbackId) {
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

unsafe fn dispatch_event(
    session: YewLynxSession,
    listener: ListenerId,
    callback: CallbackId,
    payload: &[u8],
) -> YewLynxBuffer {
    let event = encode_event(&EventMessage {
        session: SessionId::new(session).unwrap(),
        listener,
        callback,
        content_type: "application/vnd.lynx.tap".into(),
        payload: payload.into(),
    })
    .unwrap();
    // SAFETY: `event` stays readable for the duration of the call.
    unsafe { yew_lynx_dispatch(session, event.as_ptr(), event.len()) }
}

#[test]
fn mount_dispatch_and_destroy_use_flatbuffers_v2() {
    let mounted = yew_lynx_mount(1);
    assert_ne!(mounted.session, 0);
    let session = mounted.session;
    let initial_bytes = copy_and_free(mounted.response);
    assert_eq!(&initial_bytes[4..8], b"LEB2");
    let initial = decode_command_batch(&initial_bytes).unwrap();
    assert!(initial.final_commit);
    assert!(initial.commands.iter().any(|item| matches!(
        &item.command,
        Command::CreateRawText { text, .. } if text == "Count: 0"
    )));
    let (first_listener, first_callback) = listener_registration(&initial);

    // SAFETY: The event span is valid and the session/listener are live.
    let update =
        success(unsafe { dispatch_event(session, first_listener, first_callback, &[0, 255]) });
    assert!(update.commands.iter().any(|item| matches!(
        &item.command,
        Command::CreateRawText { text, .. } if text == "Count: 1"
    )));
    let (current_listener, _) = listener_registration(&update);

    // SAFETY: A stale listener ID is ordinary protocol data.
    let stale =
        failure(unsafe { dispatch_event(session, first_listener, first_callback, &[0, 255]) });
    assert_eq!(stale.status, Status::InvalidListener);

    let destroyed = yew_lynx_destroy(session);
    assert_eq!(destroyed.consumed, 1);
    let teardown = success(destroyed.response);
    assert!(teardown.commands.iter().any(|item| matches!(
        item.command,
        Command::RemoveEventListener { listener, .. } if listener == current_listener
    )));
    assert!(teardown.commands.iter().any(|item| matches!(
        item.command,
        Command::RemoveElement { parent, .. } if parent.get() == 1
    )));

    let stale_session = yew_lynx_destroy(session);
    assert_eq!(stale_session.consumed, 0);
    assert_eq!(
        failure(stale_session.response).status,
        Status::InvalidSession
    );
}

#[test]
fn zero_ids_and_invalid_event_spans_return_result_channel_failures() {
    let invalid_root = yew_lynx_mount(0);
    assert_eq!(invalid_root.session, 0);
    assert_eq!(
        failure(invalid_root.response).status,
        Status::InvalidArgument
    );

    let mounted = yew_lynx_mount(1);
    let session = mounted.session;
    success(mounted.response);
    // SAFETY: A null pointer with nonzero length is intentionally rejected before reading.
    let invalid = failure(unsafe { yew_lynx_dispatch(session, ptr::null(), 1) });
    assert_eq!(invalid.status, Status::InvalidArgument);

    let destroyed = yew_lynx_destroy(session);
    assert_eq!(destroyed.consumed, 1);
    success(destroyed.response);
}

#[test]
fn wrong_thread_destroy_does_not_consume_the_session() {
    let mounted = yew_lynx_mount(1);
    let session = mounted.session;
    success(mounted.response);

    let wrong_thread = std::thread::spawn(move || {
        let destroyed = yew_lynx_destroy(session);
        (destroyed.consumed, failure(destroyed.response).status)
    })
    .join()
    .unwrap();
    assert_eq!(wrong_thread, (0, Status::WrongThread));

    let destroyed = yew_lynx_destroy(session);
    assert_eq!(destroyed.consumed, 1);
    success(destroyed.response);
}

#[test]
fn sessions_are_scoped_to_their_owner_threads() {
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let mounted = yew_lynx_mount(1);
                assert_ne!(mounted.session, 0);
                success(mounted.response);
                barrier.wait();
                let destroyed = yew_lynx_destroy(mounted.session);
                assert_eq!(destroyed.consumed, 1);
                success(destroyed.response);
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn boundary_panics_are_encoded_as_result_channel_failures() {
    let response = failure(response_boundary(|| panic!("contained panic")));
    assert_eq!(response.status, Status::Panic);
    assert_eq!(response.message.as_deref(), Some("contained panic"));
    assert!(!response.committed);
}

#[test]
fn synchronous_result_channel_reaches_the_active_native_session() {
    let mounted = yew_lynx_mount(1);
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

    // SAFETY: The encoded response remains readable for the duration of the call.
    let echoed =
        copy_and_free(unsafe { yew_lynx_complete(session, response.as_ptr(), response.len()) });
    assert_eq!(echoed, response);
    SESSIONS.with(|sessions| {
        let sessions = sessions.sessions.borrow();
        assert_eq!(
            sessions
                .get(&session)
                .unwrap()
                .last_response
                .as_ref()
                .unwrap()
                .results[0]
                .value,
            Some(ResultValue::String("view".into()))
        );
    });

    success(yew_lynx_destroy(session).response);
}
