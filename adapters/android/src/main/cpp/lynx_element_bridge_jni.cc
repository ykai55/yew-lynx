#include <jni.h>

#include <dlfcn.h>

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <limits>
#include <new>
#include <vector>

#if defined(LYNX_ELEMENT_BRIDGE_WAMR)
#include "lynx_wamr_application.h"
#else
#include "lynx_native_application.h"
#endif

#if defined(LYNX_ELEMENT_BRIDGE_WAMR)
extern "C" JNIEXPORT const char lynx_element_bridge_wamr_backend_marker[] =
    LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_MARKER;
#endif

namespace {

static_assert(sizeof(LynxElementBridgeSession) <= sizeof(jlong),
              "A JNI long must be able to hold a session token");

void Throw(JNIEnv *env, const char *class_name, const char *message) {
  if (env->ExceptionCheck()) {
    return;
  }
  jclass exception_class = env->FindClass(class_name);
  if (exception_class == nullptr) {
    return;
  }
  env->ThrowNew(exception_class, message);
  env->DeleteLocalRef(exception_class);
}

bool SessionFromJLong(JNIEnv *env, jlong value,
                      LynxElementBridgeSession *output) {
  if (value <= 0 ||
      static_cast<uint64_t>(value) > std::numeric_limits<uint32_t>::max()) {
    Throw(env, "java/lang/IllegalArgumentException",
          "session ID must be a nonzero 32-bit integer");
    return false;
  }
  *output = static_cast<LynxElementBridgeSession>(value);
  return true;
}

void ThrowNativeStatus(JNIEnv *env, LynxNativeRendererStatus status,
                       const char *operation) {
  const char *class_name = "java/lang/IllegalStateException";
  const char *detail = "unknown status";
  switch (status) {
  case LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT:
    class_name = "java/lang/IllegalArgumentException";
    detail = "invalid argument";
    break;
  case LYNX_NATIVE_RENDERER_STATUS_INVALID_SESSION:
    detail = "invalid session";
    break;
  case LYNX_NATIVE_RENDERER_STATUS_WRONG_THREAD:
    detail = "wrong thread";
    break;
  case LYNX_NATIVE_RENDERER_STATUS_UNSUPPORTED:
    class_name = "java/lang/UnsupportedOperationException";
    detail = "unsupported";
    break;
  case LYNX_NATIVE_RENDERER_STATUS_INVALID_OWNERSHIP:
    detail = "invalid ownership";
    break;
  case LYNX_NATIVE_RENDERER_STATUS_INVALID_LISTENER:
    detail = "invalid listener";
    break;
  case LYNX_NATIVE_RENDERER_STATUS_RESOURCE_EXHAUSTED:
    class_name = "java/lang/OutOfMemoryError";
    detail = "resource exhausted";
    break;
  case LYNX_NATIVE_RENDERER_STATUS_HOST_ERROR:
    detail = "host error";
    break;
  case LYNX_NATIVE_RENDERER_STATUS_PANIC:
    class_name = "java/lang/RuntimeException";
    detail = "Rust panic";
    break;
  case LYNX_NATIVE_RENDERER_STATUS_INTERNAL_ERROR:
    detail = "internal error";
    break;
  default:
    break;
  }
  char message[128];
  std::snprintf(message, sizeof(message), "%s failed: %s (status %u)",
                operation, detail, status);
  Throw(env, class_name, message);
}

LynxNativeRendererGetApiFn ResolveNativeRendererApi(JNIEnv *env) {
  void *symbol = dlsym(RTLD_DEFAULT, "lynx_native_renderer_get_api");
#if defined(RTLD_NOLOAD)
  if (symbol == nullptr) {
    const char *libraries[] = {
        "liblynx_native_renderer.so",
        "liblynx.so",
    };
    for (const char *library : libraries) {
      void *lynx = dlopen(library, RTLD_NOW | RTLD_NOLOAD);
      if (lynx == nullptr) {
        continue;
      }
      symbol = dlsym(lynx, "lynx_native_renderer_get_api");
      dlclose(lynx);
      if (symbol != nullptr) {
        break;
      }
    }
  }
#endif
  if (symbol == nullptr) {
    Throw(
        env, "java/lang/UnsupportedOperationException",
        "native mount failed: Lynx Native Renderer API export is unavailable");
    return nullptr;
  }

  LynxNativeRendererGetApiFn get_api = nullptr;
  static_assert(sizeof(get_api) == sizeof(symbol),
                "Function and data pointers must have the same size");
  std::memcpy(&get_api, &symbol, sizeof(get_api));
  return get_api;
}

jstring BackendName(JNIEnv *env) {
#if defined(LYNX_ELEMENT_BRIDGE_WAMR)
  static constexpr char kMarkerPrefix[] = "lynx-element-bridge-backend:wasm-";
  if (std::strncmp(lynx_element_bridge_wamr_backend_marker, kMarkerPrefix,
                   sizeof(kMarkerPrefix) - 1) != 0 ||
      std::strcmp(lynx_element_bridge_wamr_backend_marker +
                      sizeof(kMarkerPrefix) - 1,
                  LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_NAME) != 0) {
    Throw(env, "java/lang/IllegalStateException",
          "WASM backend identity is invalid");
    return nullptr;
  }
  return env->NewStringUTF(LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_NAME);
#else
  const char *backend = lynx_element_bridge_backend();
  const char *marker = lynx_element_bridge_backend_marker();
  static constexpr char kMarkerPrefix[] = "lynx-element-bridge-backend:";
  if (backend == nullptr || marker == nullptr ||
      std::strncmp(marker, kMarkerPrefix, sizeof(kMarkerPrefix) - 1) != 0 ||
      std::strcmp(marker + sizeof(kMarkerPrefix) - 1, backend) != 0) {
    Throw(env, "java/lang/IllegalStateException",
          "Rust backend identity is invalid");
    return nullptr;
  }
  return env->NewStringUTF(backend);
#endif
}

#if defined(LYNX_ELEMENT_BRIDGE_WAMR)
bool CopyModule(JNIEnv *env, jbyteArray module_bytes,
                std::vector<uint8_t> *module) {
  if (module_bytes == nullptr) {
    Throw(env, "java/lang/NullPointerException", "moduleBytes");
    return false;
  }
  const jsize length = env->GetArrayLength(module_bytes);
  if (env->ExceptionCheck()) {
    return false;
  }
  if (length == 0) {
    Throw(env, "java/lang/IllegalArgumentException",
          "WASM module bytes must not be empty");
    return false;
  }
  try {
    module->resize(static_cast<size_t>(length));
  } catch (const std::bad_alloc &) {
    Throw(env, "java/lang/OutOfMemoryError",
          "could not copy WASM module bytes");
    return false;
  }
  env->GetByteArrayRegion(module_bytes, 0, length,
                          reinterpret_cast<jbyte *>(module->data()));
  return !env->ExceptionCheck();
}
#endif

} // namespace

extern "C" JNIEXPORT jlong JNICALL
Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeMount(JNIEnv *env,
                                                               jclass,
                                                               jlong host) {
  if (host == 0) {
    Throw(env, "java/lang/IllegalArgumentException",
          "Lynx host token must not be zero");
    return 0;
  }
#if defined(LYNX_ELEMENT_BRIDGE_WAMR)
  Throw(env, "java/lang/UnsupportedOperationException",
        "native mount is unavailable in WASM mode");
  return 0;
#else
  LynxNativeRendererGetApiFn get_api = ResolveNativeRendererApi(env);
  if (get_api == nullptr) {
    return 0;
  }

  LynxElementBridgeNativeMountResult mounted = lynx_element_bridge_native_mount(
      get_api, static_cast<LynxNativeHostHandle>(host));
  if (mounted.status != LYNX_NATIVE_RENDERER_STATUS_OK) {
    ThrowNativeStatus(env, mounted.status, "native mount");
    return 0;
  }
  if (mounted.session == 0) {
    ThrowNativeStatus(env, LYNX_NATIVE_RENDERER_STATUS_HOST_ERROR,
                      "native mount");
    return 0;
  }
  return static_cast<jlong>(mounted.session);
#endif
}

extern "C" JNIEXPORT jlong JNICALL
Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeMountWasm(
    JNIEnv *env, jclass, jlong host, jbyteArray module_bytes) {
#if defined(LYNX_ELEMENT_BRIDGE_WAMR)
  if (host == 0) {
    Throw(env, "java/lang/IllegalArgumentException",
          "Lynx host token must not be zero");
    return 0;
  }
  std::vector<uint8_t> module;
  if (!CopyModule(env, module_bytes, &module)) {
    return 0;
  }
  LynxNativeRendererGetApiFn get_api = ResolveNativeRendererApi(env);
  if (get_api == nullptr) {
    return 0;
  }
  LynxElementBridgeNativeMountResult mounted = lynx_element_bridge_wamr_mount(
      get_api, static_cast<LynxNativeHostHandle>(host), module.data(),
      module.size());
  if (mounted.status != LYNX_NATIVE_RENDERER_STATUS_OK) {
    ThrowNativeStatus(env, mounted.status, "WAMR mount");
    return 0;
  }
  if (mounted.session == 0) {
    ThrowNativeStatus(env, LYNX_NATIVE_RENDERER_STATUS_HOST_ERROR,
                      "WAMR mount");
    return 0;
  }
  return static_cast<jlong>(mounted.session);
#else
  (void)host;
  (void)module_bytes;
  Throw(env, "java/lang/UnsupportedOperationException",
        "WAMR mount is unavailable in a native backend");
  return 0;
#endif
}

extern "C" JNIEXPORT void JNICALL
Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeReplaceWasm(
    JNIEnv *env, jclass, jlong session, jbyteArray module_bytes) {
#if defined(LYNX_ELEMENT_BRIDGE_WAMR)
  LynxElementBridgeSession native_session = 0;
  if (!SessionFromJLong(env, session, &native_session)) {
    return;
  }
  std::vector<uint8_t> module;
  if (!CopyModule(env, module_bytes, &module)) {
    return;
  }
  LynxNativeRendererStatus status = lynx_element_bridge_wamr_replace(
      native_session, module.data(), module.size());
  if (status != LYNX_NATIVE_RENDERER_STATUS_OK) {
    ThrowNativeStatus(env, status, "WAMR replace");
  }
#else
  (void)session;
  (void)module_bytes;
  Throw(env, "java/lang/UnsupportedOperationException",
        "WAMR replace is unavailable in a native backend");
#endif
}

extern "C" JNIEXPORT void JNICALL
Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeDestroySession(
    JNIEnv *env, jclass, jlong session, jbooleanArray consumed_out) {
  if (consumed_out == nullptr || env->GetArrayLength(consumed_out) < 1) {
    Throw(env, "java/lang/IllegalArgumentException",
          "consumedOut must contain one element");
    return;
  }
  if (env->ExceptionCheck()) {
    return;
  }

  LynxElementBridgeSession native_session = 0;
  if (!SessionFromJLong(env, session, &native_session)) {
    return;
  }
  LynxElementBridgeNativeDestroyResult destroyed =
#if defined(LYNX_ELEMENT_BRIDGE_WAMR)
      lynx_element_bridge_wamr_destroy(native_session);
#else
      lynx_element_bridge_native_destroy_session(native_session);
#endif
  const jboolean consumed = destroyed.consumed != 0 ? JNI_TRUE : JNI_FALSE;
  env->SetBooleanArrayRegion(consumed_out, 0, 1, &consumed);
  if (env->ExceptionCheck()) {
    return;
  }
  if (destroyed.status != LYNX_NATIVE_RENDERER_STATUS_OK) {
    ThrowNativeStatus(env, destroyed.status, "native destroy");
  } else if (destroyed.consumed == 0) {
    ThrowNativeStatus(env, LYNX_NATIVE_RENDERER_STATUS_INTERNAL_ERROR,
                      "native destroy");
  }
}

extern "C" JNIEXPORT void JNICALL
Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeAbandonSession(
    JNIEnv *env, jclass, jlong session, jbooleanArray consumed_out) {
#if defined(LYNX_ELEMENT_BRIDGE_WAMR)
  (void)session;
  (void)consumed_out;
  Throw(env, "java/lang/UnsupportedOperationException",
        "abandon is unavailable in wasm-dioxus mode");
  return;
#else
  if (consumed_out == nullptr || env->GetArrayLength(consumed_out) < 1) {
    Throw(env, "java/lang/IllegalArgumentException",
          "consumedOut must contain one element");
    return;
  }
  if (env->ExceptionCheck()) {
    return;
  }

  LynxElementBridgeSession native_session = 0;
  if (!SessionFromJLong(env, session, &native_session)) {
    return;
  }
  LynxElementBridgeNativeDestroyResult abandoned =
      lynx_element_bridge_native_abandon_session(native_session);
  const jboolean consumed = abandoned.consumed != 0 ? JNI_TRUE : JNI_FALSE;
  env->SetBooleanArrayRegion(consumed_out, 0, 1, &consumed);
  if (env->ExceptionCheck()) {
    return;
  }
  if (abandoned.status != LYNX_NATIVE_RENDERER_STATUS_OK) {
    ThrowNativeStatus(env, abandoned.status, "native abandon");
  } else if (abandoned.consumed == 0) {
    ThrowNativeStatus(env, LYNX_NATIVE_RENDERER_STATUS_INTERNAL_ERROR,
                      "native abandon");
  }
#endif
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeBackend(JNIEnv *env,
                                                                 jclass) {
  return BackendName(env);
}
