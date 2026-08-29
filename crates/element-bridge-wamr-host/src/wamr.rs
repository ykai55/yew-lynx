use std::ffi::{CStr, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::OnceLock;

use lynx_element_bridge_core::{CommandBatch, EventMessage, NodeId, SessionId, Status};
use lynx_element_bridge_ffi::native_host::{
    NATIVE_STATUS_PANIC, NativeHostHandle, NativeRendererGetApiFn, NativeStatus, status_to_native,
};
use lynx_element_bridge_ffi::{
    BackendError, BridgeBackend, BridgeBackendCandidate, LynxElementBridgeNativeDestroyResult,
    LynxElementBridgeNativeMountResult, LynxElementBridgeSession, native_abandon_session,
    native_destroy_session, native_mount, native_replace_backend,
};
use lynx_element_bridge_wasm_guest::{
    EventRequest, GuestResponse, GuestResult, MountRequest, PROTOCOL_VERSION_V1,
    decode_guest_response, encode_event_request, encode_mount_request,
};

const ERROR_BUFFER_SIZE: usize = 256;
const EXECUTION_STACK_SIZE: u32 = 256 * 1024;
const WASM_I32: u8 = 0;
const WASM_I64: u8 = 1;

#[repr(C)]
struct WasmModuleOpaque {
    _private: [u8; 0],
}

#[repr(C)]
struct WasmModuleInstanceOpaque {
    _private: [u8; 0],
}

#[repr(C)]
struct WasmFunctionInstanceOpaque {
    _private: [u8; 0],
}

#[repr(C)]
struct WasmExecutionEnvironmentOpaque {
    _private: [u8; 0],
}

type WasmModule = *mut WasmModuleOpaque;
type WasmModuleInstance = *mut WasmModuleInstanceOpaque;
type WasmFunctionInstance = *mut WasmFunctionInstanceOpaque;
type WasmExecutionEnvironment = *mut WasmExecutionEnvironmentOpaque;

unsafe extern "C" {
    fn wasm_runtime_init() -> bool;
    fn wasm_runtime_init_thread_env() -> bool;
    fn wasm_runtime_destroy_thread_env();
    fn wasm_runtime_load(
        buffer: *mut u8,
        size: u32,
        error_buffer: *mut c_char,
        error_buffer_size: u32,
    ) -> WasmModule;
    fn wasm_runtime_unload(module: WasmModule);
    fn wasm_runtime_instantiate(
        module: WasmModule,
        stack_size: u32,
        heap_size: u32,
        error_buffer: *mut c_char,
        error_buffer_size: u32,
    ) -> WasmModuleInstance;
    fn wasm_runtime_deinstantiate(instance: WasmModuleInstance);
    fn wasm_runtime_lookup_function(
        instance: WasmModuleInstance,
        name: *const c_char,
    ) -> WasmFunctionInstance;
    fn wasm_func_get_param_count(
        function: WasmFunctionInstance,
        instance: WasmModuleInstance,
    ) -> u32;
    fn wasm_func_get_result_count(
        function: WasmFunctionInstance,
        instance: WasmModuleInstance,
    ) -> u32;
    fn wasm_func_get_param_types(
        function: WasmFunctionInstance,
        instance: WasmModuleInstance,
        types: *mut u8,
    );
    fn wasm_func_get_result_types(
        function: WasmFunctionInstance,
        instance: WasmModuleInstance,
        types: *mut u8,
    );
    fn wasm_runtime_create_exec_env(
        instance: WasmModuleInstance,
        stack_size: u32,
    ) -> WasmExecutionEnvironment;
    fn wasm_runtime_destroy_exec_env(environment: WasmExecutionEnvironment);
    fn wasm_runtime_call_wasm(
        environment: WasmExecutionEnvironment,
        function: WasmFunctionInstance,
        argc: u32,
        argv: *mut u32,
    ) -> bool;
    fn wasm_runtime_get_exception(instance: WasmModuleInstance) -> *const c_char;
    fn wasm_runtime_validate_app_addr(instance: WasmModuleInstance, offset: u64, size: u64)
    -> bool;
    fn wasm_runtime_addr_app_to_native(instance: WasmModuleInstance, offset: u64) -> *mut c_void;
}

#[derive(Clone, Copy)]
struct Exports {
    alloc: WasmFunctionInstance,
    dealloc: WasmFunctionInstance,
    mount: WasmFunctionInstance,
    dispatch_event: WasmFunctionInstance,
    destroy: WasmFunctionInstance,
    output_dealloc: WasmFunctionInstance,
}

struct WamrGuest {
    environment: WasmExecutionEnvironment,
    instance: WasmModuleInstance,
    module: WasmModule,
    exports: Exports,
    _binary: Vec<u8>,
}

impl WamrGuest {
    fn preflight(module_bytes: &[u8]) -> Result<Self, BackendError> {
        initialize_runtime()?;
        let size = u32::try_from(module_bytes.len()).map_err(|_| {
            BackendError::recoverable(Status::InvalidArgument, "WASM module is larger than 4 GiB")
        })?;
        if size == 0 {
            return Err(BackendError::recoverable(
                Status::InvalidArgument,
                "WASM module must not be empty",
            ));
        }
        let mut binary = module_bytes.to_vec();
        let mut error = [0 as c_char; ERROR_BUFFER_SIZE];
        // SAFETY: WAMR receives the writable binary for as long as the loaded module remains live.
        let module = unsafe {
            wasm_runtime_load(
                binary.as_mut_ptr(),
                size,
                error.as_mut_ptr(),
                ERROR_BUFFER_SIZE as u32,
            )
        };
        if module.is_null() {
            return Err(wamr_error("load module", &error));
        }
        // SAFETY: `module` is live and the error span is writable.
        let instance = unsafe {
            wasm_runtime_instantiate(
                module,
                EXECUTION_STACK_SIZE,
                0,
                error.as_mut_ptr(),
                ERROR_BUFFER_SIZE as u32,
            )
        };
        if instance.is_null() {
            // SAFETY: This is the sole unload for the successfully loaded module.
            unsafe { wasm_runtime_unload(module) };
            return Err(wamr_error("instantiate module", &error));
        }
        // SAFETY: `instance` is live and the requested stack size is nonzero.
        let environment = unsafe { wasm_runtime_create_exec_env(instance, EXECUTION_STACK_SIZE) };
        if environment.is_null() {
            // SAFETY: These release the live instance and module in dependency order.
            unsafe {
                wasm_runtime_deinstantiate(instance);
                wasm_runtime_unload(module);
            }
            return Err(BackendError::fatal(
                Status::ResourceExhausted,
                "WAMR could not allocate an execution environment",
            ));
        }

        let validated = (|| {
            let initialize = lookup_optional(instance, c"_initialize", &[], &[])?
                .or(lookup_optional(instance, c"__wasm_call_ctors", &[], &[])?)
                .or(lookup_optional(instance, c"_start", &[], &[])?);
            let version = lookup(instance, c"version", &[], &[WASM_I32])?;
            let exports = Exports {
                alloc: lookup(instance, c"alloc", &[WASM_I32], &[WASM_I32])?,
                dealloc: lookup(instance, c"dealloc", &[WASM_I32, WASM_I32], &[WASM_I32])?,
                mount: lookup(instance, c"mount", &[WASM_I32, WASM_I32], &[WASM_I64])?,
                dispatch_event: lookup(
                    instance,
                    c"dispatch_event",
                    &[WASM_I32, WASM_I32],
                    &[WASM_I64],
                )?,
                destroy: lookup(instance, c"destroy", &[], &[WASM_I64])?,
                output_dealloc: lookup(
                    instance,
                    c"output_dealloc",
                    &[WASM_I32, WASM_I32],
                    &[WASM_I32],
                )?,
            };
            Ok((initialize, version, exports))
        })();
        let (initialize, version, exports) = match validated {
            Ok(exports) => exports,
            Err(error) => {
                // SAFETY: Ownership was not transferred when export validation failed.
                unsafe {
                    wasm_runtime_destroy_exec_env(environment);
                    wasm_runtime_deinstantiate(instance);
                    wasm_runtime_unload(module);
                }
                return Err(error);
            }
        };
        let mut guest = Self {
            environment,
            instance,
            module,
            exports,
            _binary: binary,
        };
        if let Some(initialize) = initialize {
            let mut cells = [0; 2];
            guest.call(initialize, 0, &mut cells)?;
        }
        let actual = guest.call_i32(version, &[])?;
        if actual != PROTOCOL_VERSION_V1 {
            return Err(BackendError::recoverable(
                Status::Unsupported,
                format!("unsupported guest protocol version {actual}"),
            ));
        }
        Ok(guest)
    }

    fn request(
        &mut self,
        function: WasmFunctionInstance,
        input: &[u8],
    ) -> Result<CommandBatch, BackendError> {
        let length = u32::try_from(input.len()).map_err(|_| {
            BackendError::recoverable(
                Status::InvalidArgument,
                "guest request is larger than 4 GiB",
            )
        })?;
        let pointer = self.call_i32(self.exports.alloc, &[length])?;
        if pointer == 0 && length != 0 {
            return Err(BackendError::fatal(
                Status::ResourceExhausted,
                "guest alloc returned a null pointer",
            ));
        }
        self.write_memory(pointer, input)?;
        let response = self.call_i64(function, &[pointer, length]);
        let deallocated = self.call_i32(self.exports.dealloc, &[pointer, length]);
        if deallocated? != 1 {
            return Err(BackendError::fatal(
                Status::HostError,
                "guest rejected its input deallocation",
            ));
        }
        self.decode_output(response?)
    }

    fn destroy_application(&mut self) -> Result<CommandBatch, BackendError> {
        let descriptor = self.call_i64(self.exports.destroy, &[])?;
        self.decode_output(descriptor)
    }

    fn decode_output(&mut self, descriptor: u64) -> Result<CommandBatch, BackendError> {
        let pointer = (descriptor >> 32) as u32;
        let length = descriptor as u32;
        if pointer == 0 || length == 0 {
            return Err(BackendError::fatal(
                Status::InvalidArgument,
                "guest returned an empty output descriptor",
            ));
        }
        let bytes = self.read_memory(pointer, length)?.to_vec();
        if self.call_i32(self.exports.output_dealloc, &[pointer, length])? != 1 {
            return Err(BackendError::fatal(
                Status::HostError,
                "guest rejected its output deallocation",
            ));
        }
        let response: GuestResponse = decode_guest_response(&bytes).map_err(BackendError::from)?;
        match response.result {
            GuestResult::Ok(batch) => Ok(batch),
            GuestResult::Err {
                status: Status::Ok, ..
            } => Err(BackendError::fatal(
                Status::InvalidArgument,
                "guest error response used an OK status",
            )),
            GuestResult::Err { status, message } => Err(BackendError::recoverable(status, message)),
        }
    }

    fn call_i32(
        &mut self,
        function: WasmFunctionInstance,
        arguments: &[u32],
    ) -> Result<u32, BackendError> {
        let mut cells = [0u32; 2];
        cells[..arguments.len()].copy_from_slice(arguments);
        self.call(function, arguments.len() as u32, &mut cells)?;
        Ok(cells[0])
    }

    fn call_i64(
        &mut self,
        function: WasmFunctionInstance,
        arguments: &[u32],
    ) -> Result<u64, BackendError> {
        let mut cells = [0u32; 2];
        cells[..arguments.len()].copy_from_slice(arguments);
        self.call(function, arguments.len() as u32, &mut cells)?;
        Ok(u64::from(cells[0]) | (u64::from(cells[1]) << 32))
    }

    fn call(
        &mut self,
        function: WasmFunctionInstance,
        argc: u32,
        cells: &mut [u32; 2],
    ) -> Result<(), BackendError> {
        // SAFETY: The environment/function belong to this live instance and `cells` holds all ABI cells.
        if unsafe { wasm_runtime_call_wasm(self.environment, function, argc, cells.as_mut_ptr()) } {
            return Ok(());
        }
        // SAFETY: The instance remains live after a contained WAMR trap.
        let exception = unsafe { wasm_runtime_get_exception(self.instance) };
        let message = if exception.is_null() {
            "unknown WAMR trap".into()
        } else {
            // SAFETY: WAMR returns a NUL-terminated exception owned by the live instance.
            unsafe { CStr::from_ptr(exception) }
                .to_string_lossy()
                .into_owned()
        };
        Err(BackendError::fatal(
            Status::Panic,
            format!("guest trapped: {message}"),
        ))
    }

    fn write_memory(&self, pointer: u32, bytes: &[u8]) -> Result<(), BackendError> {
        self.validate_memory(pointer, bytes.len() as u32)?;
        if bytes.is_empty() {
            return Ok(());
        }
        // SAFETY: The range was validated and remains live for this synchronous copy.
        let destination = unsafe { wasm_runtime_addr_app_to_native(self.instance, pointer.into()) };
        if destination.is_null() {
            return Err(BackendError::fatal(
                Status::InvalidArgument,
                "WAMR could not translate guest input memory",
            ));
        }
        // SAFETY: Source and validated guest destination are nonoverlapping and equally sized.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), destination.cast(), bytes.len()) };
        Ok(())
    }

    fn read_memory(&self, pointer: u32, length: u32) -> Result<&[u8], BackendError> {
        self.validate_memory(pointer, length)?;
        // SAFETY: The range was validated and remains live until the next guest call.
        let native = unsafe { wasm_runtime_addr_app_to_native(self.instance, pointer.into()) };
        if native.is_null() {
            return Err(BackendError::fatal(
                Status::InvalidArgument,
                "WAMR could not translate guest output memory",
            ));
        }
        // SAFETY: WAMR validated the entire readable range.
        Ok(unsafe { std::slice::from_raw_parts(native.cast(), length as usize) })
    }

    fn validate_memory(&self, pointer: u32, length: u32) -> Result<(), BackendError> {
        // SAFETY: Validation only inspects the live module instance's linear-memory bounds.
        if unsafe { wasm_runtime_validate_app_addr(self.instance, pointer.into(), length.into()) } {
            Ok(())
        } else {
            Err(BackendError::fatal(
                Status::InvalidArgument,
                format!("guest memory range {pointer}..+{length} is invalid"),
            ))
        }
    }
}

impl Drop for WamrGuest {
    fn drop(&mut self) {
        // SAFETY: `WamrGuest` uniquely owns these handles and releases them in dependency order.
        unsafe {
            wasm_runtime_destroy_exec_env(self.environment);
            wasm_runtime_deinstantiate(self.instance);
            wasm_runtime_unload(self.module);
        }
    }
}

pub struct WamrBackend {
    guest: WamrGuest,
}

impl WamrBackend {
    pub fn preflight(module: &[u8]) -> Result<Self, BackendError> {
        Ok(Self {
            guest: WamrGuest::preflight(module)?,
        })
    }

    pub fn mount(
        mut self,
        session: SessionId,
        root: NodeId,
    ) -> Result<(Self, CommandBatch), BackendError> {
        let batch = self.mount_application(session, root)?;
        Ok((self, batch))
    }

    fn mount_application(
        &mut self,
        session: SessionId,
        root: NodeId,
    ) -> Result<CommandBatch, BackendError> {
        let request = encode_mount_request(&MountRequest {
            protocol_version: PROTOCOL_VERSION_V1,
            session,
            root,
        })
        .map_err(|error| BackendError::fatal(Status::InternalError, error.to_string()))?;
        let mount = self.guest.exports.mount;
        self.guest.request(mount, &request)
    }
}

impl BridgeBackendCandidate for WamrBackend {
    fn mount(&mut self, session: SessionId, root: NodeId) -> Result<CommandBatch, BackendError> {
        self.mount_application(session, root)
    }

    fn activate(self: Box<Self>) -> Box<dyn BridgeBackend> {
        self
    }
}

impl BridgeBackend for WamrBackend {
    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BackendError> {
        let request = encode_event_request(&EventRequest {
            protocol_version: PROTOCOL_VERSION_V1,
            event,
        })
        .map_err(|error| BackendError::fatal(Status::InternalError, error.to_string()))?;
        let dispatch = self.guest.exports.dispatch_event;
        self.guest.request(dispatch, &request)
    }

    fn destroy(mut self: Box<Self>, _: bool) -> Result<CommandBatch, BackendError> {
        self.guest.destroy_application()
    }

    fn discard_pending(&mut self) {}
}

/// Mounts a preflighted WAMR module through the shared native session registry.
///
/// # Safety
///
/// The renderer resolver and host must satisfy the native renderer ABI contract.
pub unsafe fn mount_module(
    get_api: Option<NativeRendererGetApiFn>,
    host: NativeHostHandle,
    module: &[u8],
) -> LynxElementBridgeNativeMountResult {
    // SAFETY: The renderer obligations are forwarded to `native_mount`.
    unsafe {
        native_mount(get_api, host, |session, root| {
            let backend = WamrBackend::preflight(module)?;
            let (backend, batch) = backend.mount(session, root)?;
            Ok((Box::new(backend), batch))
        })
    }
}

pub fn replace_module(session: LynxElementBridgeSession, module: &[u8]) -> NativeStatus {
    native_replace_backend(session, || Ok(Box::new(WamrBackend::preflight(module)?)))
}

#[unsafe(no_mangle)]
/// Mounts a WAMR application from a C-owned module span.
///
/// # Safety
///
/// `module_data` must be readable for `module_len` bytes during the call, and the renderer
/// resolver and host must satisfy the native renderer ABI contract.
pub unsafe extern "C" fn lynx_element_bridge_wamr_mount(
    get_api: Option<NativeRendererGetApiFn>,
    host: NativeHostHandle,
    module_data: *const u8,
    module_len: usize,
) -> LynxElementBridgeNativeMountResult {
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: The C caller supplies the borrowed module span for this call.
        let module = unsafe { native_span(module_data, module_len) }?;
        // SAFETY: The C caller also owns the renderer resolver and host contract.
        Ok::<_, BackendError>(unsafe { mount_module(get_api, host, module) })
    }));
    match result {
        Ok(Ok(result)) => result,
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

#[unsafe(no_mangle)]
/// Replaces a WAMR application from a C-owned module span.
///
/// # Safety
///
/// `module_data` must be readable for `module_len` bytes during the call.
pub unsafe extern "C" fn lynx_element_bridge_wamr_replace(
    session: LynxElementBridgeSession,
    module_data: *const u8,
    module_len: usize,
) -> NativeStatus {
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: The C caller supplies the borrowed module span for this call.
        Ok::<_, BackendError>(replace_module(session, unsafe {
            native_span(module_data, module_len)
        }?))
    }));
    match result {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => status_to_native(error.status),
        Err(_) => NATIVE_STATUS_PANIC,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_wamr_destroy(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    native_destroy_session(session)
}

#[unsafe(no_mangle)]
pub extern "C" fn lynx_element_bridge_wamr_abandon(
    session: LynxElementBridgeSession,
) -> LynxElementBridgeNativeDestroyResult {
    native_abandon_session(session)
}

unsafe fn native_span<'a>(data: *const u8, len: usize) -> Result<&'a [u8], BackendError> {
    if len == 0 || data.is_null() || len > isize::MAX as usize {
        return Err(BackendError::recoverable(
            Status::InvalidArgument,
            "WASM module span is invalid",
        ));
    }
    // SAFETY: The caller guarantees a readable span for the duration of the FFI call.
    Ok(unsafe { std::slice::from_raw_parts(data, len) })
}

fn initialize_runtime() -> Result<(), BackendError> {
    static INITIALIZED: OnceLock<Result<(), String>> = OnceLock::new();
    INITIALIZED
        .get_or_init(|| {
            // SAFETY: `OnceLock` makes this the process-wide initialization attempt.
            unsafe { wasm_runtime_init() }
                .then_some(())
                .ok_or_else(|| "WAMR runtime initialization failed".into())
        })
        .clone()
        .map_err(|message| BackendError::fatal(Status::HostError, message))?;
    THREAD_ENVIRONMENT.with(|environment| {
        if environment.available {
            Ok(())
        } else {
            Err(BackendError::fatal(
                Status::HostError,
                "WAMR thread environment initialization failed",
            ))
        }
    })
}

struct ThreadEnvironment {
    available: bool,
}

impl ThreadEnvironment {
    fn initialize() -> Self {
        // SAFETY: This WAMR API only initializes the calling thread.
        Self {
            available: unsafe { wasm_runtime_init_thread_env() },
        }
    }
}

impl Drop for ThreadEnvironment {
    fn drop(&mut self) {
        if self.available {
            // SAFETY: This thread-local guard owns the matching initialization on this thread.
            unsafe { wasm_runtime_destroy_thread_env() };
        }
    }
}

thread_local! {
    static THREAD_ENVIRONMENT: ThreadEnvironment = ThreadEnvironment::initialize();
}

fn lookup(
    instance: WasmModuleInstance,
    name: &CStr,
    expected_parameters: &[u8],
    expected_results: &[u8],
) -> Result<WasmFunctionInstance, BackendError> {
    // SAFETY: `instance` is live and `name` is NUL-terminated.
    let function = unsafe { wasm_runtime_lookup_function(instance, name.as_ptr()) };
    if function.is_null() {
        return Err(BackendError::recoverable(
            Status::Unsupported,
            format!("guest export {} is missing", name.to_string_lossy()),
        ));
    }
    validate_signature(
        instance,
        function,
        name,
        expected_parameters,
        expected_results,
    )
}

fn lookup_optional(
    instance: WasmModuleInstance,
    name: &CStr,
    expected_parameters: &[u8],
    expected_results: &[u8],
) -> Result<Option<WasmFunctionInstance>, BackendError> {
    // SAFETY: `instance` is live and `name` is NUL-terminated.
    let function = unsafe { wasm_runtime_lookup_function(instance, name.as_ptr()) };
    if function.is_null() {
        Ok(None)
    } else {
        validate_signature(
            instance,
            function,
            name,
            expected_parameters,
            expected_results,
        )
        .map(Some)
    }
}

fn validate_signature(
    instance: WasmModuleInstance,
    function: WasmFunctionInstance,
    name: &CStr,
    expected_parameters: &[u8],
    expected_results: &[u8],
) -> Result<WasmFunctionInstance, BackendError> {
    // SAFETY: The function belongs to this live instance.
    let parameter_count = unsafe { wasm_func_get_param_count(function, instance) } as usize;
    // SAFETY: The function belongs to this live instance.
    let result_count = unsafe { wasm_func_get_result_count(function, instance) } as usize;
    let mut parameters = vec![0; parameter_count];
    let mut results = vec![0; result_count];
    // SAFETY: Both vectors have exactly the lengths reported by WAMR.
    unsafe {
        wasm_func_get_param_types(function, instance, parameters.as_mut_ptr());
        wasm_func_get_result_types(function, instance, results.as_mut_ptr());
    }
    if parameters != expected_parameters || results != expected_results {
        return Err(BackendError::recoverable(
            Status::Unsupported,
            format!(
                "guest export {} has an incompatible signature",
                name.to_string_lossy()
            ),
        ));
    }
    Ok(function)
}

fn wamr_error(operation: &str, buffer: &[c_char; ERROR_BUFFER_SIZE]) -> BackendError {
    // SAFETY: WAMR writes a NUL-terminated diagnostic into the zero-initialized fixed buffer.
    let detail = unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_string_lossy();
    BackendError::recoverable(
        Status::Unsupported,
        format!("WAMR could not {operation}: {detail}"),
    )
}
