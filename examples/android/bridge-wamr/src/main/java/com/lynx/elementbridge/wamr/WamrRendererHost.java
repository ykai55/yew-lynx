package com.lynx.elementbridge.wamr;

/** Owns one externally supplied WASM session mounted into a Lynx renderer host. */
public final class WamrRendererHost {
  private long nativeSession;
  private boolean destroyed;

  static {
    System.loadLibrary("lynx_element_bridge_wamr");
  }

  public synchronized String mount(long host, byte[] moduleBytes) {
    if (destroyed || nativeSession != 0) {
      throw new IllegalStateException("WAMR renderer host cannot be mounted again");
    }
    if (host == 0) {
      throw new IllegalArgumentException("Lynx host token must not be zero");
    }
    if (moduleBytes == null || moduleBytes.length == 0) {
      throw new IllegalArgumentException("WASM module bytes must not be empty");
    }
    String backend = nativeBackend();
    if (!"wasm".equals(backend)) {
      throw new IllegalStateException("WAMR backend identity is invalid");
    }
    nativeSession = nativeMountWasm(host, moduleBytes);
    if (nativeSession == 0) {
      throw new IllegalStateException("WAMR mount returned a zero session");
    }
    return backend;
  }

  public synchronized void destroy() {
    if (destroyed || nativeSession == 0) {
      throw new IllegalStateException("WAMR renderer host is not mounted");
    }
    boolean[] consumed = new boolean[1];
    try {
      nativeDestroySession(nativeSession, consumed);
      if (!consumed[0]) {
        throw new IllegalStateException("WAMR destroy did not consume the session");
      }
    } finally {
      if (consumed[0]) {
        nativeSession = 0;
        destroyed = true;
      }
    }
  }

  public synchronized void abandon() {
    if (destroyed || nativeSession == 0) {
      throw new IllegalStateException("WAMR renderer host is not mounted");
    }
    boolean[] consumed = new boolean[1];
    try {
      nativeAbandonSession(nativeSession, consumed);
      if (!consumed[0]) {
        throw new IllegalStateException("WAMR abandon did not consume the session");
      }
    } finally {
      if (consumed[0]) {
        nativeSession = 0;
        destroyed = true;
      }
    }
  }

  public static String backendName() {
    return nativeBackend();
  }

  private static native long nativeMountWasm(long host, byte[] moduleBytes);
  private static native void nativeDestroySession(long session, boolean[] consumedOut);
  private static native void nativeAbandonSession(long session, boolean[] consumedOut);
  private static native String nativeBackend();
}
