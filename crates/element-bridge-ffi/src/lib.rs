#![deny(unsafe_code)]

pub mod native_host;

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread::{self, ThreadId};

use lynx_element_bridge_core::{
    BridgeError, CommandBatch, EventMessage, NodeId, SessionId, Status,
};

use native_host::{
    NATIVE_STATUS_OK, NATIVE_STATUS_PANIC, NATIVE_STATUS_UNSUPPORTED, NativeBytes,
    NativeCallbackHandle, NativeHost, NativeHostHandle, NativeListenerHandle,
    NativeRendererCallbacksV1, NativeRendererGetApiFn, NativeRendererHandle, NativeStatus,
    NativeTimerHandle, NativeUtf8, status_to_native,
};

pub type LynxElementBridgeSession = u32;
pub const NATIVE_BRIDGE_ROOT_ID: u32 = 1;

#[repr(C)]
pub struct LynxElementBridgeNativeMountResult {
    pub status: NativeStatus,
    pub session: LynxElementBridgeSession,
}

#[repr(C)]
pub struct LynxElementBridgeNativeDestroyResult {
    pub status: NativeStatus,
    pub consumed: u32,
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

pub trait BridgeBackendCandidate {
    fn mount(&mut self, session: SessionId, root: NodeId) -> Result<CommandBatch, BackendError>;

    fn activate(self: Box<Self>) -> Box<dyn BridgeBackend>;
}

struct Session {
    backend: Option<Box<dyn BridgeBackend>>,
    native_host: NativeHost,
    poisoned: bool,
}

enum SessionState {
    Ready(Box<Session>),
    Busy,
}

impl Session {
    fn poison(&mut self) {
        self.poisoned = true;
        if let Some(backend) = self.backend.as_mut() {
            backend.discard_pending();
        }
    }
}

#[derive(Clone, Debug)]
struct SessionOwner {
    thread_id: ThreadId,
}

struct ThreadSessions {
    sessions: RefCell<HashMap<LynxElementBridgeSession, SessionState>>,
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
        for state in sessions.values_mut() {
            if let SessionState::Ready(session) = state {
                if let Some(backend) = session.backend.as_mut() {
                    backend.abandon();
                }
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

fn busy_session(session_id: LynxElementBridgeSession) -> BackendError {
    BackendError::recoverable(
        Status::HostError,
        format!("session {session_id} is already executing a callback"),
    )
}

fn take_ready_session(session_id: LynxElementBridgeSession) -> Result<Box<Session>, BackendError> {
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.sessions.try_borrow_mut().map_err(|_| {
            BackendError::fatal(
                Status::InternalError,
                "session registry is already borrowed",
            )
        })?;
        let state = sessions
            .get_mut(&session_id)
            .ok_or_else(|| invalid_session(session_id))?;
        if matches!(state, SessionState::Busy) {
            return Err(busy_session(session_id));
        }
        let SessionState::Ready(session) = std::mem::replace(state, SessionState::Busy) else {
            unreachable!("the busy state returned above")
        };
        Ok(session)
    })
}

fn restore_session(
    session_id: LynxElementBridgeSession,
    session: Box<Session>,
) -> Result<(), BackendError> {
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.sessions.try_borrow_mut().map_err(|_| {
            BackendError::fatal(
                Status::InternalError,
                "session registry is already borrowed",
            )
        })?;
        let state = sessions
            .get_mut(&session_id)
            .ok_or_else(|| invalid_session(session_id))?;
        if !matches!(state, SessionState::Busy) {
            return Err(BackendError::fatal(
                Status::InternalError,
                format!("session {session_id} lost its busy state"),
            ));
        }
        *state = SessionState::Ready(session);
        Ok(())
    })
}

fn with_ready_session<T>(
    session_id: LynxElementBridgeSession,
    operation: impl FnOnce(&mut Session) -> Result<T, BackendError>,
) -> Result<T, BackendError> {
    validate_session_owner(session_id)?;
    let mut session = take_ready_session(session_id)?;
    let result = match catch_unwind(AssertUnwindSafe(|| operation(&mut session))) {
        Ok(result) => result,
        Err(payload) => {
            session.poison();
            Err(BackendError::recoverable(
                Status::Panic,
                panic_message(payload.as_ref()),
            ))
        }
    };
    restore_session(session_id, session)?;
    result
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

#[allow(unsafe_code)]
unsafe fn native_mount_internal(
    get_api: Option<NativeRendererGetApiFn>,
    host: NativeHostHandle,
    create_backend: impl FnOnce(
        SessionId,
        NodeId,
    ) -> Result<(Box<dyn BridgeBackend>, CommandBatch), BackendError>,
) -> Result<LynxElementBridgeSession, BackendError> {
    let get_api = get_api.ok_or_else(|| {
        BackendError::recoverable(
            Status::InvalidArgument,
            "native renderer API resolver must not be null",
        )
    })?;
    if host == 0 {
        return Err(BackendError::recoverable(
            Status::InvalidArgument,
            "native host handle must not be zero",
        ));
    }

    let session_id = next_session_id()?;
    let reservation = SessionReservation::new(session_id);
    let session = SessionId::new(session_id).map_err(BackendError::from)?;
    let root = NodeId::new(NATIVE_BRIDGE_ROOT_ID).map_err(BackendError::from)?;
    let (mut backend, initial_batch) = create_backend(session, root)?;
    let callbacks = NativeRendererCallbacksV1 {
        context: session_id as usize as *mut c_void,
        on_event: Some(native_on_event),
        on_timer: Some(native_on_timer),
    };
    // SAFETY: The caller guarantees that the resolver and host follow the native renderer ABI.
    let mut native_host =
        match unsafe { NativeHost::acquire(get_api, host, session, root, callbacks) } {
            Ok(host) => host,
            Err(error) => {
                backend.abandon();
                return Err(BackendError::from(error));
            }
        };
    if let Err(error) = native_host.apply(&initial_batch) {
        backend.abandon();
        return Err(BackendError::from(error));
    }
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
            SessionState::Ready(Box::new(Session {
                backend: Some(backend),
                native_host,
                poisoned: false,
            })),
        );
        Ok(())
    })?;
    reservation.commit();
    Ok(session_id)
}

/// Mounts a backend directly into a native renderer using bridge root `NodeId(1)`.
///
/// # Safety
///
/// A non-null `get_api` resolver and `host` must obey the contract declared by the native
/// renderer C ABI for the lifetime of the mounted session.
#[allow(unsafe_code)]
pub unsafe fn native_mount(
    get_api: Option<NativeRendererGetApiFn>,
    host: NativeHostHandle,
    create_backend: impl FnOnce(
        SessionId,
        NodeId,
    ) -> Result<(Box<dyn BridgeBackend>, CommandBatch), BackendError>,
) -> LynxElementBridgeNativeMountResult {
    let mounted = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: This forwards the caller obligations documented on this function.
        unsafe { native_mount_internal(get_api, host, create_backend) }
    }));
    match mounted {
        Ok(Ok(session)) => LynxElementBridgeNativeMountResult {
            status: NATIVE_STATUS_OK,
            session,
        },
        Ok(Err(error)) => LynxElementBridgeNativeMountResult {
            status: status_to_native(error.status),
            session: 0,
        },
        Err(_) => LynxElementBridgeNativeMountResult {
            status: NATIVE_STATUS_PANIC,
            session: 0,
        },
    }
}

fn callback_session(context: *mut c_void) -> Result<LynxElementBridgeSession, BackendError> {
    let session = context as usize;
    if session == 0 || session > u32::MAX as usize {
        return Err(BackendError::recoverable(
            Status::InvalidSession,
            "native callback context does not identify a session",
        ));
    }
    Ok(session as u32)
}

#[allow(unsafe_code)]
unsafe fn copy_native_span(data: *const u8, len: usize) -> Result<Vec<u8>, BackendError> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if data.is_null() || len > isize::MAX as usize {
        return Err(BackendError::recoverable(
            Status::InvalidArgument,
            "native callback span is invalid",
        ));
    }
    // SAFETY: The callback contract requires a readable borrowed span for this call.
    Ok(unsafe { std::slice::from_raw_parts(data, len) }.to_vec())
}

#[allow(unsafe_code)]
unsafe fn copy_native_utf8(span: NativeUtf8) -> Result<String, BackendError> {
    // SAFETY: This forwards the callback span contract.
    let bytes = unsafe { copy_native_span(span.data, span.len) }?;
    String::from_utf8(bytes).map_err(|_| {
        BackendError::recoverable(Status::InvalidArgument, "native callback UTF-8 is invalid")
    })
}

fn dispatch_native_event(
    session_id: LynxElementBridgeSession,
    renderer: NativeRendererHandle,
    listener: NativeListenerHandle,
    callback: NativeCallbackHandle,
    name: String,
    content_type: String,
    payload: Vec<u8>,
) -> Result<(), BackendError> {
    with_ready_session(session_id, |session| {
        if session.poisoned {
            return Err(BackendError::recoverable(
                Status::HostError,
                format!("session {session_id} is permanently poisoned"),
            ));
        }
        let event = session
            .native_host
            .event_message(renderer, listener, callback, &name, content_type, payload)
            .map_err(|error| BackendError::recoverable(error.status, error.message))?;
        let dispatched = session
            .backend
            .as_mut()
            .ok_or_else(|| {
                BackendError::fatal(
                    Status::InternalError,
                    format!("session {session_id} has no backend"),
                )
            })?
            .dispatch_event(event);
        let batch = match dispatched {
            Ok(batch) => batch,
            Err(error) => {
                if error.poison_session {
                    session.poison();
                }
                return Err(error);
            }
        };
        match session.native_host.apply(&batch) {
            Ok(()) => Ok(()),
            Err(error) => {
                session.poison();
                Err(BackendError::from(error))
            }
        }
    })
}

#[allow(unsafe_code)]
unsafe extern "C" fn native_on_event(
    context: *mut c_void,
    renderer: NativeRendererHandle,
    listener: NativeListenerHandle,
    callback: NativeCallbackHandle,
    name: NativeUtf8,
    content_type: NativeUtf8,
    payload: NativeBytes,
) -> NativeStatus {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let session = callback_session(context)?;
        validate_session_owner(session)?;
        // SAFETY: These forward the borrowed callback span contracts.
        let name = unsafe { copy_native_utf8(name) }?;
        // SAFETY: This forwards the borrowed callback span contract.
        let content_type = unsafe { copy_native_utf8(content_type) }?;
        // SAFETY: This forwards the borrowed callback span contract.
        let payload = unsafe { copy_native_span(payload.data, payload.len) }?;
        dispatch_native_event(
            session,
            renderer,
            listener,
            callback,
            name,
            content_type,
            payload,
        )
    }));
    match result {
        Ok(Ok(())) => NATIVE_STATUS_OK,
        Ok(Err(error)) => status_to_native(error.status),
        Err(_) => NATIVE_STATUS_PANIC,
    }
}

#[allow(unsafe_code)]
unsafe extern "C" fn native_on_timer(
    _: *mut c_void,
    _: NativeRendererHandle,
    _: NativeTimerHandle,
    _: NativeCallbackHandle,
) -> NativeStatus {
    NATIVE_STATUS_UNSUPPORTED
}

fn remove_ready_session(
    session_id: LynxElementBridgeSession,
) -> Result<Box<Session>, BackendError> {
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.sessions.try_borrow_mut().map_err(|_| {
            BackendError::fatal(
                Status::InternalError,
                "session registry is already borrowed",
            )
        })?;
        let state = sessions
            .get(&session_id)
            .ok_or_else(|| invalid_session(session_id))?;
        if matches!(state, SessionState::Busy) {
            return Err(busy_session(session_id));
        }
        let Some(SessionState::Ready(session)) = sessions.remove(&session_id) else {
            unreachable!("the ready state was checked above")
        };
        Ok(session)
    })
}

fn native_destroy_internal(
    session_id: LynxElementBridgeSession,
    consumed: &mut bool,
) -> Result<(), BackendError> {
    validate_session_owner(session_id)?;
    let mut session = remove_ready_session(session_id)?;
    lock_owners().remove(&session_id);
    *consumed = true;

    let was_poisoned = session.poisoned;
    let work = catch_unwind(AssertUnwindSafe(|| {
        let backend = session.backend.take().ok_or_else(|| {
            BackendError::fatal(
                Status::InternalError,
                format!("session {session_id} has no backend"),
            )
        })?;
        let destroyed = backend.destroy(was_poisoned);
        if was_poisoned {
            return Err(BackendError::recoverable(
                Status::HostError,
                format!("session {session_id} was destroyed after becoming permanently poisoned"),
            ));
        }
        session
            .native_host
            .apply(&destroyed?)
            .map_err(BackendError::from)
    }));
    let work = match work {
        Ok(result) => result,
        Err(payload) => Err(BackendError::recoverable(
            Status::Panic,
            panic_message(payload.as_ref()),
        )),
    };
    let released = session.native_host.release().map_err(BackendError::from);
    match (work, released) {
        (Err(error), _) | (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

pub fn native_destroy_session(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    let mut consumed = false;
    let destroyed = catch_unwind(AssertUnwindSafe(|| {
        native_destroy_internal(session, &mut consumed)
    }));
    let status = match destroyed {
        Ok(Ok(())) => NATIVE_STATUS_OK,
        Ok(Err(error)) => status_to_native(error.status),
        Err(_) => NATIVE_STATUS_PANIC,
    };
    LynxElementBridgeNativeDestroyResult {
        status,
        consumed: u32::from(consumed),
    }
}

/// Replaces a mounted backend while retaining the acquired native renderer.
///
/// Candidate construction and validation happen before the active backend is destroyed. Once the
/// old teardown batch has been applied, the native host starts a fresh command-sequence epoch.
pub fn native_replace_backend(
    session_id: LynxElementBridgeSession,
    preflight_candidate: impl FnOnce() -> Result<Box<dyn BridgeBackendCandidate>, BackendError>,
) -> NativeStatus {
    let replaced = catch_unwind(AssertUnwindSafe(|| {
        with_ready_session(session_id, |session| {
            if session.poisoned {
                return Err(BackendError::recoverable(
                    Status::HostError,
                    format!("session {session_id} is permanently poisoned"),
                ));
            }

            let bridge_session = SessionId::new(session_id).map_err(BackendError::from)?;
            let root = NodeId::new(NATIVE_BRIDGE_ROOT_ID).map_err(BackendError::from)?;
            let mut candidate = preflight_candidate()?;

            let old = session.backend.take().ok_or_else(|| {
                BackendError::fatal(
                    Status::InternalError,
                    format!("session {session_id} has no backend"),
                )
            })?;
            let teardown = match catch_unwind(AssertUnwindSafe(|| old.destroy(false))) {
                Ok(Ok(batch)) => batch,
                Ok(Err(error)) => {
                    session.backend = Some(candidate.activate());
                    session.poison();
                    return Err(error);
                }
                Err(payload) => {
                    session.backend = Some(candidate.activate());
                    session.poison();
                    return Err(BackendError::recoverable(
                        Status::Panic,
                        panic_message(payload.as_ref()),
                    ));
                }
            };
            if let Err(error) = session.native_host.apply(&teardown) {
                session.backend = Some(candidate.activate());
                session.poison();
                return Err(BackendError::from(error));
            }
            if let Err(error) = session.native_host.reset_application_epoch() {
                session.backend = Some(candidate.activate());
                session.poison();
                return Err(BackendError::from(error));
            }

            let initial_batch =
                match catch_unwind(AssertUnwindSafe(|| candidate.mount(bridge_session, root))) {
                    Ok(Ok(batch)) => batch,
                    Ok(Err(error)) => {
                        session.backend = Some(candidate.activate());
                        session.poison();
                        return Err(error);
                    }
                    Err(payload) => {
                        session.backend = Some(candidate.activate());
                        session.poison();
                        return Err(BackendError::recoverable(
                            Status::Panic,
                            panic_message(payload.as_ref()),
                        ));
                    }
                };
            session.backend = Some(candidate.activate());
            if let Err(error) = session.native_host.apply(&initial_batch) {
                session.poison();
                return Err(BackendError::from(error));
            }
            Ok(())
        })
    }));
    match replaced {
        Ok(Ok(())) => NATIVE_STATUS_OK,
        Ok(Err(error)) => status_to_native(error.status),
        Err(_) => NATIVE_STATUS_PANIC,
    }
}

fn native_abandon_internal(
    session_id: LynxElementBridgeSession,
    consumed: &mut bool,
) -> Result<(), BackendError> {
    validate_session_owner(session_id)?;
    let mut session = remove_ready_session(session_id)?;
    lock_owners().remove(&session_id);
    *consumed = true;

    let mut first_error = None;
    match session.backend.take() {
        Some(mut backend) => {
            if let Err(payload) = catch_unwind(AssertUnwindSafe(|| backend.abandon())) {
                first_error = Some(BackendError::recoverable(
                    Status::Panic,
                    panic_message(payload.as_ref()),
                ));
            }
            if let Err(payload) = catch_unwind(AssertUnwindSafe(|| backend.discard_pending())) {
                if first_error.is_none() {
                    first_error = Some(BackendError::recoverable(
                        Status::Panic,
                        panic_message(payload.as_ref()),
                    ));
                }
            }
            if let Err(payload) = catch_unwind(AssertUnwindSafe(|| drop(backend))) {
                if first_error.is_none() {
                    first_error = Some(BackendError::recoverable(
                        Status::Panic,
                        panic_message(payload.as_ref()),
                    ));
                }
            }
        }
        None => {
            first_error = Some(BackendError::fatal(
                Status::InternalError,
                format!("session {session_id} has no backend"),
            ));
        }
    }

    let released = match catch_unwind(AssertUnwindSafe(|| session.native_host.release())) {
        Ok(result) => result.map_err(BackendError::from),
        Err(payload) => Err(BackendError::recoverable(
            Status::Panic,
            panic_message(payload.as_ref()),
        )),
    };
    match (first_error, released) {
        (Some(error), _) | (None, Err(error)) => Err(error),
        (None, Ok(())) => Ok(()),
    }
}

/// Emergency owner-thread cleanup for a native session whose normal destroy could not consume it.
///
/// This consumes the session without producing or applying a teardown command batch.
pub fn native_abandon_session(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    let mut consumed = false;
    let abandoned = catch_unwind(AssertUnwindSafe(|| {
        native_abandon_internal(session, &mut consumed)
    }));
    let status = match abandoned {
        Ok(Ok(())) => NATIVE_STATUS_OK,
        Ok(Err(error)) => status_to_native(error.status),
        Err(_) => NATIVE_STATUS_PANIC,
    };
    LynxElementBridgeNativeDestroyResult {
        status,
        consumed: u32::from(consumed),
    }
}
