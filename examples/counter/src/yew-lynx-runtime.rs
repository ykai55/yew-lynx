//! Yew-Lynx runtime backed by the versioned mutation protocol.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread::{self, ThreadId};

use yew::{NativeAppHandle, NativeListener, NativeNode, NativeRenderer};
use yew_lynx_adapter::{
    BackendError, FailureResponse, JS_MAX_SAFE_INTEGER, ProtocolResponse, RecordingBackend,
    SuccessResponse,
};

use crate::app::Counter;

pub const YEW_LYNX_COUNTER_STATUS_OK: u32 = 0;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT: u32 = 1;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_UTF8: u32 = 2;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_SESSION: u32 = 3;
pub const YEW_LYNX_COUNTER_STATUS_WRONG_THREAD: u32 = 4;
// Reserved for ABI v1 compatibility. Roots are scoped to sessions and are never globally rejected.
pub const YEW_LYNX_COUNTER_STATUS_DUPLICATE_ROOT: u32 = 5;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_LISTENER: u32 = 6;
pub const YEW_LYNX_COUNTER_STATUS_EVENT_MISMATCH: u32 = 7;
pub const YEW_LYNX_COUNTER_STATUS_BACKEND_ERROR: u32 = 8;
pub const YEW_LYNX_COUNTER_STATUS_PANIC: u32 = 9;
pub const YEW_LYNX_COUNTER_STATUS_SESSION_POISONED: u32 = 10;
pub const YEW_LYNX_COUNTER_STATUS_RESOURCE_EXHAUSTED: u32 = 11;
pub const YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR: u32 = 12;

pub type YewLynxSession = u64;

#[repr(C)]
#[derive(Debug)]
pub struct YewLynxBuffer {
    pub data: *mut u8,
    pub len: usize,
}

#[repr(C)]
pub struct YewLynxMountResult {
    pub session: YewLynxSession,
    pub response: YewLynxBuffer,
}

#[repr(C)]
pub struct YewLynxDestroyResult {
    pub consumed: u32,
    pub response: YewLynxBuffer,
}

impl YewLynxBuffer {
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

struct Session {
    backend: Rc<RecordingBackend>,
    app: Option<NativeAppHandle<Counter>>,
    poisoned: bool,
}

#[derive(Clone, Debug)]
struct SessionOwner {
    thread_id: ThreadId,
}

struct ThreadSessions {
    sessions: RefCell<HashMap<YewLynxSession, Session>>,
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
            if let Some(mut app) = session.app.take() {
                app.abandon();
            }
        }
        // The host tree is already unreachable here; explicit destroy is still required for it.
    }
}

thread_local! {
    static SESSIONS: ThreadSessions = ThreadSessions::new();
}

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
static SESSION_OWNERS: OnceLock<Mutex<HashMap<YewLynxSession, SessionOwner>>> = OnceLock::new();

#[derive(Clone, Debug)]
struct ApiError {
    status: u32,
    message: String,
}

impl ApiError {
    fn new(status: u32, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl From<BackendError> for ApiError {
    fn from(error: BackendError) -> Self {
        let status = match error {
            BackendError::InvalidListener(_) => YEW_LYNX_COUNTER_STATUS_INVALID_LISTENER,
            BackendError::EventMismatch { .. } => YEW_LYNX_COUNTER_STATUS_EVENT_MISMATCH,
            BackendError::IdExhausted(_) => YEW_LYNX_COUNTER_STATUS_RESOURCE_EXHAUSTED,
            _ => YEW_LYNX_COUNTER_STATUS_BACKEND_ERROR,
        };
        Self::new(status, error.to_string())
    }
}

struct SessionReservation {
    session_id: YewLynxSession,
    committed: bool,
}

impl SessionReservation {
    fn new(session_id: YewLynxSession) -> Self {
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

fn lock_owners() -> MutexGuard<'static, HashMap<YewLynxSession, SessionOwner>> {
    SESSION_OWNERS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn next_session_id() -> Result<YewLynxSession, ApiError> {
    NEXT_SESSION_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            if current == 0 || current > JS_MAX_SAFE_INTEGER {
                None
            } else if current == JS_MAX_SAFE_INTEGER {
                Some(0)
            } else {
                Some(current + 1)
            }
        })
        .map_err(|_| {
            ApiError::new(
                YEW_LYNX_COUNTER_STATUS_RESOURCE_EXHAUSTED,
                "session ID space is exhausted",
            )
        })
}

fn validate_session_owner(session_id: YewLynxSession) -> Result<(), ApiError> {
    if session_id == 0 || session_id > JS_MAX_SAFE_INTEGER {
        return Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_SESSION,
            format!("invalid or stale session ID {session_id}"),
        ));
    }
    let owner = lock_owners().get(&session_id).cloned().ok_or_else(|| {
        ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_SESSION,
            format!("invalid or stale session ID {session_id}"),
        )
    })?;
    if owner.thread_id != thread::current().id() {
        return Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_WRONG_THREAD,
            format!("session {session_id} was called from a non-owner thread"),
        ));
    }
    Ok(())
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

fn mount(root_id: u64) -> Result<(YewLynxSession, SuccessResponse), ApiError> {
    if root_id == 0 || root_id > JS_MAX_SAFE_INTEGER {
        return Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT,
            format!("root ID must be between 1 and {JS_MAX_SAFE_INTEGER}"),
        ));
    }

    let session_id = next_session_id()?;
    let reservation = SessionReservation::new(session_id);
    let backend = RecordingBackend::new(NativeNode(root_id))?;
    let rendered = catch_unwind(AssertUnwindSafe({
        let backend = Rc::clone(&backend);
        move || NativeRenderer::<Counter>::new(backend, NativeNode(root_id)).render()
    }));
    let mut app = match rendered {
        Ok(app) => app,
        Err(payload) => {
            backend.discard_pending();
            return Err(ApiError::new(
                YEW_LYNX_COUNTER_STATUS_PANIC,
                panic_message(payload.as_ref()),
            ));
        }
    };
    let response = match backend.take_response() {
        Ok(response) => response,
        Err(error) => {
            let _ = catch_unwind(AssertUnwindSafe(|| app.destroy()));
            return Err(error.into());
        }
    };

    let mut app = Some(app);
    let inserted = SESSIONS.with(|sessions| {
        let mut sessions = sessions.sessions.try_borrow_mut().map_err(|_| {
            ApiError::new(
                YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR,
                "session registry is already borrowed",
            )
        })?;
        if sessions.contains_key(&session_id) {
            return Err(ApiError::new(
                YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR,
                format!("duplicate session ID {session_id}"),
            ));
        }
        sessions.insert(
            session_id,
            Session {
                backend,
                app: app.take(),
                poisoned: false,
            },
        );
        Ok(())
    });
    if let Err(error) = inserted {
        if let Some(mut app) = app {
            let _ = catch_unwind(AssertUnwindSafe(|| app.destroy()));
        }
        return Err(error);
    }
    reservation.commit();
    Ok((session_id, response))
}

fn dispatch(
    session_id: YewLynxSession,
    listener_id: u64,
    event: &str,
) -> Result<SuccessResponse, ApiError> {
    validate_session_owner(session_id)?;
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.sessions.try_borrow_mut().map_err(|_| {
            ApiError::new(
                YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR,
                "session registry is already borrowed",
            )
        })?;
        let session = sessions.get_mut(&session_id).ok_or_else(|| {
            ApiError::new(
                YEW_LYNX_COUNTER_STATUS_INVALID_SESSION,
                format!("invalid or stale session ID {session_id}"),
            )
        })?;
        if session.poisoned {
            return Err(ApiError::new(
                YEW_LYNX_COUNTER_STATUS_SESSION_POISONED,
                format!("session {session_id} is permanently poisoned"),
            ));
        }

        let dispatched = catch_unwind(AssertUnwindSafe(|| {
            session.backend.dispatch(NativeListener(listener_id), event)
        }));
        match dispatched {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                if !matches!(
                    error,
                    BackendError::InvalidListener(_) | BackendError::EventMismatch { .. }
                ) {
                    session.poisoned = true;
                    session.backend.discard_pending();
                }
                return Err(error.into());
            }
            Err(payload) => {
                session.poisoned = true;
                session.backend.discard_pending();
                return Err(ApiError::new(
                    YEW_LYNX_COUNTER_STATUS_PANIC,
                    panic_message(payload.as_ref()),
                ));
            }
        }

        match session.backend.take_response() {
            Ok(response) => Ok(response),
            Err(error) => {
                session.poisoned = true;
                session.backend.discard_pending();
                Err(error.into())
            }
        }
    })
}

fn poison_session_after_boundary_panic(session_id: YewLynxSession) {
    if validate_session_owner(session_id).is_err() {
        return;
    }
    SESSIONS.with(|sessions| {
        if let Ok(mut sessions) = sessions.sessions.try_borrow_mut() {
            if let Some(session) = sessions.get_mut(&session_id) {
                session.poisoned = true;
                session.backend.discard_pending();
            }
        }
    });
}

fn destroy(session_id: YewLynxSession, consumed: &mut bool) -> Result<SuccessResponse, ApiError> {
    validate_session_owner(session_id)?;
    let mut session = SESSIONS.with(|sessions| {
        sessions
            .sessions
            .try_borrow_mut()
            .map_err(|_| {
                ApiError::new(
                    YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR,
                    "session registry is already borrowed",
                )
            })?
            .remove(&session_id)
            .ok_or_else(|| {
                ApiError::new(
                    YEW_LYNX_COUNTER_STATUS_INVALID_SESSION,
                    format!("invalid or stale session ID {session_id}"),
                )
            })
    })?;
    lock_owners().remove(&session_id);
    *consumed = true;

    let was_poisoned = session.poisoned;
    let mut app = session.app.take().ok_or_else(|| {
        ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR,
            format!("session {session_id} has no application handle"),
        )
    })?;
    let destroyed = catch_unwind(AssertUnwindSafe(|| app.destroy()));
    if was_poisoned {
        session.backend.discard_pending();
        return Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_SESSION_POISONED,
            format!("session {session_id} was destroyed after becoming permanently poisoned"),
        ));
    }
    match destroyed {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            app.abandon();
            session.backend.discard_pending();
            return Err(ApiError::new(
                YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR,
                error.to_string(),
            ));
        }
        Err(payload) => {
            session.backend.discard_pending();
            return Err(ApiError::new(
                YEW_LYNX_COUNTER_STATUS_PANIC,
                panic_message(payload.as_ref()),
            ));
        }
    }
    session.backend.take_response().map_err(Into::into)
}

unsafe fn copy_bytes(data: *const u8, len: usize) -> Result<Vec<u8>, ApiError> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if data.is_null() || len > isize::MAX as usize {
        return Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT,
            "input byte span is invalid",
        ));
    }
    // SAFETY: The C contract requires `data` to reference `len` readable bytes for this call.
    Ok(unsafe { std::slice::from_raw_parts(data, len) }.to_vec())
}

fn parse_decimal_id(bytes: &[u8], name: &str) -> Result<u64, ApiError> {
    let value = std::str::from_utf8(bytes).map_err(|error| {
        ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_UTF8,
            format!("{name} is not UTF-8: {error}"),
        )
    })?;
    let value: u64 = value.parse().map_err(|_| {
        ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT,
            format!("{name} must be an unsigned decimal integer"),
        )
    })?;
    if value == 0 || value > JS_MAX_SAFE_INTEGER {
        return Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT,
            format!("{name} must be between 1 and {JS_MAX_SAFE_INTEGER}"),
        ));
    }
    Ok(value)
}

fn fallback_internal_error() -> Vec<u8> {
    b"{\"version\":1,\"ok\":false,\"status\":12,\"error\":\"serialization failure\",\"operations\":[]}"
        .to_vec()
}

fn response_json(result: Result<SuccessResponse, ApiError>) -> Vec<u8> {
    let response = match result {
        Ok(response) => ProtocolResponse::Success(response),
        Err(error) => match FailureResponse::new(error.status, error.message, Vec::new()) {
            Ok(response) => ProtocolResponse::Failure(response),
            Err(_) => return fallback_internal_error(),
        },
    };
    response
        .to_json()
        .unwrap_or_else(|_| fallback_internal_error())
}

#[cfg(test)]
fn response_boundary(
    operation: impl FnOnce() -> Result<SuccessResponse, ApiError>,
) -> YewLynxBuffer {
    let result = match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(payload) => Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_PANIC,
            panic_message(payload.as_ref()),
        )),
    };
    YewLynxBuffer::from_vec(response_json(result))
}

/// Mounts a counter session using an unsigned decimal UTF-8 root ID.
///
/// # Safety
///
/// When `root_id_len` is nonzero, `root_id` must point to that many readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yew_lynx_mount(
    root_id: *const u8,
    root_id_len: usize,
) -> YewLynxMountResult {
    let mounted = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: This forwards the caller obligations documented on the exported function.
        let root_id = unsafe { copy_bytes(root_id, root_id_len) }?;
        mount(parse_decimal_id(&root_id, "root ID")?)
    }));
    match mounted {
        Ok(Ok((session, response))) => YewLynxMountResult {
            session,
            response: YewLynxBuffer::from_vec(response_json(Ok(response))),
        },
        Ok(Err(error)) => YewLynxMountResult {
            session: 0,
            response: YewLynxBuffer::from_vec(response_json(Err(error))),
        },
        Err(payload) => YewLynxMountResult {
            session: 0,
            response: YewLynxBuffer::from_vec(response_json(Err(ApiError::new(
                YEW_LYNX_COUNTER_STATUS_PANIC,
                panic_message(payload.as_ref()),
            )))),
        },
    }
}

/// Dispatches an event using an unsigned decimal UTF-8 listener ID.
///
/// # Safety
///
/// Each nonempty byte span must remain readable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yew_lynx_dispatch(
    session: YewLynxSession,
    listener_id: *const u8,
    listener_id_len: usize,
    event_name: *const u8,
    event_name_len: usize,
) -> YewLynxBuffer {
    let dispatched = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: These forward the caller obligations documented on the exported function.
        let listener_id = unsafe { copy_bytes(listener_id, listener_id_len) }?;
        // SAFETY: These forward the caller obligations documented on the exported function.
        let event_name = unsafe { copy_bytes(event_name, event_name_len) }?;
        let event_name = std::str::from_utf8(&event_name).map_err(|error| {
            ApiError::new(
                YEW_LYNX_COUNTER_STATUS_INVALID_UTF8,
                format!("event name is not UTF-8: {error}"),
            )
        })?;
        dispatch(
            session,
            parse_decimal_id(&listener_id, "listener ID")?,
            event_name,
        )
    }));
    let result = match dispatched {
        Ok(result) => result,
        Err(payload) => {
            poison_session_after_boundary_panic(session);
            Err(ApiError::new(
                YEW_LYNX_COUNTER_STATUS_PANIC,
                panic_message(payload.as_ref()),
            ))
        }
    };
    YewLynxBuffer::from_vec(response_json(result))
}

/// Destroys a session. `consumed` is one only after owner-thread validation removed the token.
#[unsafe(no_mangle)]
pub extern "C" fn yew_lynx_destroy(session: YewLynxSession) -> YewLynxDestroyResult {
    let mut consumed = false;
    let destroyed = catch_unwind(AssertUnwindSafe(|| destroy(session, &mut consumed)));
    let result = match destroyed {
        Ok(result) => result,
        Err(payload) => Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_PANIC,
            panic_message(payload.as_ref()),
        )),
    };
    YewLynxDestroyResult {
        consumed: u32::from(consumed),
        response: YewLynxBuffer::from_vec(response_json(result)),
    }
}

/// Frees one buffer returned by this library.
///
/// # Safety
///
/// `buffer` must be empty or an unmodified, not-yet-freed buffer returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yew_lynx_buffer_free(buffer: YewLynxBuffer) {
    if buffer.data.is_null() {
        return;
    }
    if buffer.len == 0 || buffer.len > isize::MAX as usize {
        return;
    }
    let slice = ptr::slice_from_raw_parts_mut(buffer.data, buffer.len);
    // SAFETY: The C contract transfers back the exact boxed slice returned by this library.
    drop(unsafe { Box::from_raw(slice) });
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use yew::NativeRendererBackend;
    use yew_lynx_adapter::FiberMutation;

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

    fn mount_root(root: u64) -> YewLynxMountResult {
        let root = root.to_string();
        // SAFETY: `root` remains readable for the duration of the call.
        unsafe { yew_lynx_mount(root.as_ptr(), root.len()) }
    }

    fn success_operations(json: &[u8]) -> Vec<FiberMutation> {
        match ProtocolResponse::from_json(json).unwrap() {
            ProtocolResponse::Success(response) => response.operations,
            ProtocolResponse::Failure(response) => panic!("unexpected failure: {response:?}"),
        }
    }

    fn failure(json: &[u8]) -> FailureResponse {
        match ProtocolResponse::from_json(json).unwrap() {
            ProtocolResponse::Success(response) => panic!("unexpected success: {response:?}"),
            ProtocolResponse::Failure(response) => response,
        }
    }

    fn listener_id(operations: &[FiberMutation]) -> u64 {
        operations
            .iter()
            .find_map(|operation| match operation {
                FiberMutation::AddEventListener { listener, .. } => Some(*listener),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn mount_dispatch_and_destroy_emit_enveloped_counter_responses() {
        let root = 1;
        let mounted = mount_root(root);
        assert_ne!(mounted.session, 0);
        let session = mounted.session;
        let initial = success_operations(&copy_and_free(mounted.response));
        assert!(initial.iter().any(|operation| matches!(
            operation,
            FiberMutation::CreateText { text, .. } if text == "Count: 0"
        )));
        assert_eq!(initial.last(), Some(&FiberMutation::Flush { root }));
        let first_listener = listener_id(&initial);
        let first_listener_id = first_listener.to_string();
        SESSIONS.with(|sessions| {
            let sessions = sessions.sessions.borrow();
            let backend = &sessions.get(&session).unwrap().backend;
            backend.flush(NativeNode(root));
            backend.flush(NativeNode(root));
        });

        let event = b"tap";
        // SAFETY: All byte spans remain readable and `session` is a live integer token.
        let update = success_operations(&copy_and_free(unsafe {
            yew_lynx_dispatch(
                session,
                first_listener_id.as_ptr(),
                first_listener_id.len(),
                event.as_ptr(),
                event.len(),
            )
        }));
        assert!(update.iter().any(|operation| matches!(
            operation,
            FiberMutation::CreateText { text, .. } if text == "Count: 1"
        )));
        assert_eq!(
            update
                .iter()
                .filter(|operation| matches!(operation, FiberMutation::Flush { .. }))
                .count(),
            1
        );
        let current_listener = listener_id(&update);

        // SAFETY: A stale listener ID is data, and all byte spans remain readable.
        let stale_listener = failure(&copy_and_free(unsafe {
            yew_lynx_dispatch(
                session,
                first_listener_id.as_ptr(),
                first_listener_id.len(),
                event.as_ptr(),
                event.len(),
            )
        }));
        assert_eq!(
            stale_listener.status,
            YEW_LYNX_COUNTER_STATUS_INVALID_LISTENER
        );
        assert!(stale_listener.operations.is_empty());

        let teardown = yew_lynx_destroy(session);
        assert_eq!(teardown.consumed, 1);
        let teardown = success_operations(&copy_and_free(teardown.response));
        assert!(teardown.iter().any(|operation| matches!(
            operation,
            FiberMutation::RemoveEventListener { listener, .. } if *listener == current_listener
        )));
        assert!(teardown.iter().any(
            |operation| matches!(operation, FiberMutation::Remove { parent, .. } if *parent == root)
        ));
        assert_eq!(teardown.last(), Some(&FiberMutation::Flush { root }));

        let stale = yew_lynx_destroy(session);
        assert_eq!(stale.consumed, 0);
        assert_eq!(
            failure(&copy_and_free(stale.response)).status,
            YEW_LYNX_COUNTER_STATUS_INVALID_SESSION
        );
    }

    #[test]
    fn ids_at_the_limit_work_and_ids_over_the_limit_are_rejected() {
        let invalid_session = yew_lynx_destroy(JS_MAX_SAFE_INTEGER + 1);
        assert_eq!(invalid_session.consumed, 0);
        assert_eq!(
            failure(&copy_and_free(invalid_session.response)).status,
            YEW_LYNX_COUNTER_STATUS_INVALID_SESSION
        );

        let invalid_root = mount_root(0);
        assert_eq!(invalid_root.session, 0);
        assert_eq!(
            failure(&copy_and_free(invalid_root.response)).status,
            YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT
        );

        let over_max = mount_root(JS_MAX_SAFE_INTEGER + 1);
        assert_eq!(over_max.session, 0);
        assert_eq!(
            failure(&copy_and_free(over_max.response)).status,
            YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT
        );

        let mounted = mount_root(JS_MAX_SAFE_INTEGER);
        assert_ne!(mounted.session, 0);
        let session = mounted.session;
        let initial = success_operations(&copy_and_free(mounted.response));
        assert!(
            initial
                .iter()
                .any(|operation| matches!(operation, FiberMutation::CreateElement { node: 1, .. }))
        );
        let listener = listener_id(&initial);
        let invalid_listener = (JS_MAX_SAFE_INTEGER + 1).to_string();
        let event = b"tap";
        // SAFETY: All byte spans remain readable and `session` is live.
        let response = failure(&copy_and_free(unsafe {
            yew_lynx_dispatch(
                session,
                invalid_listener.as_ptr(),
                invalid_listener.len(),
                event.as_ptr(),
                event.len(),
            )
        }));
        assert_eq!(response.status, YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT);

        let listener = listener.to_string();
        let wrong_event = b"click";
        // SAFETY: All byte spans remain readable and `session` is live.
        let response = failure(&copy_and_free(unsafe {
            yew_lynx_dispatch(
                session,
                listener.as_ptr(),
                listener.len(),
                wrong_event.as_ptr(),
                wrong_event.len(),
            )
        }));
        assert_eq!(response.status, YEW_LYNX_COUNTER_STATUS_EVENT_MISMATCH);
        let destroyed = yew_lynx_destroy(session);
        assert_eq!(destroyed.consumed, 1);
        success_operations(&copy_and_free(destroyed.response));
    }

    #[test]
    fn wrong_thread_destroy_does_not_consume_and_owner_can_retry() {
        let mounted = mount_root(1);
        let session = mounted.session;
        success_operations(&copy_and_free(mounted.response));

        let wrong_thread = std::thread::spawn(move || {
            let destroyed = yew_lynx_destroy(session);
            (
                destroyed.consumed,
                failure(&copy_and_free(destroyed.response)).status,
            )
        })
        .join()
        .unwrap();
        assert_eq!(wrong_thread, (0, YEW_LYNX_COUNTER_STATUS_WRONG_THREAD));

        let destroyed = yew_lynx_destroy(session);
        assert_eq!(destroyed.consumed, 1);
        success_operations(&copy_and_free(destroyed.response));
    }

    #[test]
    fn separate_owner_contexts_can_mount_root_one_concurrently() {
        let barrier = Arc::new(Barrier::new(2));
        let threads: Vec<_> = (0..2)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    let mounted = mount_root(1);
                    assert_ne!(mounted.session, 0);
                    success_operations(&copy_and_free(mounted.response));
                    barrier.wait();
                    let destroyed = yew_lynx_destroy(mounted.session);
                    assert_eq!(destroyed.consumed, 1);
                    success_operations(&copy_and_free(destroyed.response));
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
    }

    #[test]
    fn thread_exit_releases_owner_bookkeeping_but_not_host_cleanup() {
        let session = std::thread::spawn(|| {
            let mounted = mount_root(1);
            success_operations(&copy_and_free(mounted.response));
            mounted.session
        })
        .join()
        .unwrap();

        assert!(!lock_owners().contains_key(&session));
        let destroyed = yew_lynx_destroy(session);
        assert_eq!(destroyed.consumed, 0);
        assert_eq!(
            failure(&copy_and_free(destroyed.response)).status,
            YEW_LYNX_COUNTER_STATUS_INVALID_SESSION
        );
    }

    #[test]
    fn backend_errors_permanently_poison_the_session() {
        let mounted = mount_root(1);
        let session = mounted.session;
        let initial = success_operations(&copy_and_free(mounted.response));
        let listener = listener_id(&initial).to_string();
        SESSIONS.with(|sessions| {
            sessions
                .sessions
                .borrow()
                .get(&session)
                .unwrap()
                .backend
                .insert_before(NativeNode(1), NativeNode(999_999), None);
        });

        let event = b"tap";
        // SAFETY: All byte spans remain readable and `session` is live.
        let first = failure(&copy_and_free(unsafe {
            yew_lynx_dispatch(
                session,
                listener.as_ptr(),
                listener.len(),
                event.as_ptr(),
                event.len(),
            )
        }));
        assert_eq!(first.status, YEW_LYNX_COUNTER_STATUS_BACKEND_ERROR);
        assert!(first.operations.is_empty());

        // SAFETY: The session remains allocated but is permanently poisoned.
        let second = failure(&copy_and_free(unsafe {
            yew_lynx_dispatch(
                session,
                listener.as_ptr(),
                listener.len(),
                event.as_ptr(),
                event.len(),
            )
        }));
        assert_eq!(second.status, YEW_LYNX_COUNTER_STATUS_SESSION_POISONED);
        assert!(second.operations.is_empty());

        let destroyed = yew_lynx_destroy(session);
        assert_eq!(destroyed.consumed, 1);
        let destroyed = failure(&copy_and_free(destroyed.response));
        assert_eq!(destroyed.status, YEW_LYNX_COUNTER_STATUS_SESSION_POISONED);
        assert!(destroyed.operations.is_empty());
    }

    #[test]
    fn invalid_spans_and_panics_return_exact_failure_envelopes() {
        let mounted = mount_root(1);
        let session = mounted.session;
        let listener =
            listener_id(&success_operations(&copy_and_free(mounted.response))).to_string();

        // SAFETY: A null pointer with nonzero length is intentionally rejected before reading.
        let invalid = failure(&copy_and_free(unsafe {
            yew_lynx_dispatch(session, listener.as_ptr(), listener.len(), ptr::null(), 1)
        }));
        assert_eq!(invalid.status, YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT);
        assert!(invalid.operations.is_empty());

        let panic = failure(&copy_and_free(response_boundary(|| {
            panic!("contained panic")
        })));
        assert_eq!(panic.status, YEW_LYNX_COUNTER_STATUS_PANIC);
        assert_eq!(panic.error, "contained panic");
        assert!(panic.operations.is_empty());

        let destroyed = yew_lynx_destroy(session);
        assert_eq!(destroyed.consumed, 1);
        success_operations(&copy_and_free(destroyed.response));
    }
}
