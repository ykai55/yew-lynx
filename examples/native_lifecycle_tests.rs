use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, c_void};
use std::mem;
use std::ptr;

use lynx_element_bridge_core::{CallbackId, CommandBatch, EventMessage, SessionId, Status};
use lynx_element_bridge_ffi::native_host::{
    NATIVE_RENDERER_ABI_VERSION, NATIVE_STATUS_HOST_ERROR, NATIVE_STATUS_INVALID_ARGUMENT,
    NATIVE_STATUS_INVALID_LISTENER, NATIVE_STATUS_INVALID_OWNERSHIP, NATIVE_STATUS_INVALID_SESSION,
    NATIVE_STATUS_OK, NATIVE_STATUS_PANIC, NATIVE_STATUS_WRONG_THREAD, NativeBytes,
    NativeCallbackHandle, NativeHostHandle, NativeListenerHandle, NativeNodeHandle,
    NativeRendererApiV1, NativeRendererCallbacksV1, NativeRendererHandle, NativeStatus,
    NativeTimerHandle, NativeUtf8,
};
use lynx_element_bridge_ffi::{BackendError, BridgeBackend, NativeTimerRequest};

use super::{
    lynx_element_bridge_backend, lynx_element_bridge_native_abandon_session,
    lynx_element_bridge_native_destroy_session, lynx_element_bridge_native_mount,
};

const HOST: NativeHostHandle = 41;
const RENDERER: NativeRendererHandle = 73;
const ROOT: NativeNodeHandle = 101;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Call {
    Release,
    Text(Vec<u8>),
    Mutation,
    AddListener {
        listener: NativeListenerHandle,
        callback: NativeCallbackHandle,
        name: Vec<u8>,
    },
    RemoveListener,
    Flush,
    CreateTimer {
        delay_millis: u64,
        repeating: bool,
        callback: NativeCallbackHandle,
        timer: NativeTimerHandle,
    },
    CancelTimer(NativeTimerHandle),
}

struct Recorder {
    callbacks: Option<NativeRendererCallbacksV1>,
    registrations: HashMap<NativeListenerHandle, (NativeCallbackHandle, Vec<u8>)>,
    timers: HashMap<NativeTimerHandle, (NativeCallbackHandle, bool)>,
    calls: Vec<Call>,
    next_node: NativeNodeHandle,
    next_timer: NativeTimerHandle,
    failure: Option<&'static str>,
    reenter_on_flush: bool,
    reenter_timer_on_flush: bool,
    nested_status: Option<NativeStatus>,
}

impl Default for Recorder {
    fn default() -> Self {
        Self {
            callbacks: None,
            registrations: HashMap::new(),
            timers: HashMap::new(),
            calls: Vec::new(),
            next_node: 200,
            next_timer: 300,
            failure: None,
            reenter_on_flush: false,
            reenter_timer_on_flush: false,
            nested_status: None,
        }
    }
}

thread_local! {
    static RECORDER: RefCell<Recorder> = RefCell::new(Recorder::default());
    static API: NativeRendererApiV1 = recording_api();
}

fn recording_api() -> NativeRendererApiV1 {
    NativeRendererApiV1 {
        abi_version: NATIVE_RENDERER_ABI_VERSION,
        struct_size: mem::size_of::<NativeRendererApiV1>(),
        acquire: Some(record_acquire),
        release: Some(record_release),
        get_root: Some(record_get_root),
        create_element: Some(record_create_element),
        create_raw_text: Some(record_create_raw_text),
        set_raw_text: Some(record_set_raw_text),
        set_attribute: Some(record_set_attribute),
        insert_before: Some(record_insert_before),
        remove_child: Some(record_remove_child),
        destroy_node: Some(record_destroy_node),
        add_event_listener: Some(record_add_listener),
        remove_event_listener: Some(record_remove_listener),
        flush: Some(record_flush),
        create_timer: Some(record_create_timer),
        cancel_timer: Some(record_cancel_timer),
    }
}

fn reset() {
    RECORDER.with(|recorder| *recorder.borrow_mut() = Recorder::default());
}

fn clear_calls() {
    RECORDER.with(|recorder| recorder.borrow_mut().calls.clear());
}

fn calls() -> Vec<Call> {
    RECORDER.with(|recorder| recorder.borrow().calls.clone())
}

fn status(operation: &'static str) -> NativeStatus {
    RECORDER.with(|recorder| {
        recorder
            .borrow()
            .failure
            .filter(|candidate| *candidate == operation)
            .map_or(NATIVE_STATUS_OK, |_| NATIVE_STATUS_HOST_ERROR)
    })
}

fn record(operation: &'static str, call: Call) -> NativeStatus {
    RECORDER.with(|recorder| recorder.borrow_mut().calls.push(call));
    status(operation)
}

unsafe fn span_bytes(data: *const u8, len: usize) -> Vec<u8> {
    if len == 0 || data.is_null() {
        return Vec::new();
    }
    // SAFETY: Recording callbacks borrow spans only for the duration of the ABI call.
    unsafe { std::slice::from_raw_parts(data, len) }.to_vec()
}

unsafe extern "C" fn get_api(version: u32) -> *const NativeRendererApiV1 {
    if version != NATIVE_RENDERER_ABI_VERSION {
        return ptr::null();
    }
    API.with(ptr::from_ref)
}

unsafe extern "C" fn record_acquire(
    host: NativeHostHandle,
    callbacks: *const NativeRendererCallbacksV1,
    renderer: *mut NativeRendererHandle,
) -> NativeStatus {
    if host != HOST || callbacks.is_null() || renderer.is_null() {
        return NATIVE_STATUS_INVALID_ARGUMENT;
    }
    // SAFETY: The lifecycle passes readable callbacks and writable renderer storage.
    let callbacks = unsafe { *callbacks };
    RECORDER.with(|recorder| recorder.borrow_mut().callbacks = Some(callbacks));
    // SAFETY: The null output case was rejected above.
    unsafe { *renderer = RENDERER };
    NATIVE_STATUS_OK
}

unsafe extern "C" fn record_release(renderer: NativeRendererHandle) -> NativeStatus {
    if renderer != RENDERER {
        return NATIVE_STATUS_INVALID_SESSION;
    }
    RECORDER.with(|recorder| recorder.borrow_mut().timers.clear());
    record("release", Call::Release)
}

unsafe extern "C" fn record_get_root(
    renderer: NativeRendererHandle,
    root: *mut NativeNodeHandle,
) -> NativeStatus {
    if renderer != RENDERER || root.is_null() {
        return NATIVE_STATUS_INVALID_ARGUMENT;
    }
    // SAFETY: The null output case was rejected above.
    unsafe { *root = ROOT };
    NATIVE_STATUS_OK
}

unsafe extern "C" fn record_create_element(
    _: NativeRendererHandle,
    _: NativeUtf8,
    node: *mut NativeNodeHandle,
) -> NativeStatus {
    let output = RECORDER.with(|recorder| {
        let mut recorder = recorder.borrow_mut();
        let output = recorder.next_node;
        recorder.next_node += 1;
        recorder.calls.push(Call::Mutation);
        output
    });
    if !node.is_null() {
        // SAFETY: NativeHost provides writable output storage.
        unsafe { *node = output };
    }
    status("create_element")
}

unsafe extern "C" fn record_create_raw_text(
    _: NativeRendererHandle,
    text: NativeUtf8,
    node: *mut NativeNodeHandle,
) -> NativeStatus {
    // SAFETY: The renderer contract provides a borrowed span for this call.
    let text = unsafe { span_bytes(text.data, text.len) };
    let output = RECORDER.with(|recorder| {
        let mut recorder = recorder.borrow_mut();
        let output = recorder.next_node;
        recorder.next_node += 1;
        recorder.calls.push(Call::Text(text));
        output
    });
    if !node.is_null() {
        // SAFETY: NativeHost provides writable output storage.
        unsafe { *node = output };
    }
    status("create_raw_text")
}

unsafe extern "C" fn record_set_raw_text(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    text: NativeUtf8,
) -> NativeStatus {
    // SAFETY: The renderer contract provides a borrowed span for this call.
    record(
        "set_raw_text",
        Call::Text(unsafe { span_bytes(text.data, text.len) }),
    )
}

unsafe extern "C" fn record_set_attribute(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    _: NativeUtf8,
    _: NativeUtf8,
) -> NativeStatus {
    record("set_attribute", Call::Mutation)
}

unsafe extern "C" fn record_insert_before(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    _: NativeNodeHandle,
    _: NativeNodeHandle,
) -> NativeStatus {
    record("insert_before", Call::Mutation)
}

unsafe extern "C" fn record_remove_child(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    _: NativeNodeHandle,
) -> NativeStatus {
    record("remove_child", Call::Mutation)
}

unsafe extern "C" fn record_destroy_node(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
) -> NativeStatus {
    record("destroy_node", Call::Mutation)
}

unsafe extern "C" fn record_add_listener(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    listener: NativeListenerHandle,
    callback: NativeCallbackHandle,
    name: NativeUtf8,
) -> NativeStatus {
    // SAFETY: The renderer contract provides a borrowed span for this call.
    let name = unsafe { span_bytes(name.data, name.len) };
    RECORDER.with(|recorder| {
        recorder
            .borrow_mut()
            .registrations
            .insert(listener, (callback, name.clone()));
    });
    record(
        "add_event_listener",
        Call::AddListener {
            listener,
            callback,
            name,
        },
    )
}

unsafe extern "C" fn record_remove_listener(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    listener: NativeListenerHandle,
    _: NativeCallbackHandle,
    _: NativeUtf8,
) -> NativeStatus {
    RECORDER.with(|recorder| {
        recorder.borrow_mut().registrations.remove(&listener);
    });
    record("remove_event_listener", Call::RemoveListener)
}

unsafe extern "C" fn record_flush(renderer: NativeRendererHandle) -> NativeStatus {
    if renderer != RENDERER {
        return NATIVE_STATUS_INVALID_SESSION;
    }
    let reentry = RECORDER.with(|recorder| {
        let mut recorder = recorder.borrow_mut();
        recorder.calls.push(Call::Flush);
        if recorder.reenter_on_flush {
            recorder.reenter_on_flush = false;
            recorder.callbacks.map(|callbacks| (callbacks, None))
        } else if recorder.reenter_timer_on_flush {
            recorder.reenter_timer_on_flush = false;
            let timer = recorder
                .timers
                .iter()
                .next()
                .map(|(timer, (callback, _))| (*timer, *callback));
            recorder.callbacks.map(|callbacks| (callbacks, timer))
        } else {
            None
        }
    });
    if let Some((callback_table, timer)) = reentry {
        let nested_status = if let Some((timer, callback)) = timer {
            // SAFETY: The copied callback table remains valid synchronously.
            unsafe {
                callback_table.on_timer.expect("timer callback")(
                    callback_table.context,
                    RENDERER,
                    timer,
                    callback,
                )
            }
        } else {
            let (listener, callback, name) = registration();
            // SAFETY: The copied callback table and borrowed spans remain valid synchronously.
            unsafe {
                callback_table.on_event.expect("event callback")(
                    callback_table.context,
                    RENDERER,
                    listener,
                    callback,
                    utf8(&name),
                    utf8(b"application/vnd.lynx.tap"),
                    bytes(&[]),
                )
            }
        };
        RECORDER.with(|recorder| recorder.borrow_mut().nested_status = Some(nested_status));
    }
    status("flush")
}

unsafe extern "C" fn record_create_timer(
    renderer: NativeRendererHandle,
    delay_millis: u64,
    repeating: u32,
    callback: NativeCallbackHandle,
    timer: *mut NativeTimerHandle,
) -> NativeStatus {
    if renderer != RENDERER || timer.is_null() || callback == 0 || repeating > 1 {
        return NATIVE_STATUS_INVALID_ARGUMENT;
    }
    let timer_handle = RECORDER.with(|recorder| {
        let mut recorder = recorder.borrow_mut();
        let timer = recorder.next_timer;
        recorder.next_timer += 1;
        recorder.timers.insert(timer, (callback, repeating != 0));
        recorder.calls.push(Call::CreateTimer {
            delay_millis,
            repeating: repeating != 0,
            callback,
            timer,
        });
        timer
    });
    let timer_status = status("create_timer");
    if timer_status == NATIVE_STATUS_OK {
        // SAFETY: The null output case was rejected above.
        unsafe { *timer = timer_handle };
    }
    timer_status
}

unsafe extern "C" fn record_cancel_timer(
    renderer: NativeRendererHandle,
    timer: NativeTimerHandle,
) -> NativeStatus {
    if renderer != RENDERER {
        return NATIVE_STATUS_INVALID_SESSION;
    }
    let owned = RECORDER.with(|recorder| recorder.borrow_mut().timers.remove(&timer).is_some());
    let status = record("cancel_timer", Call::CancelTimer(timer));
    if owned {
        status
    } else {
        NATIVE_STATUS_INVALID_OWNERSHIP
    }
}

fn utf8(value: &[u8]) -> NativeUtf8 {
    NativeUtf8 {
        data: value.as_ptr(),
        len: value.len(),
    }
}

fn bytes(value: &[u8]) -> NativeBytes {
    NativeBytes {
        data: value.as_ptr(),
        len: value.len(),
    }
}

fn callbacks() -> NativeRendererCallbacksV1 {
    RECORDER.with(|recorder| {
        recorder
            .borrow()
            .callbacks
            .expect("callbacks were not acquired")
    })
}

fn registration() -> (NativeListenerHandle, NativeCallbackHandle, Vec<u8>) {
    RECORDER.with(|recorder| {
        let recorder = recorder.borrow();
        let (listener, (callback, name)) = recorder
            .registrations
            .iter()
            .next()
            .expect("listener was not registered");
        (*listener, *callback, name.clone())
    })
}

fn timer_registration() -> (NativeTimerHandle, NativeCallbackHandle, bool) {
    RECORDER.with(|recorder| {
        let recorder = recorder.borrow();
        let (timer, (callback, repeating)) = recorder
            .timers
            .iter()
            .next()
            .expect("timer was not registered");
        (*timer, *callback, *repeating)
    })
}

fn mount() -> lynx_element_bridge_ffi::LynxElementBridgeNativeMountResult {
    reset();
    // SAFETY: The recording resolver and table implement the production ABI for this test.
    unsafe { lynx_element_bridge_native_mount(Some(get_api), HOST) }
}

#[allow(clippy::too_many_arguments)]
unsafe fn send_event(
    callback_table: NativeRendererCallbacksV1,
    context: *mut c_void,
    renderer: NativeRendererHandle,
    listener: NativeListenerHandle,
    callback: NativeCallbackHandle,
    name: NativeUtf8,
    content_type: NativeUtf8,
    payload: NativeBytes,
) -> NativeStatus {
    // SAFETY: The caller supplies spans satisfying the callback contract or deliberate malformed
    // spans which the callback must reject before reading.
    unsafe {
        callback_table.on_event.expect("event callback")(
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

unsafe fn send_timer(
    callback_table: NativeRendererCallbacksV1,
    context: *mut c_void,
    renderer: NativeRendererHandle,
    timer: NativeTimerHandle,
    callback: NativeCallbackHandle,
) -> NativeStatus {
    // SAFETY: The caller keeps the captured function table and session context live.
    unsafe { callback_table.on_timer.expect("timer callback")(context, renderer, timer, callback) }
}

unsafe fn fire_timer(
    callback_table: NativeRendererCallbacksV1,
    timer: NativeTimerHandle,
    callback: NativeCallbackHandle,
) -> NativeStatus {
    let repeating = RECORDER.with(|recorder| {
        recorder
            .borrow()
            .timers
            .get(&timer)
            .map(|(_, repeating)| *repeating)
            .expect("timer was not registered")
    });
    // SAFETY: The captured callback table, context, and timer identity remain live synchronously.
    let status = unsafe {
        send_timer(
            callback_table,
            callback_table.context,
            RENDERER,
            timer,
            callback,
        )
    };
    if !repeating || status != NATIVE_STATUS_OK {
        RECORDER.with(|recorder| recorder.borrow_mut().timers.remove(&timer));
    }
    status
}

#[test]
fn native_mount_timer_event_and_destroy_apply_batches_through_the_function_table() {
    // SAFETY: The backend identity points to static NUL-terminated storage.
    let backend = unsafe { CStr::from_ptr(lynx_element_bridge_backend()) }
        .to_str()
        .unwrap();
    assert!(matches!(backend, "yew" | "dioxus"));
    assert_eq!(
        mem::size_of::<lynx_element_bridge_ffi::LynxElementBridgeNativeMountResult>(),
        8
    );

    let mounted = mount();
    assert_eq!(mounted.status, NATIVE_STATUS_OK);
    assert_ne!(mounted.session, 0);
    assert!(calls().contains(&Call::Text(b"Count: 0".to_vec())));
    assert!(calls().contains(&Call::Text(b"Timer: pending".to_vec())));
    assert!(calls().contains(&Call::Mutation));
    let mounted_calls = calls();
    let flush = mounted_calls
        .iter()
        .position(|call| *call == Call::Flush)
        .expect("initial render was not flushed");
    let create_timer = mounted_calls
        .iter()
        .position(|call| matches!(call, Call::CreateTimer { .. }))
        .expect("initial timer was not registered");
    assert!(flush < create_timer);
    let callback_table = callbacks();
    let (timer, timer_callback, repeating) = timer_registration();
    assert!(!repeating);

    clear_calls();
    // SAFETY: The captured timer identity and callback context remain live synchronously.
    let timer_status = unsafe {
        send_timer(
            callback_table,
            callback_table.context,
            RENDERER,
            timer,
            timer_callback,
        )
    };
    assert_eq!(timer_status, NATIVE_STATUS_OK);
    assert!(calls().contains(&Call::Text(b"Timer: fired".to_vec())));
    assert!(!calls().contains(&Call::Text(b"Count: 1".to_vec())));
    assert_eq!(
        calls().iter().filter(|call| **call == Call::Flush).count(),
        1
    );
    assert_eq!(calls().last(), Some(&Call::Flush));

    clear_calls();
    // SAFETY: The callback data is readable, but the one-shot timer is now stale.
    let stale_timer = unsafe {
        send_timer(
            callback_table,
            callback_table.context,
            RENDERER,
            timer,
            timer_callback,
        )
    };
    assert_eq!(stale_timer, NATIVE_STATUS_INVALID_ARGUMENT);
    assert!(calls().is_empty());

    let (listener, callback, name) = registration();
    clear_calls();
    // SAFETY: All spans remain readable for the synchronous callback.
    let event_status = unsafe {
        send_event(
            callback_table,
            callback_table.context,
            RENDERER,
            listener,
            callback,
            utf8(&name),
            utf8(b"application/vnd.lynx.tap"),
            bytes(&[0, 255]),
        )
    };
    assert_eq!(event_status, NATIVE_STATUS_OK);
    assert!(calls().contains(&Call::Text(b"Count: 1".to_vec())));
    assert_eq!(calls().last(), Some(&Call::Flush));

    clear_calls();
    let destroyed = lynx_element_bridge_native_destroy_session(mounted.session);
    assert_eq!(destroyed.status, NATIVE_STATUS_OK);
    assert_eq!(destroyed.consumed, 1);
    assert!(calls().contains(&Call::RemoveListener));
    assert!(calls().contains(&Call::Mutation));
    assert!(calls().contains(&Call::Flush));
    assert!(
        !calls()
            .iter()
            .any(|call| matches!(call, Call::CancelTimer(_)))
    );
    assert_eq!(calls().last(), Some(&Call::Release));

    // SAFETY: The callback data remains readable, but the consumed context is now stale.
    let stale_callback = unsafe {
        send_event(
            callback_table,
            callback_table.context,
            RENDERER,
            listener,
            callback,
            utf8(&name),
            utf8(b"application/vnd.lynx.tap"),
            bytes(&[]),
        )
    };
    assert_eq!(stale_callback, NATIVE_STATUS_INVALID_SESSION);
}

#[test]
fn native_timer_rejects_foreign_or_mismatched_identities_without_framework_mutation() {
    let mounted = mount();
    let callback_table = callbacks();
    let (timer, callback, _) = timer_registration();
    clear_calls();

    // SAFETY: Each call uses ordinary callback identity data and a live function table.
    let rejected = unsafe {
        [
            send_timer(callback_table, ptr::null_mut(), RENDERER, timer, callback),
            send_timer(
                callback_table,
                callback_table.context,
                RENDERER + 1,
                timer,
                callback,
            ),
            send_timer(
                callback_table,
                callback_table.context,
                RENDERER,
                timer + 1,
                callback,
            ),
            send_timer(
                callback_table,
                callback_table.context,
                RENDERER,
                timer,
                callback + 1,
            ),
        ]
    };
    assert_eq!(rejected[0], NATIVE_STATUS_INVALID_SESSION);
    assert_eq!(rejected[1], NATIVE_STATUS_INVALID_SESSION);
    assert_eq!(rejected[2], NATIVE_STATUS_INVALID_ARGUMENT);
    assert_eq!(rejected[3], NATIVE_STATUS_INVALID_ARGUMENT);
    assert!(calls().is_empty());

    // A callback mismatch must not consume the valid one-shot registration.
    // SAFETY: The exact captured timer identity remains live.
    assert_eq!(
        unsafe {
            send_timer(
                callback_table,
                callback_table.context,
                RENDERER,
                timer,
                callback,
            )
        },
        NATIVE_STATUS_OK
    );
    assert!(calls().contains(&Call::Text(b"Timer: fired".to_vec())));
    assert_eq!(
        lynx_element_bridge_native_destroy_session(mounted.session).status,
        NATIVE_STATUS_OK
    );
}

#[test]
fn native_event_rejects_identity_and_span_errors_without_framework_mutation() {
    let mounted = mount();
    let callback_table = callbacks();
    let (listener, callback, name) = registration();
    clear_calls();

    let rejected = [
        // SAFETY: All ordinary spans remain readable for each synchronous callback.
        unsafe {
            send_event(
                callback_table,
                ptr::null_mut(),
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                bytes(&[]),
            )
        },
        // SAFETY: All spans remain readable for the synchronous callback.
        unsafe {
            send_event(
                callback_table,
                callback_table.context,
                RENDERER + 1,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                bytes(&[]),
            )
        },
        // SAFETY: All spans remain readable for the synchronous callback.
        unsafe {
            send_event(
                callback_table,
                callback_table.context,
                RENDERER,
                listener + 1,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                bytes(&[]),
            )
        },
        // SAFETY: All spans remain readable for the synchronous callback.
        unsafe {
            send_event(
                callback_table,
                callback_table.context,
                RENDERER,
                listener,
                callback + 1,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                bytes(&[]),
            )
        },
        // SAFETY: All spans remain readable for the synchronous callback.
        unsafe {
            send_event(
                callback_table,
                callback_table.context,
                RENDERER,
                listener,
                callback,
                utf8(b"click"),
                utf8(b"application/vnd.lynx.tap"),
                bytes(&[]),
            )
        },
        // SAFETY: The malformed span is rejected before it can be read.
        unsafe {
            send_event(
                callback_table,
                callback_table.context,
                RENDERER,
                listener,
                callback,
                NativeUtf8 {
                    data: ptr::null(),
                    len: 1,
                },
                utf8(b"application/vnd.lynx.tap"),
                bytes(&[]),
            )
        },
        // SAFETY: The invalid UTF-8 is readable and rejected during validation.
        unsafe {
            send_event(
                callback_table,
                callback_table.context,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(&[255]),
                bytes(&[]),
            )
        },
        // SAFETY: The malformed payload is rejected before it can be read.
        unsafe {
            send_event(
                callback_table,
                callback_table.context,
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
    assert_eq!(rejected[0], NATIVE_STATUS_INVALID_SESSION);
    assert_eq!(rejected[1], NATIVE_STATUS_INVALID_SESSION);
    assert_eq!(rejected[2], NATIVE_STATUS_INVALID_LISTENER);
    assert_eq!(rejected[3], NATIVE_STATUS_INVALID_LISTENER);
    assert_eq!(rejected[4], NATIVE_STATUS_INVALID_LISTENER);
    assert_eq!(rejected[5], NATIVE_STATUS_INVALID_ARGUMENT);
    assert_eq!(rejected[6], NATIVE_STATUS_INVALID_ARGUMENT);
    assert_eq!(rejected[7], NATIVE_STATUS_INVALID_ARGUMENT);
    assert!(calls().is_empty());

    // SAFETY: All spans remain readable for the synchronous callback.
    assert_eq!(
        unsafe {
            send_event(
                callback_table,
                callback_table.context,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                bytes(&[]),
            )
        },
        NATIVE_STATUS_OK
    );
    assert!(calls().contains(&Call::Text(b"Count: 1".to_vec())));
    assert_eq!(
        lynx_element_bridge_native_destroy_session(mounted.session).status,
        NATIVE_STATUS_OK
    );
}

#[test]
fn native_destroy_preserves_owner_thread_and_consumption_semantics() {
    let mounted = mount();
    let session = mounted.session;
    let callback_table = callbacks();
    let event_callback = callback_table.on_event.expect("event callback");
    let timer_callback = callback_table.on_timer.expect("timer callback");
    let context = callback_table.context as usize;
    let (listener, callback, name) = registration();
    let (timer, timer_callback_id, _) = timer_registration();
    let wrong_thread_event = std::thread::spawn(move || {
        let content_type = b"application/vnd.lynx.tap";
        // SAFETY: The spans are local to this synchronous call; owner validation rejects it.
        unsafe {
            event_callback(
                context as *mut c_void,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(content_type),
                bytes(&[]),
            )
        }
    })
    .join()
    .unwrap();
    assert_eq!(wrong_thread_event, NATIVE_STATUS_WRONG_THREAD);

    let wrong_thread_timer = std::thread::spawn(move || {
        // SAFETY: Owner validation rejects the live callback identity before dispatch.
        unsafe { timer_callback(context as *mut c_void, RENDERER, timer, timer_callback_id) }
    })
    .join()
    .unwrap();
    assert_eq!(wrong_thread_timer, NATIVE_STATUS_WRONG_THREAD);

    let wrong_thread = std::thread::spawn(move || {
        let result = lynx_element_bridge_native_destroy_session(session);
        (result.status, result.consumed)
    })
    .join()
    .unwrap();
    assert_eq!(wrong_thread, (NATIVE_STATUS_WRONG_THREAD, 0));

    clear_calls();
    let destroyed = lynx_element_bridge_native_destroy_session(session);
    assert_eq!(
        (destroyed.status, destroyed.consumed),
        (NATIVE_STATUS_OK, 1)
    );
    let destroyed_calls = calls();
    assert!(
        !destroyed_calls
            .iter()
            .any(|call| matches!(call, Call::CancelTimer(_)))
    );
    assert_eq!(destroyed_calls.last(), Some(&Call::Release));
    assert_eq!(
        destroyed_calls
            .iter()
            .filter(|call| **call == Call::Release)
            .count(),
        1
    );
    let stale = lynx_element_bridge_native_destroy_session(session);
    assert_eq!(
        (stale.status, stale.consumed),
        (NATIVE_STATUS_INVALID_SESSION, 0)
    );
}

#[test]
fn native_abandon_rejects_the_wrong_thread_without_consuming() {
    let mounted = mount();
    let session = mounted.session;

    let wrong_thread = std::thread::spawn(move || {
        let result = lynx_element_bridge_native_abandon_session(session);
        (result.status, result.consumed)
    })
    .join()
    .unwrap();
    assert_eq!(wrong_thread, (NATIVE_STATUS_WRONG_THREAD, 0));

    clear_calls();
    let abandoned = lynx_element_bridge_native_abandon_session(session);
    assert_eq!(
        (abandoned.status, abandoned.consumed),
        (NATIVE_STATUS_OK, 1)
    );
    assert_eq!(
        calls()
            .iter()
            .filter(|call| **call == Call::Release)
            .count(),
        1
    );
}

#[test]
fn native_abandon_consumes_a_poisoned_session_without_applying_teardown() {
    let mounted = mount();
    let callback_table = callbacks();
    let (listener, callback, name) = registration();
    RECORDER.with(|recorder| recorder.borrow_mut().failure = Some("flush"));

    // SAFETY: All spans remain readable for the synchronous callback.
    assert_eq!(
        unsafe {
            send_event(
                callback_table,
                callback_table.context,
                RENDERER,
                listener,
                callback,
                utf8(&name),
                utf8(b"application/vnd.lynx.tap"),
                bytes(&[]),
            )
        },
        NATIVE_STATUS_HOST_ERROR
    );

    RECORDER.with(|recorder| recorder.borrow_mut().failure = None);
    clear_calls();
    let abandoned = lynx_element_bridge_native_abandon_session(mounted.session);
    assert_eq!(
        (abandoned.status, abandoned.consumed),
        (NATIVE_STATUS_OK, 1)
    );
    assert!(
        !calls()
            .iter()
            .any(|call| matches!(call, Call::Mutation | Call::RemoveListener | Call::Flush))
    );
    assert_eq!(calls().last(), Some(&Call::Release));
    assert_eq!(
        calls()
            .iter()
            .filter(|call| **call == Call::Release)
            .count(),
        1
    );

    let stale_abandon = lynx_element_bridge_native_abandon_session(mounted.session);
    assert_eq!(
        (stale_abandon.status, stale_abandon.consumed),
        (NATIVE_STATUS_INVALID_SESSION, 0)
    );
    let stale_destroy = lynx_element_bridge_native_destroy_session(mounted.session);
    assert_eq!(
        (stale_destroy.status, stale_destroy.consumed),
        (NATIVE_STATUS_INVALID_SESSION, 0)
    );
    assert_eq!(
        calls()
            .iter()
            .filter(|call| **call == Call::Release)
            .count(),
        1
    );
}

#[test]
fn native_abandon_does_not_retry_a_failed_host_release() {
    let mounted = mount();
    clear_calls();
    RECORDER.with(|recorder| recorder.borrow_mut().failure = Some("release"));

    let abandoned = lynx_element_bridge_native_abandon_session(mounted.session);
    assert_eq!(
        (abandoned.status, abandoned.consumed),
        (NATIVE_STATUS_HOST_ERROR, 1)
    );
    assert_eq!(
        calls()
            .iter()
            .filter(|call| **call == Call::Release)
            .count(),
        1
    );

    RECORDER.with(|recorder| recorder.borrow_mut().failure = None);
    let stale = lynx_element_bridge_native_abandon_session(mounted.session);
    assert_eq!(
        (stale.status, stale.consumed),
        (NATIVE_STATUS_INVALID_SESSION, 0)
    );
    assert_eq!(
        calls()
            .iter()
            .filter(|call| **call == Call::Release)
            .count(),
        1
    );
}

#[test]
fn reentrant_event_and_timer_callbacks_fail_as_busy() {
    let mounted = mount();
    let callback_table = callbacks();
    let (listener, callback, name) = registration();
    clear_calls();
    RECORDER.with(|recorder| recorder.borrow_mut().reenter_on_flush = true);

    // SAFETY: All spans remain readable for the synchronous callback.
    let outer = unsafe {
        send_event(
            callback_table,
            callback_table.context,
            RENDERER,
            listener,
            callback,
            utf8(&name),
            utf8(b"application/vnd.lynx.tap"),
            bytes(&[]),
        )
    };
    assert_eq!(outer, NATIVE_STATUS_OK);
    assert_eq!(
        RECORDER.with(|recorder| recorder.borrow().nested_status),
        Some(NATIVE_STATUS_HOST_ERROR)
    );

    let (timer, timer_callback, _) = timer_registration();
    clear_calls();
    RECORDER.with(|recorder| {
        let mut recorder = recorder.borrow_mut();
        recorder.nested_status = None;
        recorder.reenter_timer_on_flush = true;
    });
    // SAFETY: The exact captured timer identity remains live for this synchronous callback.
    let outer_timer = unsafe {
        send_timer(
            callback_table,
            callback_table.context,
            RENDERER,
            timer,
            timer_callback,
        )
    };
    assert_eq!(outer_timer, NATIVE_STATUS_OK);
    assert_eq!(
        RECORDER.with(|recorder| recorder.borrow().nested_status),
        Some(NATIVE_STATUS_HOST_ERROR)
    );
    assert_eq!(
        calls().iter().filter(|call| **call == Call::Flush).count(),
        1
    );
    assert_eq!(
        lynx_element_bridge_native_destroy_session(mounted.session).status,
        NATIVE_STATUS_OK
    );
}

#[test]
fn native_host_failure_poisons_the_session_but_destroy_still_releases_once() {
    let mounted = mount();
    let callback_table = callbacks();
    let (listener, callback, name) = registration();
    clear_calls();
    RECORDER.with(|recorder| recorder.borrow_mut().failure = Some("flush"));

    // SAFETY: All spans remain readable for the synchronous callback.
    let failed = unsafe {
        send_event(
            callback_table,
            callback_table.context,
            RENDERER,
            listener,
            callback,
            utf8(&name),
            utf8(b"application/vnd.lynx.tap"),
            bytes(&[]),
        )
    };
    assert_eq!(failed, NATIVE_STATUS_HOST_ERROR);

    RECORDER.with(|recorder| recorder.borrow_mut().failure = None);
    clear_calls();
    // SAFETY: The callback data is valid but the lifecycle must reject the poisoned session.
    let poisoned = unsafe {
        send_event(
            callback_table,
            callback_table.context,
            RENDERER,
            listener,
            callback,
            utf8(&name),
            utf8(b"application/vnd.lynx.tap"),
            bytes(&[]),
        )
    };
    assert_eq!(poisoned, NATIVE_STATUS_HOST_ERROR);
    assert!(calls().is_empty());

    let destroyed = lynx_element_bridge_native_destroy_session(mounted.session);
    assert_eq!(destroyed.status, NATIVE_STATUS_HOST_ERROR);
    assert_eq!(destroyed.consumed, 1);
    assert_eq!(calls(), vec![Call::Release]);
}

struct FailingRepeatingTimerBackend {
    session: SessionId,
}

impl BridgeBackend for FailingRepeatingTimerBackend {
    fn initial_native_timers(&self) -> Vec<NativeTimerRequest> {
        vec![NativeTimerRequest {
            delay_millis: 1,
            repeating: true,
            callback: CallbackId::new(91).unwrap(),
        }]
    }

    fn dispatch_event(&mut self, _: EventMessage) -> Result<CommandBatch, BackendError> {
        Err(BackendError::recoverable(
            Status::Unsupported,
            "events are not used by this test backend",
        ))
    }

    fn dispatch_timer(&mut self, _: CallbackId) -> Result<CommandBatch, BackendError> {
        Err(BackendError::recoverable(
            Status::HostError,
            "repeating timer callback failed",
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

#[test]
fn retired_repeating_timer_does_not_block_native_host_release() {
    reset();
    // SAFETY: The recording function table implements the native renderer ABI for this test.
    let mounted = unsafe {
        lynx_element_bridge_ffi::native_mount(Some(get_api), HOST, |session, _| {
            Ok((
                Box::new(FailingRepeatingTimerBackend { session }),
                CommandBatch {
                    session,
                    sequence: 0,
                    commands: Vec::new(),
                    final_commit: true,
                },
            ))
        })
    };
    assert_eq!(mounted.status, NATIVE_STATUS_OK);
    let callback_table = callbacks();
    let (timer, callback, repeating) = timer_registration();
    assert!(repeating);

    // SAFETY: This simulates the native renderer firing and retiring a repeating timer whose
    // Rust callback returns non-OK.
    assert_eq!(
        unsafe { fire_timer(callback_table, timer, callback) },
        NATIVE_STATUS_HOST_ERROR
    );
    clear_calls();

    let destroyed = lynx_element_bridge_ffi::native_destroy_session(mounted.session);
    assert_eq!(
        (destroyed.status, destroyed.consumed),
        (NATIVE_STATUS_OK, 1)
    );
    assert!(
        !calls()
            .iter()
            .any(|call| matches!(call, Call::CancelTimer(_)))
    );
    assert_eq!(
        calls()
            .iter()
            .filter(|call| **call == Call::Release)
            .count(),
        1
    );
    assert_eq!(calls().last(), Some(&Call::Release));
}

struct PanickingTimerBackend {
    session: SessionId,
}

impl BridgeBackend for PanickingTimerBackend {
    fn initial_native_timers(&self) -> Vec<NativeTimerRequest> {
        vec![NativeTimerRequest {
            delay_millis: 1,
            repeating: false,
            callback: CallbackId::new(91).unwrap(),
        }]
    }

    fn dispatch_event(&mut self, _: EventMessage) -> Result<CommandBatch, BackendError> {
        Err(BackendError::recoverable(
            Status::Unsupported,
            "events are not used by this test backend",
        ))
    }

    fn dispatch_timer(&mut self, _: CallbackId) -> Result<CommandBatch, BackendError> {
        panic!("contained timer panic")
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

#[test]
fn native_timer_panics_are_contained_and_poison_the_session() {
    reset();
    // SAFETY: The recording function table implements the native renderer ABI for this test.
    let mounted = unsafe {
        lynx_element_bridge_ffi::native_mount(Some(get_api), HOST, |session, _| {
            Ok((
                Box::new(PanickingTimerBackend { session }),
                CommandBatch {
                    session,
                    sequence: 0,
                    commands: Vec::new(),
                    final_commit: true,
                },
            ))
        })
    };
    assert_eq!(mounted.status, NATIVE_STATUS_OK);
    let callback_table = callbacks();
    let (timer, callback, _) = timer_registration();

    // SAFETY: The captured one-shot timer identity remains live for this callback.
    let panicked = unsafe {
        send_timer(
            callback_table,
            callback_table.context,
            RENDERER,
            timer,
            callback,
        )
    };
    assert_eq!(panicked, NATIVE_STATUS_PANIC);
    // SAFETY: The callback data is valid but the session is permanently poisoned.
    let poisoned = unsafe {
        send_timer(
            callback_table,
            callback_table.context,
            RENDERER,
            timer,
            callback,
        )
    };
    assert_eq!(poisoned, NATIVE_STATUS_HOST_ERROR);

    let destroyed = lynx_element_bridge_ffi::native_destroy_session(mounted.session);
    assert_eq!(destroyed.status, NATIVE_STATUS_HOST_ERROR);
    assert_eq!(destroyed.consumed, 1);
    assert_eq!(calls().last(), Some(&Call::Release));
}

#[test]
fn native_mount_rejects_missing_inputs_without_acquiring() {
    reset();
    // SAFETY: A missing resolver is represented explicitly and rejected before invocation.
    let no_api = unsafe { lynx_element_bridge_native_mount(None, HOST) };
    assert_eq!(no_api.status, NATIVE_STATUS_INVALID_ARGUMENT);
    assert_eq!(no_api.session, 0);
    // SAFETY: A zero host is rejected before the valid resolver is invoked.
    let no_host = unsafe { lynx_element_bridge_native_mount(Some(get_api), 0) };
    assert_eq!(no_host.status, NATIVE_STATUS_INVALID_ARGUMENT);
    assert_eq!(no_host.session, 0);
    assert!(calls().is_empty());
}

#[test]
fn native_mount_failure_does_not_publish_a_session_and_releases_the_renderer() {
    reset();
    RECORDER.with(|recorder| recorder.borrow_mut().failure = Some("flush"));

    // SAFETY: The recording resolver and table implement the production ABI for this test.
    let mounted = unsafe { lynx_element_bridge_native_mount(Some(get_api), HOST) };
    assert_eq!(mounted.status, NATIVE_STATUS_HOST_ERROR);
    assert_eq!(mounted.session, 0);
    assert_eq!(calls().last(), Some(&Call::Release));
    assert_eq!(
        calls()
            .iter()
            .filter(|call| **call == Call::Release)
            .count(),
        1
    );

    reset();
    RECORDER.with(|recorder| recorder.borrow_mut().failure = Some("create_timer"));
    // SAFETY: The recording resolver and table implement the production ABI for this test.
    let mounted = unsafe { lynx_element_bridge_native_mount(Some(get_api), HOST) };
    assert_eq!(mounted.status, NATIVE_STATUS_HOST_ERROR);
    assert_eq!(mounted.session, 0);
    assert!(
        calls()
            .iter()
            .any(|call| matches!(call, Call::CreateTimer { .. }))
    );
    assert_eq!(calls().last(), Some(&Call::Release));
    assert_eq!(
        calls()
            .iter()
            .filter(|call| **call == Call::Release)
            .count(),
        1
    );
}
