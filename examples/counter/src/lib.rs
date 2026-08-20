//! Counter static library backed by Lynx Element Bridge protocol v2.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread::{self, ThreadId};

use lynx_element_bridge_core::{
    BridgeError, CommandBatch, EventMessage, NodeId, ResponseBatch, SessionId, Status,
};
use lynx_element_bridge_wire::{
    decode_event, decode_response, encode_command_batch, encode_failure,
};
use lynx_element_bridge_yew::{YewAdapter, YewAdapterError};
use yew::prelude::*;
use yew::{NativeAppHandle, NativeEvent, NativeNode, NativeRenderer};

pub const YEW_LYNX_COUNTER_STATUS_OK: u32 = 0;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT: u32 = 1;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_SESSION: u32 = 2;
pub const YEW_LYNX_COUNTER_STATUS_WRONG_THREAD: u32 = 3;
pub const YEW_LYNX_COUNTER_STATUS_UNSUPPORTED: u32 = 4;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_OWNERSHIP: u32 = 5;
pub const YEW_LYNX_COUNTER_STATUS_INVALID_LISTENER: u32 = 6;
pub const YEW_LYNX_COUNTER_STATUS_RESOURCE_EXHAUSTED: u32 = 7;
pub const YEW_LYNX_COUNTER_STATUS_HOST_ERROR: u32 = 8;
pub const YEW_LYNX_COUNTER_STATUS_PANIC: u32 = 9;
pub const YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR: u32 = 10;

#[function_component(Counter)]
pub fn counter() -> Html {
    let count = use_state(|| 0);
    let increment = {
        let count = count.clone();
        Callback::from(move |_: NativeEvent| count.set(*count + 1))
    };

    html! {
        <view style="height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center;">
            <text id="counter-value" style="font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 32px;">{format!("Count: {}", *count)}</text>
            <view id="counter-increment" style="height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;" ontap={increment}>
                <text style="font-size: 28px; font-weight: 600; color: #ffffff;">{"Increment"}</text>
            </view>
        </view>
    }
}

pub type YewLynxSession = u32;

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
    backend: Rc<YewAdapter>,
    app: Option<NativeAppHandle<Counter>>,
    poisoned: bool,
    last_response: Option<ResponseBatch>,
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

static NEXT_SESSION_ID: AtomicU32 = AtomicU32::new(1);
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

impl From<BridgeError> for ApiError {
    fn from(error: BridgeError) -> Self {
        Self::new(status_code(error.status), error.to_string())
    }
}

impl From<YewAdapterError> for ApiError {
    fn from(error: YewAdapterError) -> Self {
        let status = match &error {
            YewAdapterError::Bridge(error) => status_code(error.status),
            YewAdapterError::InvalidListener(_) | YewAdapterError::EventMismatch { .. } => {
                YEW_LYNX_COUNTER_STATUS_INVALID_LISTENER
            }
            YewAdapterError::CallbackExhausted => YEW_LYNX_COUNTER_STATUS_RESOURCE_EXHAUSTED,
            YewAdapterError::InvalidNode(_) | YewAdapterError::Borrowed(_) => {
                YEW_LYNX_COUNTER_STATUS_HOST_ERROR
            }
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

fn status_code(status: Status) -> u32 {
    match status {
        Status::Ok => YEW_LYNX_COUNTER_STATUS_OK,
        Status::InvalidArgument => YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT,
        Status::InvalidSession => YEW_LYNX_COUNTER_STATUS_INVALID_SESSION,
        Status::WrongThread => YEW_LYNX_COUNTER_STATUS_WRONG_THREAD,
        Status::Unsupported => YEW_LYNX_COUNTER_STATUS_UNSUPPORTED,
        Status::InvalidOwnership => YEW_LYNX_COUNTER_STATUS_INVALID_OWNERSHIP,
        Status::InvalidListener => YEW_LYNX_COUNTER_STATUS_INVALID_LISTENER,
        Status::ResourceExhausted => YEW_LYNX_COUNTER_STATUS_RESOURCE_EXHAUSTED,
        Status::HostError => YEW_LYNX_COUNTER_STATUS_HOST_ERROR,
        Status::Panic => YEW_LYNX_COUNTER_STATUS_PANIC,
        Status::InternalError => YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR,
    }
}

fn core_status(status: u32) -> Status {
    match status {
        YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT => Status::InvalidArgument,
        YEW_LYNX_COUNTER_STATUS_INVALID_SESSION => Status::InvalidSession,
        YEW_LYNX_COUNTER_STATUS_WRONG_THREAD => Status::WrongThread,
        YEW_LYNX_COUNTER_STATUS_UNSUPPORTED => Status::Unsupported,
        YEW_LYNX_COUNTER_STATUS_INVALID_OWNERSHIP => Status::InvalidOwnership,
        YEW_LYNX_COUNTER_STATUS_INVALID_LISTENER => Status::InvalidListener,
        YEW_LYNX_COUNTER_STATUS_RESOURCE_EXHAUSTED => Status::ResourceExhausted,
        YEW_LYNX_COUNTER_STATUS_HOST_ERROR => Status::HostError,
        YEW_LYNX_COUNTER_STATUS_PANIC => Status::Panic,
        _ => Status::InternalError,
    }
}

fn next_session_id() -> Result<YewLynxSession, ApiError> {
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
            ApiError::new(
                YEW_LYNX_COUNTER_STATUS_RESOURCE_EXHAUSTED,
                "session ID space is exhausted",
            )
        })
}

fn validate_session_owner(session_id: YewLynxSession) -> Result<(), ApiError> {
    if session_id == 0 {
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

fn mount(root_id: u32) -> Result<(YewLynxSession, CommandBatch), ApiError> {
    if root_id == 0 {
        return Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT,
            "root ID must not be zero",
        ));
    }

    let session_id = next_session_id()?;
    let reservation = SessionReservation::new(session_id);
    let backend = YewAdapter::new(SessionId::new(session_id)?, NodeId::new(root_id)?)?;
    let rendered = catch_unwind(AssertUnwindSafe({
        let backend = Rc::clone(&backend);
        move || NativeRenderer::<Counter>::new(backend, NativeNode(root_id.into())).render()
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
    let response = match backend.take_batch() {
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
                last_response: None,
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

fn dispatch(session_id: YewLynxSession, event: EventMessage) -> Result<CommandBatch, ApiError> {
    validate_session_owner(session_id)?;
    if event.session != SessionId::new(session_id)? {
        return Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_SESSION,
            "Event channel session does not match the active session",
        ));
    }
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
                YEW_LYNX_COUNTER_STATUS_HOST_ERROR,
                format!("session {session_id} is permanently poisoned"),
            ));
        }

        let dispatched = catch_unwind(AssertUnwindSafe(|| session.backend.dispatch_event(&event)));
        match dispatched {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                if !matches!(
                    error,
                    YewAdapterError::InvalidListener(_) | YewAdapterError::EventMismatch { .. }
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

        match session.backend.take_batch() {
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

fn complete(session_id: YewLynxSession, bytes: Vec<u8>) -> Result<Vec<u8>, ApiError> {
    validate_session_owner(session_id)?;
    let response = decode_response(&bytes).map_err(|error| {
        ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT,
            format!("invalid Result channel envelope: {error}"),
        )
    })?;
    if response.session != Some(SessionId::new(session_id)?) {
        return Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_SESSION,
            "Result channel session does not match the active session",
        ));
    }
    if !response.committed {
        return Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT,
            "Result channel response is not committed",
        ));
    }
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
        session.last_response = Some(response);
        Ok(bytes)
    })
}

fn destroy(session_id: YewLynxSession, consumed: &mut bool) -> Result<CommandBatch, ApiError> {
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
            YEW_LYNX_COUNTER_STATUS_HOST_ERROR,
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
    session.backend.destroy().map_err(Into::into)
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

fn fallback_internal_error() -> Vec<u8> {
    encode_failure(0, 0, Status::InternalError, "serialization failure")
}

fn response_wire(result: Result<CommandBatch, ApiError>, session: u32) -> Vec<u8> {
    match result {
        Ok(response) => {
            encode_command_batch(&response).unwrap_or_else(|_| fallback_internal_error())
        }
        Err(error) => encode_failure(session, 0, core_status(error.status), &error.message),
    }
}

#[cfg(test)]
fn response_boundary(operation: impl FnOnce() -> Result<CommandBatch, ApiError>) -> YewLynxBuffer {
    let result = match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(payload) => Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_PANIC,
            panic_message(payload.as_ref()),
        )),
    };
    YewLynxBuffer::from_vec(response_wire(result, 0))
}

/// Mounts a counter session using a nonzero protocol v2 root ID.
#[unsafe(no_mangle)]
pub extern "C" fn yew_lynx_mount(root_id: u32) -> YewLynxMountResult {
    let mounted = catch_unwind(AssertUnwindSafe(|| mount(root_id)));
    match mounted {
        Ok(Ok((session, response))) => YewLynxMountResult {
            session,
            response: YewLynxBuffer::from_vec(response_wire(Ok(response), session)),
        },
        Ok(Err(error)) => YewLynxMountResult {
            session: 0,
            response: YewLynxBuffer::from_vec(response_wire(Err(error), 0)),
        },
        Err(payload) => YewLynxMountResult {
            session: 0,
            response: YewLynxBuffer::from_vec(response_wire(
                Err(ApiError::new(
                    YEW_LYNX_COUNTER_STATUS_PANIC,
                    panic_message(payload.as_ref()),
                )),
                0,
            )),
        },
    }
}

/// Dispatches a protocol v2 Event channel envelope.
///
/// # Safety
///
/// Each nonempty byte span must remain readable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yew_lynx_dispatch(
    session: YewLynxSession,
    event: *const u8,
    event_len: usize,
) -> YewLynxBuffer {
    let dispatched = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: These forward the caller obligations documented on the exported function.
        let event = unsafe { copy_bytes(event, event_len) }?;
        let event = decode_event(&event).map_err(|error| {
            ApiError::new(
                YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT,
                format!("invalid Event channel envelope: {error}"),
            )
        })?;
        dispatch(session, event)
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
    YewLynxBuffer::from_vec(response_wire(result, session))
}

/// Accepts a synchronous Result channel response for a previously emitted batch.
///
/// # Safety
///
/// When `response_len` is nonzero, `response` must point to that many readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yew_lynx_complete(
    session: YewLynxSession,
    response: *const u8,
    response_len: usize,
) -> YewLynxBuffer {
    let completed = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: This forwards the caller obligations documented on the exported function.
        let response = unsafe { copy_bytes(response, response_len) }?;
        complete(session, response)
    }));
    let result = match completed {
        Ok(result) => result,
        Err(payload) => Err(ApiError::new(
            YEW_LYNX_COUNTER_STATUS_PANIC,
            panic_message(payload.as_ref()),
        )),
    };
    YewLynxBuffer::from_vec(match result {
        Ok(response) => response,
        Err(error) => encode_failure(session, 0, core_status(error.status), &error.message),
    })
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
        response: YewLynxBuffer::from_vec(response_wire(result, session)),
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
mod tests_v2;
