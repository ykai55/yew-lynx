#include <jni.h>

#include <cstdint>
#include <limits>
#include <new>
#include <vector>

#include "yew_lynx.h"

namespace {

static_assert(sizeof(YewLynxSession) <= sizeof(jlong),
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

bool ReadBytes(JNIEnv* env, jbyteArray input, std::vector<uint8_t>* output) {
  if (input == nullptr) {
    Throw(env, "java/lang/NullPointerException", "UTF-8 input must not be null");
    return false;
  }

  const jsize length = env->GetArrayLength(input);
  if (env->ExceptionCheck()) {
    return false;
  }
  output->resize(static_cast<size_t>(length));
  if (length != 0) {
    env->GetByteArrayRegion(input, 0, length,
                            reinterpret_cast<jbyte*>(output->data()));
  }
  return !env->ExceptionCheck();
}

const uint8_t* BytesData(const std::vector<uint8_t>& bytes) {
  static constexpr uint8_t kEmptyInput = 0;
  return bytes.empty() ? &kEmptyInput : bytes.data();
}

jbyteArray CopyAndFreeBuffer(JNIEnv* env, YewLynxBuffer buffer) {
  if (buffer.len > static_cast<size_t>(std::numeric_limits<jsize>::max())) {
    yew_lynx_buffer_free(buffer);
    Throw(env, "java/lang/IllegalStateException", "Rust response exceeds JNI array limits");
    return nullptr;
  }
  if (buffer.len != 0 && buffer.data == nullptr) {
    Throw(env, "java/lang/IllegalStateException", "Rust returned an invalid buffer");
    return nullptr;
  }

  const jsize length = static_cast<jsize>(buffer.len);
  jbyteArray result = env->NewByteArray(length);
  if (result != nullptr && length != 0) {
    env->SetByteArrayRegion(result, 0, length,
                            reinterpret_cast<const jbyte*>(buffer.data));
  }
  yew_lynx_buffer_free(buffer);

  if (env->ExceptionCheck()) {
    if (result != nullptr) {
      env->DeleteLocalRef(result);
    }
    return nullptr;
  }
  return result;
}

void DestroyAfterMountFailure(YewLynxSession session) {
  if (session == 0) {
    return;
  }
  YewLynxDestroyResult cleanup = yew_lynx_destroy(session);
  yew_lynx_buffer_free(cleanup.response);
}

YewLynxSession SessionFromJLong(jlong session) {
  return static_cast<YewLynxSession>(session);
}

jlong SessionToJLong(YewLynxSession session) {
  return static_cast<jlong>(session);
}

}  // namespace

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_yew_lynx_YewLynxModule_nativeMount(JNIEnv* env, jclass,
                                             jbyteArray root_id,
                                             jlongArray session_out) {
  try {
    if (session_out == nullptr || env->GetArrayLength(session_out) < 1) {
      Throw(env, "java/lang/IllegalArgumentException",
            "sessionOut must contain one element");
      return nullptr;
    }
    if (env->ExceptionCheck()) {
      return nullptr;
    }

    std::vector<uint8_t> root_id_bytes;
    if (!ReadBytes(env, root_id, &root_id_bytes)) {
      return nullptr;
    }

    YewLynxMountResult mounted =
        yew_lynx_mount(BytesData(root_id_bytes), root_id_bytes.size());
    if (mounted.session > YEW_LYNX_JS_MAX_SAFE_INTEGER) {
      yew_lynx_buffer_free(mounted.response);
      DestroyAfterMountFailure(mounted.session);
      Throw(env, "java/lang/IllegalStateException",
            "Rust returned an unsafe session token");
      return nullptr;
    }
    jbyteArray batch = CopyAndFreeBuffer(env, mounted.response);
    if (batch == nullptr) {
      DestroyAfterMountFailure(mounted.session);
      return nullptr;
    }

    const jlong session = SessionToJLong(mounted.session);
    env->SetLongArrayRegion(session_out, 0, 1, &session);
    if (env->ExceptionCheck()) {
      env->DeleteLocalRef(batch);
      DestroyAfterMountFailure(mounted.session);
      return nullptr;
    }
    return batch;
  } catch (const std::bad_alloc&) {
    Throw(env, "java/lang/OutOfMemoryError", "Unable to allocate JNI input buffer");
  } catch (...) {
    Throw(env, "java/lang/RuntimeException", "Unexpected native bridge failure");
  }
  return nullptr;
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_yew_lynx_YewLynxModule_nativeDispatch(JNIEnv* env, jclass,
                                                jlong session,
                                                jbyteArray listener_id,
                                                jbyteArray event_name) {
  try {
    if (session == 0) {
      Throw(env, "java/lang/IllegalStateException", "Session is not mounted");
      return nullptr;
    }

    std::vector<uint8_t> listener_id_bytes;
    std::vector<uint8_t> event_name_bytes;
    if (!ReadBytes(env, listener_id, &listener_id_bytes) ||
        !ReadBytes(env, event_name, &event_name_bytes)) {
      return nullptr;
    }

    return CopyAndFreeBuffer(
        env, yew_lynx_dispatch(SessionFromJLong(session),
                               BytesData(listener_id_bytes), listener_id_bytes.size(),
                               BytesData(event_name_bytes), event_name_bytes.size()));
  } catch (const std::bad_alloc&) {
    Throw(env, "java/lang/OutOfMemoryError", "Unable to allocate JNI input buffer");
  } catch (...) {
    Throw(env, "java/lang/RuntimeException", "Unexpected native bridge failure");
  }
  return nullptr;
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_yew_lynx_YewLynxModule_nativeDestroy(JNIEnv* env, jclass,
                                               jlong session,
                                               jbooleanArray consumed_out) {
  try {
    if (session == 0) {
      Throw(env, "java/lang/IllegalStateException", "Session is not mounted");
      return nullptr;
    }
    if (consumed_out == nullptr || env->GetArrayLength(consumed_out) < 1) {
      Throw(env, "java/lang/IllegalArgumentException",
            "consumedOut must contain one element");
      return nullptr;
    }
    if (env->ExceptionCheck()) {
      return nullptr;
    }

    YewLynxDestroyResult destroyed =
        yew_lynx_destroy(SessionFromJLong(session));
    const jboolean consumed = destroyed.consumed != 0 ? JNI_TRUE : JNI_FALSE;
    env->SetBooleanArrayRegion(consumed_out, 0, 1, &consumed);
    if (env->ExceptionCheck()) {
      yew_lynx_buffer_free(destroyed.response);
      return nullptr;
    }
    return CopyAndFreeBuffer(env, destroyed.response);
  } catch (...) {
    Throw(env, "java/lang/RuntimeException", "Unexpected native bridge failure");
  }
  return nullptr;
}
