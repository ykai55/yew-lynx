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

bool IdFromJLong(JNIEnv* env, jlong value, const char* name, uint32_t* output) {
  if (value <= 0 || static_cast<uint64_t>(value) > std::numeric_limits<uint32_t>::max()) {
    Throw(env, "java/lang/IllegalArgumentException", name);
    return false;
  }
  *output = static_cast<uint32_t>(value);
  return true;
}

jlong SessionToJLong(YewLynxSession session) {
  return static_cast<jlong>(session);
}

}  // namespace

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_yew_lynx_YewLynxModule_nativeMount(JNIEnv* env, jclass,
                                             jlong root_id,
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

    uint32_t native_root_id = 0;
    if (!IdFromJLong(env, root_id, "root ID must be a nonzero 32-bit integer",
                     &native_root_id)) {
      return nullptr;
    }

    YewLynxMountResult mounted = yew_lynx_mount(native_root_id);
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
Java_com_yew_lynx_YewLynxModule_nativeDispatchEvent(JNIEnv* env, jclass,
                                                     jlong session,
                                                     jbyteArray event) {
  try {
    if (session == 0) {
      Throw(env, "java/lang/IllegalStateException", "Session is not mounted");
      return nullptr;
    }

    uint32_t native_session = 0;
    std::vector<uint8_t> event_bytes;
    if (!IdFromJLong(env, session, "session ID must be a nonzero 32-bit integer",
                     &native_session) ||
        !ReadBytes(env, event, &event_bytes)) {
      return nullptr;
    }

    return CopyAndFreeBuffer(
        env, yew_lynx_dispatch(native_session, BytesData(event_bytes),
                               event_bytes.size()));
  } catch (const std::bad_alloc&) {
    Throw(env, "java/lang/OutOfMemoryError", "Unable to allocate JNI input buffer");
  } catch (...) {
    Throw(env, "java/lang/RuntimeException", "Unexpected native bridge failure");
  }
  return nullptr;
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_yew_lynx_YewLynxModule_nativeComplete(JNIEnv* env, jclass,
                                                jlong session,
                                                jbyteArray response) {
  try {
    uint32_t native_session = 0;
    std::vector<uint8_t> response_bytes;
    if (!IdFromJLong(env, session, "session ID must be a nonzero 32-bit integer",
                     &native_session) ||
        !ReadBytes(env, response, &response_bytes)) {
      return nullptr;
    }
    return CopyAndFreeBuffer(
        env, yew_lynx_complete(native_session, BytesData(response_bytes),
                               response_bytes.size()));
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

    uint32_t native_session = 0;
    if (!IdFromJLong(env, session, "session ID must be a nonzero 32-bit integer",
                     &native_session)) {
      return nullptr;
    }
    YewLynxDestroyResult destroyed = yew_lynx_destroy(native_session);
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
