#include <jni.h>

#include <dlfcn.h>

#include <cstdio>
#include <cstdint>
#include <cstring>
#include <limits>

#include "lynx_native_application.h"

namespace {

static_assert(sizeof(LynxElementBridgeSession) <= sizeof(jlong),
              "A JNI long must be able to hold a session token");

void Throw(JNIEnv* env, const char* class_name, const char* message) {
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

bool SessionFromJLong(JNIEnv* env, jlong value,
                      LynxElementBridgeSession* output) {
  if (value <= 0 ||
      static_cast<uint64_t>(value) > std::numeric_limits<uint32_t>::max()) {
    Throw(env, "java/lang/IllegalArgumentException",
          "session ID must be a nonzero 32-bit integer");
    return false;
  }
  *output = static_cast<LynxElementBridgeSession>(value);
  return true;
}

void ThrowNativeStatus(JNIEnv* env, LynxNativeRendererStatus status,
                       const char* operation) {
  const char* class_name = "java/lang/IllegalStateException";
  const char* detail = "unknown status";
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
  std::snprintf(message, sizeof(message), "%s failed: %s (status %u)", operation,
                detail, status);
  Throw(env, class_name, message);
}

LynxNativeRendererGetApiFn ResolveNativeRendererApi(JNIEnv* env) {
  void* symbol = dlsym(RTLD_DEFAULT, "lynx_native_renderer_get_api");
#if defined(RTLD_NOLOAD)
  if (symbol == nullptr) {
    void* lynx = dlopen("liblynx.so", RTLD_NOW | RTLD_NOLOAD);
    if (lynx != nullptr) {
      symbol = dlsym(lynx, "lynx_native_renderer_get_api");
      dlclose(lynx);
    }
  }
#endif
  if (symbol == nullptr) {
    Throw(env, "java/lang/UnsupportedOperationException",
          "native mount failed: Lynx Native Renderer API export is unavailable");
    return nullptr;
  }

  LynxNativeRendererGetApiFn get_api = nullptr;
  static_assert(sizeof(get_api) == sizeof(symbol),
                "Function and data pointers must have the same size");
  std::memcpy(&get_api, &symbol, sizeof(get_api));
  return get_api;
}

jstring BackendName(JNIEnv* env) {
  const char* backend = lynx_element_bridge_backend();
  const char* marker = lynx_element_bridge_backend_marker();
  static constexpr char kMarkerPrefix[] = "lynx-element-bridge-backend:";
  if (backend == nullptr || marker == nullptr ||
      std::strncmp(marker, kMarkerPrefix, sizeof(kMarkerPrefix) - 1) != 0 ||
      std::strcmp(marker + sizeof(kMarkerPrefix) - 1, backend) != 0) {
    Throw(env, "java/lang/IllegalStateException",
          "Rust backend identity is invalid");
    return nullptr;
  }
  return env->NewStringUTF(backend);
}

}  // namespace

extern "C" JNIEXPORT jlong JNICALL
Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeMount(
    JNIEnv* env, jclass, jlong host) {
  if (host == 0) {
    Throw(env, "java/lang/IllegalArgumentException",
          "Lynx host token must not be zero");
    return 0;
  }
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
}

extern "C" JNIEXPORT void JNICALL
Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeDestroySession(
    JNIEnv* env, jclass, jlong session, jbooleanArray consumed_out) {
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
      lynx_element_bridge_native_destroy_session(native_session);
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
    JNIEnv* env, jclass, jlong session, jbooleanArray consumed_out) {
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
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_lynx_elementbridge_LynxNativeRendererHost_nativeBackend(JNIEnv* env,
                                                                 jclass) {
  return BackendName(env);
}
