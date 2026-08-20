#![deny(unsafe_code)]

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread::{self, ThreadId};

use lynx_element_bridge_core::{
    BridgeError, CommandBatch, EventMessage, NodeId, ResponseBatch, SessionId, Status,
};
use lynx_element_bridge_wire::{
    decode_event, decode_response, encode_command_batch, encode_failure,
};

pub type LynxElementBridgeSession = u32;

#[repr(C)]
#[derive(Debug)]
pub struct LynxElementBridgeBuffer {
    pub data: *mut u8,
    pub len: usize,
}

#[repr(C)]
pub struct LynxElementBridgeMountResult {
    pub session: LynxElementBridgeSession,
    pub response: LynxElementBridgeBuffer,
}

#[repr(C)]
pub struct LynxElementBridgeDestroyResult {
    pub consumed: u32,
    pub response: LynxElementBridgeBuffer,
}

impl LynxElementBridgeBuffer {
    fn empty() -> Self {
        Self {
            data: ptr::null_mut(),
            len: 0,
        }
    }

    fn from_vec(bytes: Vec<u8>) -> Self {
        if bytes.is_empty() {
            return Self::empty();
        }
        let mut bytes = bytes.into_boxed_slice();
        let buffer = Self {
            data: bytes.as_mut_ptr(),
            len: bytes.len(),
        };
        std::mem::forget(bytes);
        buffer
    }
}

#[derive(Clone, Debug)]
pub struct BackendError {
    pub status: Status,
    pub message: String,
    poison_session: bool,
}

impl BackendError {
    pub fn fatal(status: Status, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            poison_session: true,
        }
    }

    pub fn recoverable(status: Status, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            poison_session: false,
        }
    }
}

impl From<BridgeError> for BackendError {
    fn from(error: BridgeError) -> Self {
        Self::fatal(error.status, error.message)
    }
}

pub trait BridgeBackend {
    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BackendError>;

    fn destroy(self: Box<Self>, poisoned: bool) -> Result<CommandBatch, BackendError>;

    fn discard_pending(&mut self);

    fn abandon(&mut self) {}
}

struct Session {
    backend: Option<Box<dyn BridgeBackend>>,
    poisoned: bool,
    last_response: Option<ResponseBatch>,
}

#[derive(Clone, Debug)]
struct SessionOwner {
    thread_id: ThreadId,
}

struct ThreadSessions {
    sessions: RefCell<HashMap<LynxElementBridgeSession, Session>>,
}

impl ThreadSessions {
    fn new() -> Self {
        Self {
            sessions: RefCell::new(HashMap::new()),
        }
    }
}

impl Drop for ThreadSessions {
    fn drop(&mut self) {
        let sessions = self.sessions.get_mut();
        if sessions.is_empty() {
            return;
        }
        let mut owners = lock_owners();
        for session_id in sessions.keys() {
            owners.remove(session_id);
        }
        drop(owners);
        for session in sessions.values_mut() {
            if let Some(backend) = session.backend.as_mut() {
                backend.abandon();
            }
        }
    }
}

thread_local! {
    static SESSIONS: ThreadSessions = ThreadSessions::new();
}

static NEXT_SESSION_ID: AtomicU32 = AtomicU32::new(1);
static SESSION_OWNERS: OnceLock<Mutex<HashMap<LynxElementBridgeSession, SessionOwner>>> =
    OnceLock::new();

struct SessionReservation {
    session_id: LynxElementBridgeSession,
    committed: bool,
}

impl SessionReservation {
    fn new(session_id: LynxElementBridgeSession) -> Self {
        lock_owners().insert(
            session_id,
            SessionOwner {
                thread_id: thread::current().id(),
            },
        );
        Self {
            session_id,
            committed: false,
        }
    }

    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for SessionReservation {
    fn drop(&mut self) {
        if !self.committed {
            lock_owners().remove(&self.session_id);
        }
    }
}

fn lock_owners() -> MutexGuard<'static, HashMap<LynxElementBridgeSession, SessionOwner>> {
    SESSION_OWNERS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn next_session_id() -> Result<LynxElementBridgeSession, BackendError> {
    NEXT_SESSION_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            if current == 0 {
                None
            } else if current == u32::MAX {
                Some(0)
            } else {
                Some(current + 1)
            }
        })
        .map_err(|_| {
            BackendError::fatal(Status::ResourceExhausted, "session ID space is exhausted")
        })
}

fn validate_session_owner(session_id: LynxElementBridgeSession) -> Result<(), BackendError> {
    if session_id == 0 {
        return Err(invalid_session(session_id));
    }
    let owner = lock_owners()
        .get(&session_id)
        .cloned()
        .ok_or_else(|| invalid_session(session_id))?;
    if owner.thread_id != thread::current().id() {
        return Err(BackendError::recoverable(
            Status::WrongThread,
            format!("session {session_id} was called from a non-owner thread"),
        ));
    }
    Ok(())
}

fn invalid_session(session_id: LynxElementBridgeSession) -> BackendError {
    BackendError::recoverable(
        Status::InvalidSession,
        format!("invalid or stale session ID {session_id}"),
    )
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).into()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "Rust panic".into()
    }
}

fn mount_internal(
    root_id: u32,
    create_backend: impl FnOnce(
        SessionId,
        NodeId,
    ) -> Result<(Box<dyn BridgeBackend>, CommandBatch), BackendError>,
) -> Result<(LynxElementBridgeSession, CommandBatch), BackendError> {
    if root_id == 0 {
        return Err(BackendError::recoverable(
            Status::InvalidArgument,
            "root ID must not be zero",
        ));
    }

    let session_id = next_session_id()?;
    let reservation = SessionReservation::new(session_id);
    let (backend, response) = create_backend(
        SessionId::new(session_id).map_err(BackendError::from)?,
        NodeId::new(root_id).map_err(BackendError::from)?,
    )?;
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.sessions.try_borrow_mut().map_err(|_| {
            BackendError::fatal(
                Status::InternalError,
                "session registry is already borrowed",
            )
        })?;
        if sessions.contains_key(&session_id) {
            return Err(BackendError::fatal(
                Status::InternalError,
                format!("duplicate session ID {session_id}"),
            ));
        }
        sessions.insert(
            session_id,
            Session {
                backend: Some(backend),
                poisoned: false,
                last_response: None,
            },
        );
        Ok(())
    })?;
    reservation.commit();
    Ok((session_id, response))
}

pub fn mount(
    root_id: u32,
    create_backend: impl FnOnce(
        SessionId,
        NodeId,
    ) -> Result<(Box<dyn BridgeBackend>, CommandBatch), BackendError>,
) -> LynxElementBridgeMountResult {
    let mounted = catch_unwind(AssertUnwindSafe(|| mount_internal(root_id, create_backend)));
    match mounted {
        Ok(Ok((session, response))) => LynxElementBridgeMountResult {
            session,
            response: LynxElementBridgeBuffer::from_vec(response_wire(Ok(response), session)),
        },
        Ok(Err(error)) => LynxElementBridgeMountResult {
            session: 0,
            response: LynxElementBridgeBuffer::from_vec(response_wire(Err(error), 0)),
        },
        Err(payload) => LynxElementBridgeMountResult {
            session: 0,
            response: LynxElementBridgeBuffer::from_vec(response_wire(
                Err(BackendError::recoverable(
                    Status::Panic,
                    panic_message(payload.as_ref()),
                )),
                0,
            )),
        },
    }
}

fn dispatch_internal(
    session_id: LynxElementBridgeSession,
    event: EventMessage,
) -> Result<CommandBatch, BackendError> {
    validate_session_owner(session_id)?;
    if event.session != SessionId::new(session_id).map_err(BackendError::from)? {
        return Err(BackendError::recoverable(
            Status::InvalidSession,
            "Event channel session does not match the active session",
        ));
    }
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.sessions.try_borrow_mut().map_err(|_| {
            BackendError::fatal(
                Status::InternalError,
                "session registry is already borrowed",
            )
        })?;
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(|| invalid_session(session_id))?;
        if session.poisoned {
            return Err(BackendError::recoverable(
                Status::HostError,
                format!("session {session_id} is permanently poisoned"),
            ));
        }
        let backend = session.backend.as_mut().ok_or_else(|| {
            BackendError::fatal(
                Status::InternalError,
                format!("session {session_id} has no backend"),
            )
        })?;
        match backend.dispatch_event(event) {
            Ok(response) => Ok(response),
            Err(error) => {
                if error.poison_session {
                    session.poisoned = true;
                    backend.discard_pending();
                }
                Err(error)
            }
        }
    })
}

fn poison_session_after_boundary_panic(session_id: LynxElementBridgeSession) {
    if validate_session_owner(session_id).is_err() {
        return;
    }
    SESSIONS.with(|sessions| {
        if let Ok(mut sessions) = sessions.sessions.try_borrow_mut() {
            if let Some(session) = sessions.get_mut(&session_id) {
                session.poisoned = true;
                if let Some(backend) = session.backend.as_mut() {
                    backend.discard_pending();
                }
            }
        }
    });
}

/// Dispatches a borrowed protocol v2 Event channel envelope.
///
/// # Safety
///
/// When `event_len` is nonzero, `event` must point to that many readable bytes.
#[allow(unsafe_code)]
pub unsafe fn dispatch_event(
    session: LynxElementBridgeSession,
    event: *const u8,
    event_len: usize,
) -> LynxElementBridgeBuffer {
    let dispatched = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: This forwards the caller obligations documented on this function.
        let event = unsafe { copy_bytes(event, event_len) }?;
        let event = decode_event(&event).map_err(|error| {
            BackendError::recoverable(
                Status::InvalidArgument,
                format!("invalid Event channel envelope: {error}"),
            )
        })?;
        dispatch_internal(session, event)
    }));
    let result = match dispatched {
        Ok(result) => result,
        Err(payload) => {
            let _ = catch_unwind(AssertUnwindSafe(|| {
                poison_session_after_boundary_panic(session)
            }));
            Err(BackendError::recoverable(
                Status::Panic,
                panic_message(payload.as_ref()),
            ))
        }
    };
    LynxElementBridgeBuffer::from_vec(response_wire(result, session))
}

fn complete_internal(
    session_id: LynxElementBridgeSession,
    bytes: Vec<u8>,
) -> Result<Vec<u8>, BackendError> {
    validate_session_owner(session_id)?;
    SESSIONS.with(|sessions| {
        let sessions = sessions.sessions.try_borrow().map_err(|_| {
            BackendError::fatal(
                Status::InternalError,
                "session registry is already borrowed",
            )
        })?;
        let session = sessions
            .get(&session_id)
            .ok_or_else(|| invalid_session(session_id))?;
        if session.poisoned {
            return Err(BackendError::recoverable(
                Status::HostError,
                format!("session {session_id} is permanently poisoned"),
            ));
        }
        Ok(())
    })?;
    let response = decode_response(&bytes).map_err(|error| {
        BackendError::recoverable(
            Status::InvalidArgument,
            format!("invalid Result channel envelope: {error}"),
        )
    })?;
    if response.session != Some(SessionId::new(session_id).map_err(BackendError::from)?) {
        return Err(BackendError::recoverable(
            Status::InvalidSession,
            "Result channel session does not match the active session",
        ));
    }
    if !response.committed {
        return Err(BackendError::recoverable(
            Status::InvalidArgument,
            "Result channel response is not committed",
        ));
    }
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.sessions.try_borrow_mut().map_err(|_| {
            BackendError::fatal(
                Status::InternalError,
                "session registry is already borrowed",
            )
        })?;
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(|| invalid_session(session_id))?;
        if session.poisoned {
            return Err(BackendError::recoverable(
                Status::HostError,
                format!("session {session_id} is permanently poisoned"),
            ));
        }
        session.last_response = Some(response);
        Ok(bytes)
    })
}

/// Accepts a borrowed synchronous Result channel response.
///
/// # Safety
///
/// When `response_len` is nonzero, `response` must point to that many readable bytes.
#[allow(unsafe_code)]
pub unsafe fn complete_batch(
    session: LynxElementBridgeSession,
    response: *const u8,
    response_len: usize,
) -> LynxElementBridgeBuffer {
    let completed = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: This forwards the caller obligations documented on this function.
        let response = unsafe { copy_bytes(response, response_len) }?;
        complete_internal(session, response)
    }));
    let result = match completed {
        Ok(result) => result,
        Err(payload) => Err(BackendError::recoverable(
            Status::Panic,
            panic_message(payload.as_ref()),
        )),
    };
    LynxElementBridgeBuffer::from_vec(match result {
        Ok(response) => response,
        Err(error) => encode_failure(session, 0, error.status, &error.message),
    })
}

fn destroy_internal(
    session_id: LynxElementBridgeSession,
    consumed: &mut bool,
) -> Result<CommandBatch, BackendError> {
    validate_session_owner(session_id)?;
    let mut session = SESSIONS.with(|sessions| {
        sessions
            .sessions
            .try_borrow_mut()
            .map_err(|_| {
                BackendError::fatal(
                    Status::InternalError,
                    "session registry is already borrowed",
                )
            })?
            .remove(&session_id)
            .ok_or_else(|| invalid_session(session_id))
    })?;
    lock_owners().remove(&session_id);
    *consumed = true;

    let was_poisoned = session.poisoned;
    let backend = session.backend.take().ok_or_else(|| {
        BackendError::fatal(
            Status::InternalError,
            format!("session {session_id} has no backend"),
        )
    })?;
    let destroyed = catch_unwind(AssertUnwindSafe(|| backend.destroy(was_poisoned)));
    if was_poisoned {
        return Err(BackendError::recoverable(
            Status::HostError,
            format!("session {session_id} was destroyed after becoming permanently poisoned"),
        ));
    }
    match destroyed {
        Ok(result) => result,
        Err(payload) => Err(BackendError::recoverable(
            Status::Panic,
            panic_message(payload.as_ref()),
        )),
    }
}

pub fn destroy_session(session: LynxElementBridgeSession) -> LynxElementBridgeDestroyResult {
    let mut consumed = false;
    let destroyed = catch_unwind(AssertUnwindSafe(|| {
        destroy_internal(session, &mut consumed)
    }));
    let result = match destroyed {
        Ok(result) => result,
        Err(payload) => Err(BackendError::recoverable(
            Status::Panic,
            panic_message(payload.as_ref()),
        )),
    };
    LynxElementBridgeDestroyResult {
        consumed: u32::from(consumed),
        response: LynxElementBridgeBuffer::from_vec(response_wire(result, session)),
    }
}

#[allow(unsafe_code)]
unsafe fn copy_bytes(data: *const u8, len: usize) -> Result<Vec<u8>, BackendError> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if data.is_null() || len > isize::MAX as usize {
        return Err(BackendError::recoverable(
            Status::InvalidArgument,
            "input byte span is invalid",
        ));
    }
    // SAFETY: The C contract requires `data` to reference `len` readable bytes for this call.
    Ok(unsafe { std::slice::from_raw_parts(data, len) }.to_vec())
}

fn fallback_internal_error() -> Vec<u8> {
    encode_failure(0, 0, Status::InternalError, "serialization failure")
}

fn response_wire(
    result: Result<CommandBatch, BackendError>,
    session: LynxElementBridgeSession,
) -> Vec<u8> {
    match result {
        Ok(response) => {
            encode_command_batch(&response).unwrap_or_else(|_| fallback_internal_error())
        }
        Err(error) => encode_failure(session, 0, error.status, &error.message),
    }
}

/// Frees one buffer returned by this crate.
///
/// # Safety
///
/// `buffer` must be empty or an unmodified, not-yet-freed buffer returned by this crate.
#[allow(unsafe_code)]
pub unsafe fn buffer_free(buffer: LynxElementBridgeBuffer) {
    if buffer.data.is_null() {
        return;
    }
    if buffer.len == 0 || buffer.len > isize::MAX as usize {
        return;
    }
    let slice = ptr::slice_from_raw_parts_mut(buffer.data, buffer.len);
    // SAFETY: The C contract transfers back the exact boxed slice returned by this crate.
    drop(unsafe { Box::from_raw(slice) });
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use std::ptr;
    use std::sync::{Arc, Barrier};

    use lynx_element_bridge_core::{CommandBatch, ResponseBatch};
    use lynx_element_bridge_wire::{decode_response, encode_response};

    use super::*;

    struct EmptyBackend {
        session: SessionId,
        panic_on_dispatch: bool,
    }

    impl BridgeBackend for EmptyBackend {
        fn dispatch_event(&mut self, _: EventMessage) -> Result<CommandBatch, BackendError> {
            assert!(!self.panic_on_dispatch, "contained backend panic");
            Err(BackendError::recoverable(
                Status::InvalidListener,
                "invalid listener",
            ))
        }

        fn destroy(self: Box<Self>, _: bool) -> Result<CommandBatch, BackendError> {
            Ok(CommandBatch {
                session: self.session,
                sequence: 1,
                commands: Vec::new(),
                final_commit: true,
            })
        }

        fn discard_pending(&mut self) {}
    }

    fn create_empty(
        session: SessionId,
        _: NodeId,
    ) -> Result<(Box<dyn BridgeBackend>, CommandBatch), BackendError> {
        Ok((
            Box::new(EmptyBackend {
                session,
                panic_on_dispatch: false,
            }),
            CommandBatch {
                session,
                sequence: 0,
                commands: Vec::new(),
                final_commit: true,
            },
        ))
    }

    fn copy_and_free(buffer: LynxElementBridgeBuffer) -> Vec<u8> {
        let bytes = if buffer.data.is_null() {
            Vec::new()
        } else {
            // SAFETY: The buffer came from this API and remains live.
            unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) }.to_vec()
        };
        // SAFETY: The allocating API receives the buffer exactly once.
        unsafe { buffer_free(buffer) };
        bytes
    }

    #[test]
    fn malformed_spans_and_stale_sessions_return_result_failures() {
        // SAFETY: Empty buffers own no allocation and may always be returned.
        unsafe {
            buffer_free(LynxElementBridgeBuffer {
                data: ptr::null_mut(),
                len: 0,
            })
        };
        let invalid = mount(0, create_empty);
        assert_eq!(invalid.session, 0);
        assert_eq!(
            decode_response(&copy_and_free(invalid.response))
                .unwrap()
                .status,
            Status::InvalidArgument
        );

        let mounted = mount(1, create_empty);
        copy_and_free(mounted.response);
        // SAFETY: A null, nonempty span is intentionally rejected before reading.
        let malformed = unsafe { dispatch_event(mounted.session, ptr::null(), 1) };
        assert_eq!(
            decode_response(&copy_and_free(malformed)).unwrap().status,
            Status::InvalidArgument
        );
        let destroyed = destroy_session(mounted.session);
        assert_eq!(destroyed.consumed, 1);
        copy_and_free(destroyed.response);
        let stale = destroy_session(mounted.session);
        assert_eq!(stale.consumed, 0);
        assert_eq!(
            decode_response(&copy_and_free(stale.response))
                .unwrap()
                .status,
            Status::InvalidSession
        );
    }

    #[test]
    fn complete_acknowledges_committed_wire_bytes() {
        let mounted = mount(1, create_empty);
        let session = mounted.session;
        copy_and_free(mounted.response);
        let response = encode_response(&ResponseBatch {
            session: Some(SessionId::new(session).unwrap()),
            sequence: 0,
            status: Status::Ok,
            message: None,
            results: Vec::new(),
            committed: true,
        })
        .unwrap();
        // SAFETY: The response bytes remain readable for this call.
        let echoed = unsafe { complete_batch(session, response.as_ptr(), response.len()) };
        assert_eq!(copy_and_free(echoed), response);
        copy_and_free(destroy_session(session).response);
    }

    #[test]
    fn wrong_thread_destroy_does_not_consume_the_session() {
        let mounted = mount(1, create_empty);
        let session = mounted.session;
        copy_and_free(mounted.response);
        let barrier = Arc::new(Barrier::new(2));
        let worker_barrier = Arc::clone(&barrier);
        let worker = std::thread::spawn(move || {
            worker_barrier.wait();
            let destroyed = destroy_session(session);
            (
                destroyed.consumed,
                decode_response(&copy_and_free(destroyed.response))
                    .unwrap()
                    .status,
            )
        });
        barrier.wait();
        assert_eq!(worker.join().unwrap(), (0, Status::WrongThread));
        let destroyed = destroy_session(session);
        assert_eq!(destroyed.consumed, 1);
        copy_and_free(destroyed.response);
    }

    #[test]
    fn mount_and_dispatch_panics_are_contained_and_dispatch_poisons_the_session() {
        let mounted = mount(1, |_, _| panic!("contained mount panic"));
        assert_eq!(mounted.session, 0);
        let failure = decode_response(&copy_and_free(mounted.response)).unwrap();
        assert_eq!(failure.status, Status::Panic);
        assert_eq!(failure.message.as_deref(), Some("contained mount panic"));

        let mounted = mount(1, |session, _| {
            Ok((
                Box::new(EmptyBackend {
                    session,
                    panic_on_dispatch: true,
                }),
                CommandBatch {
                    session,
                    sequence: 0,
                    commands: Vec::new(),
                    final_commit: true,
                },
            ))
        });
        let session = mounted.session;
        copy_and_free(mounted.response);
        let event = lynx_element_bridge_wire::encode_event(&EventMessage {
            session: SessionId::new(session).unwrap(),
            listener: lynx_element_bridge_core::ListenerId::new(1).unwrap(),
            callback: lynx_element_bridge_core::CallbackId::new(1).unwrap(),
            content_type: "application/octet-stream".into(),
            payload: Vec::new(),
        })
        .unwrap();
        // SAFETY: The encoded event remains readable for this call.
        let panicked = unsafe { dispatch_event(session, event.as_ptr(), event.len()) };
        assert_eq!(
            decode_response(&copy_and_free(panicked)).unwrap().status,
            Status::Panic
        );
        let completion = encode_response(&ResponseBatch {
            session: Some(SessionId::new(session).unwrap()),
            sequence: 0,
            status: Status::Ok,
            message: None,
            results: Vec::new(),
            committed: true,
        })
        .unwrap();
        // SAFETY: The encoded response remains readable for this call.
        let poisoned_completion =
            unsafe { complete_batch(session, completion.as_ptr(), completion.len()) };
        let poisoned_completion = decode_response(&copy_and_free(poisoned_completion)).unwrap();
        assert_eq!(poisoned_completion.status, Status::HostError);
        assert!(
            poisoned_completion
                .message
                .as_deref()
                .unwrap()
                .contains("permanently poisoned")
        );
        // SAFETY: The encoded event remains readable for this call.
        let poisoned = unsafe { dispatch_event(session, event.as_ptr(), event.len()) };
        assert_eq!(
            decode_response(&copy_and_free(poisoned)).unwrap().status,
            Status::HostError
        );
        let destroyed = destroy_session(session);
        assert_eq!(destroyed.consumed, 1);
        let destroyed = decode_response(&copy_and_free(destroyed.response)).unwrap();
        assert_eq!(destroyed.status, Status::HostError);
        assert!(
            destroyed
                .message
                .as_deref()
                .unwrap()
                .contains("permanently poisoned")
        );
    }
}
