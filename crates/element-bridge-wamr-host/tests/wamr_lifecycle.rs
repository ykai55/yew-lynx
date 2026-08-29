#![cfg(feature = "wamr")]

use std::collections::HashSet;
use std::ffi::c_void;
use std::fmt::Write;
use std::mem;
use std::path::PathBuf;
use std::process::Command;
use std::ptr;
use std::sync::{LazyLock, Mutex};

use lynx_element_bridge_core::{NodeId, Status};
use lynx_element_bridge_ffi::native_host::*;
use lynx_element_bridge_wamr_host::{WamrBackend, mount_module, replace_module};
use lynx_element_bridge_wasm_guest::{
    GuestResponse, GuestResult, PROTOCOL_VERSION_V2, encode_guest_response,
};

const HOST: NativeHostHandle = 7;
const RENDERER: NativeRendererHandle = 8;
const ROOT: NativeNodeHandle = 100;

#[derive(Clone, Copy)]
struct StoredCallbacks {
    context: usize,
    on_event: NativeOnEventFn,
}

struct RendererState {
    callbacks: Option<StoredCallbacks>,
    nodes: HashSet<NativeNodeHandle>,
    listeners: HashSet<NativeListenerHandle>,
    attributes: Vec<(NativeNodeHandle, String, Option<String>)>,
    reentrant_replace: Option<(u32, Vec<u8>)>,
    reentrant_status: Option<NativeStatus>,
    next_node: NativeNodeHandle,
    releases: u32,
}

impl Default for RendererState {
    fn default() -> Self {
        Self {
            callbacks: None,
            nodes: HashSet::from([ROOT]),
            listeners: HashSet::new(),
            attributes: Vec::new(),
            reentrant_replace: None,
            reentrant_status: None,
            next_node: 200,
            releases: 0,
        }
    }
}

static STATE: LazyLock<Mutex<RendererState>> =
    LazyLock::new(|| Mutex::new(RendererState::default()));
static TEST_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" fn acquire(
    host: NativeHostHandle,
    callbacks: *const NativeRendererCallbacksV1,
    renderer: *mut NativeRendererHandle,
) -> NativeStatus {
    if host != HOST || callbacks.is_null() || renderer.is_null() {
        return NATIVE_STATUS_INVALID_ARGUMENT;
    }
    // SAFETY: NativeHost supplies readable callbacks and a writable renderer output.
    let callbacks = unsafe { *callbacks };
    let Some(on_event) = callbacks.on_event else {
        return NATIVE_STATUS_INVALID_ARGUMENT;
    };
    STATE.lock().unwrap().callbacks = Some(StoredCallbacks {
        context: callbacks.context as usize,
        on_event,
    });
    // SAFETY: Validated above.
    unsafe { *renderer = RENDERER };
    NATIVE_STATUS_OK
}

unsafe extern "C" fn release(renderer: NativeRendererHandle) -> NativeStatus {
    if renderer != RENDERER {
        return NATIVE_STATUS_INVALID_SESSION;
    }
    let mut state = STATE.lock().unwrap();
    state.releases += 1;
    state.callbacks = None;
    NATIVE_STATUS_OK
}

unsafe extern "C" fn get_root(
    renderer: NativeRendererHandle,
    root: *mut NativeNodeHandle,
) -> NativeStatus {
    if renderer != RENDERER || root.is_null() {
        return NATIVE_STATUS_INVALID_ARGUMENT;
    }
    // SAFETY: Validated above.
    unsafe { *root = ROOT };
    NATIVE_STATUS_OK
}

unsafe extern "C" fn create_element(
    _: NativeRendererHandle,
    _: NativeUtf8,
    output: *mut NativeNodeHandle,
) -> NativeStatus {
    create_node(output)
}

unsafe extern "C" fn create_raw_text(
    _: NativeRendererHandle,
    _: NativeUtf8,
    output: *mut NativeNodeHandle,
) -> NativeStatus {
    create_node(output)
}

fn create_node(output: *mut NativeNodeHandle) -> NativeStatus {
    if output.is_null() {
        return NATIVE_STATUS_INVALID_ARGUMENT;
    }
    let mut state = STATE.lock().unwrap();
    let node = state.next_node;
    state.next_node += 1;
    state.nodes.insert(node);
    // SAFETY: The caller supplied a nonnull output pointer.
    unsafe { *output = node };
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
    node: NativeNodeHandle,
    name: NativeUtf8,
    value: NativeUtf8,
) -> NativeStatus {
    // SAFETY: NativeHost supplies valid borrowed UTF-8 spans.
    let name = unsafe { text(name) };
    let value = if value.data.is_null() {
        None
    } else {
        // SAFETY: A nonnull optional value is a valid borrowed span.
        Some(unsafe { text(value) })
    };
    let replacement = {
        let mut state = STATE.lock().unwrap();
        state.attributes.push((node, name, value));
        state.reentrant_replace.take()
    };
    if let Some((session, module)) = replacement {
        let status = replace_module(session, &module);
        STATE.lock().unwrap().reentrant_status = Some(status);
    }
    NATIVE_STATUS_OK
}

unsafe fn text(value: NativeUtf8) -> String {
    // SAFETY: The native renderer ABI guarantees a readable borrowed span.
    String::from_utf8(unsafe { std::slice::from_raw_parts(value.data, value.len) }.to_vec())
        .unwrap()
}

unsafe extern "C" fn insert_before(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    _: NativeNodeHandle,
    _: NativeNodeHandle,
) -> NativeStatus {
    NATIVE_STATUS_OK
}

unsafe extern "C" fn remove_child(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    _: NativeNodeHandle,
) -> NativeStatus {
    NATIVE_STATUS_OK
}

unsafe extern "C" fn destroy_node(_: NativeRendererHandle, node: NativeNodeHandle) -> NativeStatus {
    if STATE.lock().unwrap().nodes.remove(&node) {
        NATIVE_STATUS_OK
    } else {
        NATIVE_STATUS_INVALID_OWNERSHIP
    }
}

unsafe extern "C" fn add_listener(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    listener: NativeListenerHandle,
    _: NativeCallbackHandle,
    _: NativeUtf8,
) -> NativeStatus {
    if STATE.lock().unwrap().listeners.insert(listener) {
        NATIVE_STATUS_OK
    } else {
        NATIVE_STATUS_INVALID_LISTENER
    }
}

unsafe extern "C" fn remove_listener(
    _: NativeRendererHandle,
    _: NativeNodeHandle,
    listener: NativeListenerHandle,
    _: NativeCallbackHandle,
    _: NativeUtf8,
) -> NativeStatus {
    if STATE.lock().unwrap().listeners.remove(&listener) {
        NATIVE_STATUS_OK
    } else {
        NATIVE_STATUS_INVALID_LISTENER
    }
}

unsafe extern "C" fn flush(_: NativeRendererHandle) -> NativeStatus {
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

static API: NativeRendererApiV1 = NativeRendererApiV1 {
    abi_version: NATIVE_RENDERER_ABI_VERSION,
    struct_size: mem::size_of::<NativeRendererApiV1>(),
    acquire: Some(acquire),
    release: Some(release),
    get_root: Some(get_root),
    create_element: Some(create_element),
    create_raw_text: Some(create_raw_text),
    set_raw_text: Some(set_raw_text),
    set_attribute: Some(set_attribute),
    insert_before: Some(insert_before),
    remove_child: Some(remove_child),
    destroy_node: Some(destroy_node),
    add_event_listener: Some(add_listener),
    remove_event_listener: Some(remove_listener),
    flush: Some(flush),
    create_timer: Some(create_timer),
    cancel_timer: Some(cancel_timer),
};

unsafe extern "C" fn get_api(version: u32) -> *const NativeRendererApiV1 {
    if version == NATIVE_RENDERER_ABI_VERSION {
        ptr::addr_of!(API)
    } else {
        ptr::null()
    }
}

#[test]
fn real_wamr_runs_mount_event_replace_and_destroy_through_native_renderer() {
    let _serial = TEST_LOCK.lock().unwrap();
    *STATE.lock().unwrap() = RendererState::default();
    let module = fixture_module();

    // SAFETY: The test API implements the full native renderer V1 contract.
    let mounted = unsafe { mount_module(Some(get_api), HOST, &module) };
    assert_eq!(mounted.status, NATIVE_STATUS_OK);
    assert_ne!(mounted.session, 0);
    assert_eq!(STATE.lock().unwrap().listeners, HashSet::from([1]));
    let old_native_listener = *STATE.lock().unwrap().listeners.iter().next().unwrap();
    assert!(
        STATE
            .lock()
            .unwrap()
            .attributes
            .iter()
            .any(|(_, name, value)| {
                name == "data-state" && value.as_deref() == Some("mounted")
            })
    );

    let callbacks = STATE.lock().unwrap().callbacks.unwrap();
    STATE.lock().unwrap().reentrant_replace = Some((mounted.session, module.clone()));
    // SAFETY: The renderer synchronously invokes the callback identity registered by NativeHost.
    let event_status = unsafe {
        (callbacks.on_event)(
            callbacks.context as *mut c_void,
            RENDERER,
            1,
            1,
            span("tap"),
            span("application/octet-stream"),
            NativeBytes {
                data: ptr::null(),
                len: 0,
            },
        )
    };
    assert_eq!(event_status, NATIVE_STATUS_OK);
    assert_eq!(
        STATE.lock().unwrap().reentrant_status,
        Some(NATIVE_STATUS_HOST_ERROR)
    );
    assert!(
        STATE
            .lock()
            .unwrap()
            .attributes
            .iter()
            .any(|(_, name, value)| { name == "data-state" && value.as_deref() == Some("event") })
    );

    let missing =
        wat::parse_str("(module (func (export \"version\") (result i32) i32.const 2))").unwrap();
    assert_eq!(
        replace_module(mounted.session, &missing),
        NATIVE_STATUS_UNSUPPORTED
    );
    assert_eq!(STATE.lock().unwrap().listeners, HashSet::from([1]));
    assert_eq!(STATE.lock().unwrap().nodes.len(), 2);

    assert_eq!(replace_module(mounted.session, &module), NATIVE_STATUS_OK);
    let state = STATE.lock().unwrap();
    assert_eq!(state.releases, 0, "replace must retain the renderer");
    assert_eq!(state.listeners.len(), 1);
    let new_native_listener = *state.listeners.iter().next().unwrap();
    assert_ne!(new_native_listener, old_native_listener);
    assert_eq!(state.nodes.len(), 2, "old application nodes must be gone");
    let event_updates_before = state
        .attributes
        .iter()
        .filter(|(_, name, value)| name == "data-state" && value.as_deref() == Some("event"))
        .count();
    drop(state);

    // SAFETY: This deliberately replays an event queued for the removed native listener.
    let stale_event_status = unsafe {
        (callbacks.on_event)(
            callbacks.context as *mut c_void,
            RENDERER,
            old_native_listener,
            1,
            span("tap"),
            span("application/octet-stream"),
            NativeBytes {
                data: ptr::null(),
                len: 0,
            },
        )
    };
    assert_eq!(stale_event_status, NATIVE_STATUS_INVALID_LISTENER);
    assert_eq!(
        STATE
            .lock()
            .unwrap()
            .attributes
            .iter()
            .filter(|(_, name, value)| {
                name == "data-state" && value.as_deref() == Some("event")
            })
            .count(),
        event_updates_before
    );

    // SAFETY: This uses the native listener identity registered by the replacement guest.
    let new_event_status = unsafe {
        (callbacks.on_event)(
            callbacks.context as *mut c_void,
            RENDERER,
            new_native_listener,
            1,
            span("tap"),
            span("application/octet-stream"),
            NativeBytes {
                data: ptr::null(),
                len: 0,
            },
        )
    };
    assert_eq!(new_event_status, NATIVE_STATUS_OK);
    assert_eq!(
        STATE
            .lock()
            .unwrap()
            .attributes
            .iter()
            .filter(|(_, name, value)| {
                name == "data-state" && value.as_deref() == Some("event")
            })
            .count(),
        event_updates_before + 1
    );

    let destroyed =
        lynx_element_bridge_wamr_host::lynx_element_bridge_wamr_destroy(mounted.session);
    assert_eq!(destroyed.status, NATIVE_STATUS_OK);
    assert_eq!(destroyed.consumed, 1);
    let state = STATE.lock().unwrap();
    assert_eq!(state.releases, 1);
    assert!(state.listeners.is_empty());
    assert_eq!(state.nodes, HashSet::from([ROOT]));
}

#[test]
fn real_wamr_runs_yew_guest_mount_event_and_destroy_through_native_renderer() {
    let _serial = TEST_LOCK.lock().unwrap();
    *STATE.lock().unwrap() = RendererState::default();
    let module = yew_counter_module();

    // SAFETY: The test API implements the full native renderer V1 contract.
    let mounted = unsafe { mount_module(Some(get_api), HOST, &module) };
    assert_eq!(mounted.status, NATIVE_STATUS_OK);
    assert_ne!(mounted.session, 0);
    assert_eq!(STATE.lock().unwrap().listeners, HashSet::from([1]));
    assert!(STATE.lock().unwrap().nodes.len() > 2);

    let callbacks = STATE.lock().unwrap().callbacks.unwrap();
    // SAFETY: The renderer synchronously invokes the callback identity registered by YewAdapter.
    let event_status = unsafe {
        (callbacks.on_event)(
            callbacks.context as *mut c_void,
            RENDERER,
            1,
            1,
            span("tap"),
            span("application/vnd.lynx.tap"),
            NativeBytes {
                data: ptr::null(),
                len: 0,
            },
        )
    };
    assert_eq!(event_status, NATIVE_STATUS_OK);

    let destroyed =
        lynx_element_bridge_wamr_host::lynx_element_bridge_wamr_destroy(mounted.session);
    assert_eq!(
        (destroyed.status, destroyed.consumed),
        (NATIVE_STATUS_OK, 1)
    );
    let state = STATE.lock().unwrap();
    assert_eq!(state.releases, 1);
    assert!(state.listeners.is_empty());
    assert_eq!(state.nodes, HashSet::from([ROOT]));
}

#[test]
fn preflight_rejects_bad_version_and_missing_exports_and_contains_traps() {
    let _serial = TEST_LOCK.lock().unwrap();
    let bad_version = wat::parse_str(module_wat("(i32.const 1)", "(i64.const 0)")).unwrap();
    assert_eq!(
        WamrBackend::preflight(&bad_version).err().unwrap().status,
        Status::Unsupported
    );

    let missing =
        wat::parse_str("(module (func (export \"version\") (result i32) i32.const 2))").unwrap();
    assert_eq!(
        WamrBackend::preflight(&missing).err().unwrap().status,
        Status::Unsupported
    );

    let trapped = wat::parse_str(module_wat("(i32.const 2)", "unreachable")).unwrap();
    let error = match WamrBackend::preflight(&trapped)
        .unwrap()
        .mount(NodeId::new(1).unwrap())
    {
        Ok(_) => panic!("trapping guest unexpectedly mounted"),
        Err(error) => error,
    };
    assert_eq!(error.status, Status::Panic);
}

#[test]
fn real_wamr_rejects_error_response_with_ok_status_for_mount_and_replace() {
    let _serial = TEST_LOCK.lock().unwrap();
    *STATE.lock().unwrap() = RendererState::default();
    let invalid = error_with_ok_status_module();

    // SAFETY: The test API implements the full native renderer V1 contract.
    let rejected = unsafe { mount_module(Some(get_api), HOST, &invalid) };
    assert_eq!(rejected.status, NATIVE_STATUS_INVALID_ARGUMENT);
    assert_eq!(rejected.session, 0);

    // SAFETY: The test API implements the full native renderer V1 contract.
    let mounted = unsafe { mount_module(Some(get_api), HOST, &fixture_module()) };
    assert_eq!(mounted.status, NATIVE_STATUS_OK);
    assert_ne!(mounted.session, 0);
    assert_eq!(
        replace_module(mounted.session, &invalid),
        NATIVE_STATUS_INVALID_ARGUMENT
    );

    let destroyed =
        lynx_element_bridge_wamr_host::lynx_element_bridge_wamr_destroy(mounted.session);
    assert_eq!(destroyed.consumed, 1);
}

#[test]
fn real_wamr_rejects_invalid_guest_outputs_without_reading_out_of_bounds() {
    let _serial = TEST_LOCK.lock().unwrap();
    let truncated = {
        let mut response = encode_guest_response(&GuestResponse {
            protocol_version: PROTOCOL_VERSION_V2,
            result: GuestResult::Err {
                status: Status::HostError,
                message: "guest failure".into(),
            },
        })
        .unwrap();
        response.pop();
        response
    };
    let v1_response = encode_guest_response(&GuestResponse {
        protocol_version: 1,
        result: GuestResult::Err {
            status: Status::HostError,
            message: "v1 response".into(),
        },
    })
    .unwrap();
    let cases = [
        (
            "zero descriptor",
            output_module(&[], 0, 1),
            Status::InvalidArgument,
            "guest returned an empty output descriptor",
        ),
        (
            "out-of-bounds descriptor",
            output_module(&[], (65_535_u64 << 32) | 2, 1),
            Status::InvalidArgument,
            "guest memory range 65535..+2 is invalid",
        ),
        (
            "overflowing descriptor",
            output_module(&[], (u64::from(u32::MAX - 1) << 32) | 4, 1),
            Status::InvalidArgument,
            "guest memory range 4294967294..+4 is invalid",
        ),
        (
            "v1 response",
            output_module(&v1_response, (1024_u64 << 32) | v1_response.len() as u64, 1),
            Status::Unsupported,
            "unsupported protocol version 1",
        ),
        (
            "malformed postcard",
            output_module(&[1, 2], (1024_u64 << 32) | 2, 1),
            Status::InvalidArgument,
            "invalid postcard message: Serde Deserialization Error",
        ),
        (
            "truncated postcard",
            output_module(&truncated, (1024_u64 << 32) | truncated.len() as u64, 1),
            Status::InvalidArgument,
            "invalid postcard message: Hit the end of buffer, expected more data",
        ),
        (
            "rejected output deallocation",
            output_module(&[1], (1024_u64 << 32) | 1, 0),
            Status::HostError,
            "guest rejected its output deallocation",
        ),
    ];

    for (name, module, expected_status, expected_message) in cases {
        let error = match WamrBackend::preflight(&module)
            .unwrap()
            .mount(NodeId::new(1).unwrap())
        {
            Ok(_) => panic!("{name} unexpectedly mounted"),
            Err(error) => error,
        };
        assert_eq!(error.status, expected_status, "{name}");
        assert_eq!(error.message, expected_message, "{name}");
    }
}

#[test]
fn real_wamr_replace_output_failure_cleans_up_and_poisoned_session_remains_consumable() {
    let _serial = TEST_LOCK.lock().unwrap();
    *STATE.lock().unwrap() = RendererState::default();

    // SAFETY: The test API implements the full native renderer V1 contract.
    let mounted = unsafe { mount_module(Some(get_api), HOST, &fixture_module()) };
    assert_eq!(mounted.status, NATIVE_STATUS_OK);

    let malformed = output_module(&[1, 2], (1024_u64 << 32) | 2, 1);
    assert_eq!(
        replace_module(mounted.session, &malformed),
        NATIVE_STATUS_INVALID_ARGUMENT
    );
    let state = STATE.lock().unwrap();
    assert!(state.listeners.is_empty());
    assert_eq!(state.nodes, HashSet::from([ROOT]));
    drop(state);

    assert_eq!(
        replace_module(mounted.session, &fixture_module()),
        NATIVE_STATUS_HOST_ERROR
    );
    let destroyed =
        lynx_element_bridge_wamr_host::lynx_element_bridge_wamr_destroy(mounted.session);
    assert_eq!(
        (destroyed.status, destroyed.consumed),
        (NATIVE_STATUS_HOST_ERROR, 1)
    );
    assert_eq!(STATE.lock().unwrap().releases, 1);
}

fn module_wat(version_body: &str, lifecycle_body: &str) -> String {
    format!(
        r#"(module
            (memory 1)
            (func (export "version") (result i32) {version_body})
            (func (export "alloc") (param i32) (result i32) i32.const 8)
            (func (export "dealloc") (param i32 i32) (result i32) i32.const 1)
            (func (export "mount") (param i32 i32) (result i64) {lifecycle_body})
            (func (export "dispatch_event") (param i32 i32) (result i64) i64.const 0)
            (func (export "destroy") (result i64) i64.const 0)
            (func (export "output_dealloc") (param i32 i32) (result i32) i32.const 1))"#
    )
}

fn fixture_module() -> Vec<u8> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = workspace.join("target/wamr-fixture");
    let status = Command::new(env!("CARGO"))
        .current_dir(&workspace)
        .args([
            "build",
            "--locked",
            "--release",
            "--target",
            "wasm32-wasip1",
            "-p",
            "lynx-element-bridge-wasm-guest",
            "--example",
            "abi_fixture",
            "--target-dir",
        ])
        .arg(&target)
        .status()
        .expect("build wasm fixture");
    assert!(status.success(), "WASM fixture build failed: {status}");
    std::fs::read(target.join("wasm32-wasip1/release/examples/abi_fixture.wasm"))
        .expect("read built wasm fixture")
}

fn error_with_ok_status_module() -> Vec<u8> {
    let response = encode_guest_response(&GuestResponse {
        protocol_version: PROTOCOL_VERSION_V2,
        result: GuestResult::Err {
            status: Status::Ok,
            message: "invalid guest error".into(),
        },
    })
    .unwrap();
    output_module(&response, (1024_u64 << 32) | response.len() as u64, 1)
}

fn output_module(bytes: &[u8], descriptor: u64, output_dealloc_result: u32) -> Vec<u8> {
    let mut data = String::new();
    for byte in bytes {
        write!(data, "\\{byte:02x}").unwrap();
    }
    wat::parse_str(format!(
        r#"(module
            (memory 1)
            (data (i32.const 1024) "{data}")
            (func (export "version") (result i32) i32.const 2)
            (func (export "alloc") (param i32) (result i32) i32.const 8)
            (func (export "dealloc") (param i32 i32) (result i32) i32.const 1)
            (func (export "mount") (param i32 i32) (result i64)
                i64.const {})
            (func (export "dispatch_event") (param i32 i32) (result i64)
                i64.const {})
            (func (export "destroy") (result i64) i64.const {})
            (func (export "output_dealloc") (param i32 i32) (result i32)
                i32.const {output_dealloc_result}))"#,
        descriptor, descriptor, descriptor,
    ))
    .unwrap()
}

fn yew_counter_module() -> Vec<u8> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = workspace.join("target/wamr-yew-counter");
    let status = Command::new(env!("CARGO"))
        .current_dir(&workspace)
        .args([
            "build",
            "--locked",
            "--release",
            "--target",
            "wasm32-wasip1",
            "-p",
            "yew-lynx-counter",
            "--target-dir",
        ])
        .arg(&target)
        .status()
        .expect("build Yew WASM counter");
    assert!(status.success(), "Yew WASM counter build failed: {status}");
    std::fs::read(target.join("wasm32-wasip1/release/yew_lynx_counter.wasm"))
        .expect("read built Yew WASM counter")
}

fn span(value: &'static str) -> NativeUtf8 {
    NativeUtf8 {
        data: value.as_ptr(),
        len: value.len(),
    }
}
