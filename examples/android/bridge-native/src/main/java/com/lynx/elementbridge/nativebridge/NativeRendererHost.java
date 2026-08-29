package com.lynx.elementbridge.nativebridge;

/** Owns one native Rust application session mounted into a Lynx renderer host. */
public final class NativeRendererHost {
  private long nativeSession;
  private boolean destroyed;

  static {
    System.loadLibrary("lynx_element_bridge_native");
  }

  public synchronized String mount(long host) {
    if (destroyed || nativeSession != 0) {
      throw new IllegalStateException("Native renderer host cannot be mounted again");
    }
    if (host == 0) {
      throw new IllegalArgumentException("Lynx host token must not be zero");
    }
    String backend = nativeBackend();
    if (backend == null || backend.isEmpty()) {
      throw new IllegalStateException("Rust backend identity is invalid");
    }
    nativeSession = nativeMount(host);
    if (nativeSession == 0) {
      throw new IllegalStateException("Native mount returned a zero session");
    }
    return backend;
  }

  public synchronized void destroy() {
    requireMounted();
    boolean[] consumed = new boolean[1];
    try {
      nativeDestroySession(nativeSession, consumed);
      if (!consumed[0]) {
        throw new IllegalStateException("Native destroy did not consume the session");
      }
    } finally {
      if (consumed[0]) close();
    }
  }

  public synchronized void abandon() {
    requireMounted();
    boolean[] consumed = new boolean[1];
    try {
      nativeAbandonSession(nativeSession, consumed);
      if (!consumed[0]) {
        throw new IllegalStateException("Native abandon did not consume the session");
      }
    } finally {
      if (consumed[0]) close();
    }
  }

  public static String backendName() {
    return nativeBackend();
  }

  private void requireMounted() {
    if (destroyed || nativeSession == 0) {
      throw new IllegalStateException("Native renderer host is not mounted");
    }
  }

  private void close() {
    nativeSession = 0;
    destroyed = true;
  }

  private static native long nativeMount(long host);
  private static native void nativeDestroySession(long session, boolean[] consumedOut);
  private static native void nativeAbandonSession(long session, boolean[] consumedOut);
  private static native String nativeBackend();
}
