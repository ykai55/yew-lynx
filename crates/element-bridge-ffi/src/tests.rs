use std::cell::RefCell;
use std::ffi::c_void;
use std::mem;
use std::ptr;
use std::rc::Rc;

use lynx_element_bridge_core::{
    CallbackId, Command, CommandBatch, EventMessage, ListenerId, NodeId, Status,
};

use super::native_host::{
    NATIVE_RENDERER_ABI_VERSION, NATIVE_STATUS_HOST_ERROR, NATIVE_STATUS_INVALID_ARGUMENT,
    NATIVE_STATUS_INVALID_LISTENER, NATIVE_STATUS_INVALID_SESSION, NATIVE_STATUS_OK,
    NATIVE_STATUS_UNSUPPORTED, NATIVE_STATUS_WRONG_THREAD, NativeBytes, NativeCallbackHandle,
    NativeHostHandle, NativeListenerHandle, NativeNodeHandle, NativeRendererApiV1,
    NativeRendererCallbacksV1, NativeRendererHandle, NativeStatus, NativeTimerHandle, NativeUtf8,
};
use super::*;

const HOST: NativeHostHandle = 41;
const RENDERER: NativeRendererHandle = 73;
const ROOT: NativeNodeHandle = 101;

#[derive(Default)]
struct RendererRecorder {
    callbacks: Option<NativeRendererCallbacksV1>,
    listener: Option<(NativeListenerHandle, NativeCallbackHandle, Vec<u8>)>,
    failure: Option<&'static str>,
    flushes: usize,
    releases: usize,
    reenter_on_flush: bool,
    nested_status: Option<NativeStatus>,
    style_sheet: Vec<u8>,
    style_sheet_clears: usize,
}

#[derive(Default)]
struct BackendRecorder {
    dispatches: usize,
    destroys: usize,
    discards: usize,
    abandons: usize,
    destroyed_poisoned: bool,
    payload: Vec<u8>,
}

struct TestBackend {
    recorder: Rc<RefCell<BackendRecorder>>,
    sequence: u32,
}

impl TestBackend {
    fn new(recorder: Rc<RefCell<BackendRecorder>>) -> Self {
        Self {
            recorder,
            sequence: 1,
        }
    }

    fn batch(&mut self, commands: Vec<Command>) -> CommandBatch {
        let sequence = self.sequence;
        self.sequence += 1;
        CommandBatch {
            sequence,
            commands,
            final_commit: true,
        }
    }
}

impl BridgeBackend for TestBackend {
    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BackendError> {
        let mut recorder = self.recorder.borrow_mut();
        recorder.dispatches += 1;
        recorder.payload = event.payload;
        drop(recorder);
        Ok(self.batch(Vec::new()))
    }

    fn destroy(mut self: Box<Self>, poisoned: bool) -> Result<CommandBatch, BackendError> {
        let mut recorder = self.recorder.borrow_mut();
        recorder.destroys += 1;
        recorder.destroyed_poisoned = poisoned;
        drop(recorder);
        if poisoned {
            return Err(BackendError::recoverable(
                Status::HostError,
                "test backend is poisoned",
            ));
        }
        Ok(self.batch(vec![Command::RemoveEventListener {
            node: NodeId::new(1).unwrap(),
            listener: ListenerId::new(1).unwrap(),
            callback: CallbackId::new(1).unwrap(),
            name: "tap".into(),
        }]))
    }

    fn discard_pending(&mut self) {
        self.recorder.borrow_mut().discards += 1;
    }

    fn abandon(&mut self) {
        self.recorder.borrow_mut().abandons += 1;
    }
}

thread_local! {
    static RENDERER_RECORDER: RefCell<RendererRecorder> = RefCell::new(RendererRecorder::default());
}

fn reset_renderer() {
    RENDERER_RECORDER.with(|recorder| *recorder.borrow_mut() = RendererRecorder::default());
}

unsafe fn bytes(span: NativeUtf8) -> Vec<u8> {
    if span.len == 0 {
        return Vec::new();
    }
    // SAFETY: Test callbacks only receive spans valid for the duration of the call.
    unsafe { std::slice::from_raw_parts(span.data, span.len) }.to_vec()
}

unsafe extern "C" fn get_api(version: u32) -> *const NativeRendererApiV1 {
    if version == NATIVE_RENDERER_ABI_VERSION {
        ptr::addr_of!(API)
    } else {
        ptr::null()
    }
}

unsafe extern "C" fn acquire(
    host: NativeHostHandle,
    callbacks: *const NativeRendererCallbacksV1,
    renderer: *mut NativeRendererHandle,
) -> NativeStatus {
    if host != HOST || callbacks.is_null() || renderer.is_null() {
        return NATIVE_STATUS_INVALID_ARGUMENT;
    }
    // SAFETY: The pointers were validated above.
    let callbacks = unsafe { *callbacks };
    RENDERER_RECORDER.with(|recorder| recorder.borrow_mut().callbacks = Some(callbacks));
    // SAFETY: The output pointer was validated above.
    unsafe { *renderer = RENDERER };
    NATIVE_STATUS_OK
}

unsafe extern "C" fn release(renderer: NativeRendererHandle) -> NativeStatus {
    if renderer != RENDERER {
        return NATIVE_STATUS_INVALID_SESSION;
    }
    RENDERER_RECORDER.with(|recorder| {
        let mut recorder = recorder.borrow_mut();
        recorder.releases += 1;
        if recorder.failure == Some("release") {
            NATIVE_STATUS_HOST_ERROR
        } else {
            NATIVE_STATUS_OK
        }
    })
}

unsafe extern "C" fn get_root(
    renderer: NativeRendererHandle,
    root: *mut NativeNodeHandle,
) -> NativeStatus {
    if renderer != RENDERER || root.is_null() {
        return NATIVE_STATUS_INVALID_ARGUMENT;
    }
    // SAFETY: The output pointer was validated above.
    unsafe { *root = ROOT };
    NATIVE_STATUS_OK
}

unsafe extern "C" fn create_node(
    _: NativeRendererHandle,
    _: NativeUtf8,
    node: *mut NativeNodeHandle,
) -> NativeStatus {
    if !node.is_null() {
        // SAFETY: NativeHost provides writable output storage.
        unsafe { *node = 200 };
    }
    NATIVE_STATUS_OK
}

unsafe extern "C" fn set_raw_text(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    _: NativeUtf8,
) -> NativeStatus {
    NATIVE_STATUS_OK
}

unsafe extern "C" fn set_attribute(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    _: NativeUtf8,
    _: NativeUtf8,
) -> NativeStatus {
    NATIVE_STATUS_OK
}

unsafe extern "C" fn insert_before(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    _: NativeNodeHandle,
    _: NativeNodeHandle,
) -> NativeStatus {
    NATIVE_STATUS_OK
}

unsafe extern "C" fn node_operation(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    _: NativeNodeHandle,
) -> NativeStatus {
    NATIVE_STATUS_OK
}

unsafe extern "C" fn destroy_node(_: NativeRendererHandle, _: NativeNodeHandle) -> NativeStatus {
    NATIVE_STATUS_OK
}

unsafe extern "C" fn add_listener(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    listener: NativeListenerHandle,
    callback: NativeCallbackHandle,
    name: NativeUtf8,
) -> NativeStatus {
    // SAFETY: NativeHost provides a readable name span.
    let name = unsafe { bytes(name) };
    RENDERER_RECORDER.with(|recorder| {
        recorder.borrow_mut().listener = Some((listener, callback, name));
    });
    NATIVE_STATUS_OK
}

unsafe extern "C" fn remove_listener(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    listener: NativeListenerHandle,
    callback: NativeCallbackHandle,
    _: NativeUtf8,
) -> NativeStatus {
    RENDERER_RECORDER.with(|recorder| {
        let mut recorder = recorder.borrow_mut();
        match recorder.listener.take() {
            Some((registered_listener, registered_callback, _))
                if registered_listener == listener && registered_callback == callback =>
            {
                NATIVE_STATUS_OK
            }
            registration => {
                recorder.listener = registration;
                NATIVE_STATUS_INVALID_LISTENER
            }
        }
    })
}

unsafe extern "C" fn flush(_: NativeRendererHandle) -> NativeStatus {
    let reentry = RENDERER_RECORDER.with(|recorder| {
        let mut recorder = recorder.borrow_mut();
        recorder.flushes += 1;
        if recorder.failure == Some("flush") {
            return Err(NATIVE_STATUS_HOST_ERROR);
        }
        if recorder.reenter_on_flush {
            recorder.reenter_on_flush = false;
            Ok(recorder.callbacks.zip(recorder.listener.clone()))
        } else {
            Ok(None)
        }
    });
    let Some((callbacks, (listener, callback, name))) = (match reentry {
        Ok(reentry) => reentry,
        Err(status) => return status,
    }) else {
        return NATIVE_STATUS_OK;
    };
    let event = callbacks.on_event.expect("event callback");
    // SAFETY: All copied callback data and local spans remain valid synchronously.
    let status = unsafe {
        event(
            callbacks.context,
            RENDERER,
            listener,
            callback,
            utf8(&name),
            utf8(b"application/vnd.lynx.tap"),
            native_bytes(&[]),
        )
    };
    RENDERER_RECORDER.with(|recorder| recorder.borrow_mut().nested_status = Some(status));
    NATIVE_STATUS_OK
}

unsafe extern "C" fn create_timer(
    _: NativeRendererHandle,
    _: u64,
    _: u32,
    _: NativeCallbackHandle,
    _: *mut NativeTimerHandle,
) -> NativeStatus {
    NATIVE_STATUS_UNSUPPORTED
}

unsafe extern "C" fn cancel_timer(_: NativeRendererHandle, _: NativeTimerHandle) -> NativeStatus {
    NATIVE_STATUS_UNSUPPORTED
}

unsafe extern "C" fn import_style_sheet(
    _: NativeRendererHandle,
    fragment: NativeBytes,
) -> NativeStatus {
    let bytes = if fragment.len == 0 {
        Vec::new()
    } else {
        // SAFETY: NativeHost provides a valid borrowed span for this call.
        unsafe { std::slice::from_raw_parts(fragment.data, fragment.len) }.to_vec()
    };
    RENDERER_RECORDER.with(|recorder| recorder.borrow_mut().style_sheet = bytes);
    NATIVE_STATUS_OK
}

unsafe extern "C" fn clear_style_sheets(_: NativeRendererHandle) -> NativeStatus {
    RENDERER_RECORDER.with(|recorder| {
        let mut recorder = recorder.borrow_mut();
        recorder.style_sheet.clear();
        recorder.style_sheet_clears += 1;
        if recorder.failure == Some("clear_style_sheets") {
            NATIVE_STATUS_HOST_ERROR
        } else {
            NATIVE_STATUS_OK
        }
    })
}

static API: NativeRendererApiV1 = NativeRendererApiV1 {
    abi_version: NATIVE_RENDERER_ABI_VERSION,
    struct_size: mem::size_of::<NativeRendererApiV1>(),
    acquire: Some(acquire),
    release: Some(release),
    get_root: Some(get_root),
    create_element: Some(create_node),
    create_raw_text: Some(create_node),
    set_raw_text: Some(set_raw_text),
    set_attribute: Some(set_attribute),
    insert_before: Some(insert_before),
    remove_child: Some(node_operation),
    destroy_node: Some(destroy_node),
    add_event_listener: Some(add_listener),
    remove_event_listener: Some(remove_listener),
    flush: Some(flush),
    create_timer: Some(create_timer),
    cancel_timer: Some(cancel_timer),
    import_style_sheet: Some(import_style_sheet),
    clear_style_sheets: Some(clear_style_sheets),
};

fn utf8(value: &[u8]) -> NativeUtf8 {
    NativeUtf8 {
        data: value.as_ptr(),
        len: value.len(),
    }
}

fn native_bytes(value: &[u8]) -> NativeBytes {
    NativeBytes {
        data: value.as_ptr(),
        len: value.len(),
    }
}

fn initial_batch() -> CommandBatch {
    CommandBatch {
        sequence: 0,
        commands: vec![Command::AddEventListener {
            node: NodeId::new(1).unwrap(),
            listener: ListenerId::new(1).unwrap(),
            callback: CallbackId::new(1).unwrap(),
            name: "tap".into(),
        }],
        final_commit: true,
    }
}

fn mount() -> (
    LynxElementBridgeNativeMountResult,
    Rc<RefCell<BackendRecorder>>,
) {
    reset_renderer();
    let recorder = Rc::new(RefCell::new(BackendRecorder::default()));
    let backend_recorder = Rc::clone(&recorder);
    // SAFETY: The static renderer API implements the complete native renderer contract.
    let mounted = unsafe {
        native_mount(Some(get_api), HOST, move |_| {
            Ok((
                Box::new(TestBackend::new(backend_recorder)),
                initial_batch(),
            ))
        })
    };
    (mounted, recorder)
}

fn callback() -> (
    NativeRendererCallbacksV1,
    NativeListenerHandle,
    NativeCallbackHandle,
    Vec<u8>,
) {
    RENDERER_RECORDER.with(|recorder| {
        let recorder = recorder.borrow();
        let (listener, callback, name) = recorder.listener.clone().expect("registered listener");
        (
            recorder.callbacks.expect("acquired callbacks"),
            listener,
            callback,
            name,
        )
    })
}

#[allow(clippy::too_many_arguments)]
unsafe fn send_event(
    callbacks: NativeRendererCallbacksV1,
    context: *mut c_void,
    renderer: NativeRendererHandle,
    listener: NativeListenerHandle,
    callback: NativeCallbackHandle,
    name: NativeUtf8,
    content_type: NativeUtf8,
    payload: NativeBytes,
) -> NativeStatus {
    // SAFETY: Callers provide valid spans or intentionally malformed spans rejected before reads.
    unsafe {
        callbacks.on_event.expect("event callback")(
            context,
            renderer,
            listener,
            callback,
            name,
            content_type,
            payload,
        )
    }
}

#[test]
fn mount_event_destroy_and_timer_follow_the_public_native_lifecycle() {
    let (mounted, backend) = mount();
    assert_eq!(mounted.status, NATIVE_STATUS_OK);
    assert_ne!(mounted.session, 0);
    let (callbacks, listener, callback, name) = callback();
    assert_eq!(
        // SAFETY: Callback identity and spans come from the live registration.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[0, 255]),
            )
        },
        NATIVE_STATUS_OK
    );
    assert_eq!(backend.borrow().payload, vec![0, 255]);
    assert_eq!(
        // SAFETY: The timer callback is a required ABI shim with no borrowed spans.
        unsafe {
            callbacks.on_timer.expect("timer callback")(callbacks.context, RENDERER, 9, callback)
        },
        NATIVE_STATUS_UNSUPPORTED
    );

    let destroyed = native_destroy_session(mounted.session);
    assert_eq!(
        (destroyed.status, destroyed.consumed),
        (NATIVE_STATUS_OK, 1)
    );
    assert_eq!(backend.borrow().destroys, 1);
    assert_eq!(
        RENDERER_RECORDER.with(|recorder| recorder.borrow().releases),
        1
    );
    assert_eq!(
        // SAFETY: This deliberately replays a callback for the consumed session.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        },
        NATIVE_STATUS_INVALID_SESSION
    );
}

#[test]
fn event_callback_rejects_invalid_identity_and_spans_before_dispatch() {
    let (mounted, backend) = mount();
    let (callbacks, listener, callback, name) = callback();
    let rejected = [
        // SAFETY: A null context is rejected before session access.
        unsafe {
            send_event(
                callbacks,
                ptr::null_mut(),
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        },
        // SAFETY: Ordinary spans remain valid synchronously.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER + 1,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        },
        // SAFETY: Ordinary spans remain valid synchronously.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener + 1,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        },
        // SAFETY: Ordinary spans remain valid synchronously.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener,
                callback + 1,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        },
        // SAFETY: Ordinary spans remain valid synchronously.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener,
                callback,
                utf8(b"click"),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        },
        // SAFETY: The malformed name span is rejected before reading.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener,
                callback,
                NativeUtf8 {
                    data: ptr::null(),
                    len: 1,
                },
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        },
        // SAFETY: The invalid UTF-8 is readable and rejected during validation.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(&[255]),
                native_bytes(&[]),
            )
        },
        // SAFETY: The malformed payload span is rejected before reading.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                NativeBytes {
                    data: ptr::null(),
                    len: 1,
                },
            )
        },
    ];
    assert_eq!(
        rejected,
        [
            NATIVE_STATUS_INVALID_SESSION,
            NATIVE_STATUS_INVALID_SESSION,
            NATIVE_STATUS_INVALID_LISTENER,
            NATIVE_STATUS_INVALID_LISTENER,
            NATIVE_STATUS_INVALID_LISTENER,
            NATIVE_STATUS_INVALID_ARGUMENT,
            NATIVE_STATUS_INVALID_ARGUMENT,
            NATIVE_STATUS_INVALID_ARGUMENT,
        ]
    );
    assert_eq!(backend.borrow().dispatches, 0);
    assert_eq!(
        native_destroy_session(mounted.session).status,
        NATIVE_STATUS_OK
    );
}

#[test]
fn destroy_and_abandon_preserve_owner_thread_and_consumption() {
    let (mounted, _) = mount();
    let session = mounted.session;
    let (callbacks, listener, callback, name) = callback();
    let event_callback = callbacks.on_event.expect("event callback");
    let context = callbacks.context as usize;
    let wrong_thread_event = std::thread::spawn(move || {
        // SAFETY: The spans are local to this call; owner validation rejects before dispatch.
        unsafe {
            event_callback(
                context as *mut c_void,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        }
    })
    .join()
    .unwrap();
    assert_eq!(wrong_thread_event, NATIVE_STATUS_WRONG_THREAD);
    let wrong_thread = std::thread::spawn(move || {
        let destroyed = native_destroy_session(session);
        (destroyed.status, destroyed.consumed)
    })
    .join()
    .unwrap();
    assert_eq!(wrong_thread, (NATIVE_STATUS_WRONG_THREAD, 0));
    let destroyed = native_destroy_session(session);
    assert_eq!(
        (destroyed.status, destroyed.consumed),
        (NATIVE_STATUS_OK, 1)
    );
    let stale = native_destroy_session(session);
    assert_eq!(
        (stale.status, stale.consumed),
        (NATIVE_STATUS_INVALID_SESSION, 0)
    );

    let (mounted, backend) = mount();
    let session = mounted.session;
    let wrong_thread = std::thread::spawn(move || {
        let abandoned = native_abandon_session(session);
        (abandoned.status, abandoned.consumed)
    })
    .join()
    .unwrap();
    assert_eq!(wrong_thread, (NATIVE_STATUS_WRONG_THREAD, 0));
    let abandoned = native_abandon_session(session);
    assert_eq!(
        (abandoned.status, abandoned.consumed),
        (NATIVE_STATUS_OK, 1)
    );
    assert_eq!(backend.borrow().abandons, 1);
    assert_eq!(backend.borrow().destroys, 0);
}

#[test]
fn poisoned_session_abandons_without_teardown_and_releases_once() {
    let (mounted, backend) = mount();
    let (callbacks, listener, callback, name) = callback();
    RENDERER_RECORDER.with(|recorder| recorder.borrow_mut().failure = Some("flush"));
    assert_eq!(
        // SAFETY: Callback identity and spans come from the live registration.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        },
        NATIVE_STATUS_HOST_ERROR
    );
    RENDERER_RECORDER.with(|recorder| recorder.borrow_mut().failure = None);

    let abandoned = native_abandon_session(mounted.session);
    assert_eq!(
        (abandoned.status, abandoned.consumed),
        (NATIVE_STATUS_OK, 1)
    );
    let backend = backend.borrow();
    assert_eq!(backend.dispatches, 1);
    assert_eq!(backend.discards, 2);
    assert_eq!(backend.abandons, 1);
    assert_eq!(backend.destroys, 0);
    drop(backend);
    assert_eq!(
        RENDERER_RECORDER.with(|recorder| recorder.borrow().releases),
        1
    );
    assert_eq!(
        native_destroy_session(mounted.session).status,
        NATIVE_STATUS_INVALID_SESSION
    );
}

#[test]
fn poisoned_session_rejects_events_and_destroy_still_consumes_and_releases() {
    let (mounted, backend) = mount();
    let (callbacks, listener, callback, name) = callback();
    RENDERER_RECORDER.with(|recorder| recorder.borrow_mut().failure = Some("flush"));
    assert_eq!(
        // SAFETY: Callback identity and spans come from the live registration.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        },
        NATIVE_STATUS_HOST_ERROR
    );
    RENDERER_RECORDER.with(|recorder| recorder.borrow_mut().failure = None);
    assert_eq!(
        // SAFETY: Callback identity and spans remain valid, but the session is poisoned.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        },
        NATIVE_STATUS_HOST_ERROR
    );
    assert_eq!(backend.borrow().dispatches, 1);

    let destroyed = native_destroy_session(mounted.session);
    assert_eq!(
        (destroyed.status, destroyed.consumed),
        (NATIVE_STATUS_HOST_ERROR, 1)
    );
    assert_eq!(backend.borrow().destroys, 1);
    assert!(backend.borrow().destroyed_poisoned);
    assert_eq!(
        RENDERER_RECORDER.with(|recorder| recorder.borrow().releases),
        1
    );
}

#[test]
fn abandon_consumes_session_even_when_release_fails() {
    let (mounted, _) = mount();
    RENDERER_RECORDER.with(|recorder| recorder.borrow_mut().failure = Some("release"));
    let abandoned = native_abandon_session(mounted.session);
    assert_eq!(
        (abandoned.status, abandoned.consumed),
        (NATIVE_STATUS_HOST_ERROR, 1)
    );
    RENDERER_RECORDER.with(|recorder| recorder.borrow_mut().failure = None);
    let stale = native_abandon_session(mounted.session);
    assert_eq!(
        (stale.status, stale.consumed),
        (NATIVE_STATUS_INVALID_SESSION, 0)
    );
    assert_eq!(
        RENDERER_RECORDER.with(|recorder| recorder.borrow().releases),
        1
    );
}

#[test]
fn reentrant_event_is_rejected_while_outer_event_completes() {
    let (mounted, backend) = mount();
    let (callbacks, listener, callback, name) = callback();
    RENDERER_RECORDER.with(|recorder| recorder.borrow_mut().reenter_on_flush = true);
    assert_eq!(
        // SAFETY: Callback identity and spans come from the live registration.
        unsafe {
            send_event(
                callbacks,
                callbacks.context,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                native_bytes(&[]),
            )
        },
        NATIVE_STATUS_OK
    );
    assert_eq!(
        RENDERER_RECORDER.with(|recorder| recorder.borrow().nested_status),
        Some(NATIVE_STATUS_HOST_ERROR)
    );
    assert_eq!(backend.borrow().dispatches, 1);
    assert_eq!(
        native_destroy_session(mounted.session).status,
        NATIVE_STATUS_OK
    );
}

#[test]
fn mount_rejects_invalid_inputs_and_rolls_back_failed_initial_flush() {
    reset_renderer();
    // SAFETY: Missing resolver and zero host are rejected before dereferencing native inputs.
    let missing_api =
        unsafe { native_mount(None, HOST, |_| unreachable!("backend must not be created")) };
    assert_eq!(
        (missing_api.status, missing_api.session),
        (NATIVE_STATUS_INVALID_ARGUMENT, 0)
    );
    // SAFETY: Missing resolver and zero host are rejected before dereferencing native inputs.
    let missing_host = unsafe {
        native_mount(Some(get_api), 0, |_| {
            unreachable!("backend must not be created")
        })
    };
    assert_eq!(
        (missing_host.status, missing_host.session),
        (NATIVE_STATUS_INVALID_ARGUMENT, 0)
    );

    let backend = Rc::new(RefCell::new(BackendRecorder::default()));
    let backend_recorder = Rc::clone(&backend);
    RENDERER_RECORDER.with(|recorder| recorder.borrow_mut().failure = Some("flush"));
    // SAFETY: The static renderer API implements the complete native renderer contract.
    let failed = unsafe {
        native_mount(Some(get_api), HOST, move |_| {
            Ok((
                Box::new(TestBackend::new(backend_recorder)),
                initial_batch(),
            ))
        })
    };
    assert_eq!(
        (failed.status, failed.session),
        (NATIVE_STATUS_HOST_ERROR, 0)
    );
    assert_eq!(backend.borrow().abandons, 1);
    assert_eq!(
        RENDERER_RECORDER.with(|recorder| recorder.borrow().releases),
        1
    );
}
