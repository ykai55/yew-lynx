package com.lynx.elementbridge;

/** Owns one Rust application session mounted into an opaque Lynx native renderer host. */
public final class LynxNativeRendererHost {
  interface NativeCalls {
    boolean isAvailable();

    long mount(long host);

    void destroy(long session, boolean[] consumedOut);

    void abandon(long session, boolean[] consumedOut);

    String backend();
  }

  private static final NativeCalls JNI_NATIVE_CALLS = new NativeCalls() {
    private final boolean available = loadNativeLibrary();

    @Override
    public boolean isAvailable() {
      return available;
    }

    @Override
    public long mount(long host) {
      return nativeMount(host);
    }

    @Override
    public void destroy(long session, boolean[] consumedOut) {
      nativeDestroySession(session, consumedOut);
    }

    @Override
    public void abandon(long session, boolean[] consumedOut) {
      nativeAbandonSession(session, consumedOut);
    }

    @Override
    public String backend() {
      return nativeBackend();
    }
  };

  private final NativeCalls nativeCalls;
  private long nativeSession;
  private boolean destroyed;

  public LynxNativeRendererHost() {
    this(JNI_NATIVE_CALLS);
  }

  LynxNativeRendererHost(NativeCalls nativeCalls) {
    if (nativeCalls == null) {
      throw new NullPointerException("nativeCalls");
    }
    this.nativeCalls = nativeCalls;
  }

  /** Mounts exactly once and returns the identity exported by the selected Rust backend. */
  public synchronized String mount(long host) {
    if (destroyed) {
      throw new IllegalStateException("Native renderer host is destroyed");
    }
    if (nativeSession != 0) {
      throw new IllegalStateException("Native renderer host is already mounted");
    }
    if (host == 0) {
      throw new IllegalArgumentException("Lynx host token must not be zero");
    }
    if (!nativeCalls.isAvailable()) {
      throw new IllegalStateException("Native renderer bridge is unavailable");
    }

    String backend = nativeCalls.backend();
    if (backend == null || backend.isEmpty()) {
      throw new IllegalStateException("Rust backend identity is invalid");
    }
    long session = nativeCalls.mount(host);
    if (session == 0) {
      throw new IllegalStateException("Native mount returned a zero session");
    }
    nativeSession = session;
    return backend;
  }

  /** Destroys the Rust session; a consumed failure still permanently closes this owner. */
  public synchronized void destroy() {
    if (destroyed) {
      throw new IllegalStateException("Native renderer host is already destroyed");
    }
    if (nativeSession == 0) {
      throw new IllegalStateException("Native renderer host is not mounted");
    }

    boolean[] consumedOut = new boolean[1];
    try {
      nativeCalls.destroy(nativeSession, consumedOut);
      if (!consumedOut[0]) {
        throw new IllegalStateException("Native destroy did not consume the session");
      }
    } finally {
      if (consumedOut[0]) {
        nativeSession = 0;
        destroyed = true;
      }
    }
  }

  /** Emergency cleanup after normal destroy failed without consuming the Rust session. */
  public synchronized void abandon() {
    if (destroyed) {
      throw new IllegalStateException("Native renderer host is already destroyed");
    }
    if (nativeSession == 0) {
      throw new IllegalStateException("Native renderer host is not mounted");
    }

    boolean[] consumedOut = new boolean[1];
    try {
      nativeCalls.abandon(nativeSession, consumedOut);
      if (!consumedOut[0]) {
        throw new IllegalStateException("Native abandon did not consume the session");
      }
    } finally {
      if (consumedOut[0]) {
        nativeSession = 0;
        destroyed = true;
      }
    }
  }

  public static String backendName() {
    return JNI_NATIVE_CALLS.isAvailable() ? JNI_NATIVE_CALLS.backend() : "unavailable";
  }

  private static boolean loadNativeLibrary() {
    try {
      System.loadLibrary("lynx_element_bridge");
      return true;
    } catch (LinkageError error) {
      return false;
    }
  }

  private static native long nativeMount(long host);

  private static native void nativeDestroySession(long session, boolean[] consumedOut);

  private static native void nativeAbandonSession(long session, boolean[] consumedOut);

  private static native String nativeBackend();
}
