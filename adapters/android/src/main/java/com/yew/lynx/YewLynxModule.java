package com.yew.lynx;

import android.content.Context;
import com.lynx.jsbridge.LynxMethod;
import com.lynx.jsbridge.LynxModule;
import java.nio.charset.StandardCharsets;

public final class YewLynxModule extends LynxModule {
  public static final String NAME = "YewLynx";

  private static final String ERROR_NATIVE_UNAVAILABLE =
      "{\"version\":1,\"ok\":false,\"status\":12,\"error\":\"native_bridge_unavailable\",\"operations\":[]}";
  private static final String ERROR_ALREADY_MOUNTED =
      "{\"version\":1,\"ok\":false,\"status\":12,\"error\":\"already_mounted\",\"operations\":[]}";
  private static final String ERROR_NOT_MOUNTED =
      "{\"version\":1,\"ok\":false,\"status\":3,\"error\":\"not_mounted\",\"operations\":[]}";
  private static final String ERROR_MODULE_DESTROYED =
      "{\"version\":1,\"ok\":false,\"status\":3,\"error\":\"module_destroyed\",\"operations\":[]}";
  private static final String ERROR_INVALID_ARGUMENT =
      "{\"version\":1,\"ok\":false,\"status\":1,\"error\":\"invalid_argument\",\"operations\":[]}";
  private static final String ERROR_NATIVE_FAILURE =
      "{\"version\":1,\"ok\":false,\"status\":12,\"error\":\"native_bridge_failure\",\"operations\":[]}";

  interface NativeCalls {
    boolean isAvailable();

    byte[] mount(byte[] rootId, long[] sessionOut);

    byte[] dispatch(long session, byte[] listenerId, byte[] eventName);

    byte[] destroy(long session, boolean[] consumedOut);
  }

  private static final NativeCalls JNI_NATIVE_CALLS = new NativeCalls() {
    private final boolean available = loadNativeLibrary();

    @Override
    public boolean isAvailable() {
      return available;
    }

    @Override
    public byte[] mount(byte[] rootId, long[] sessionOut) {
      return nativeMount(rootId, sessionOut);
    }

    @Override
    public byte[] dispatch(long session, byte[] listenerId, byte[] eventName) {
      return nativeDispatch(session, listenerId, eventName);
    }

    @Override
    public byte[] destroy(long session, boolean[] consumedOut) {
      return nativeDestroy(session, consumedOut);
    }
  };

  private final NativeCalls nativeCalls;
  private long nativeSession;
  private boolean destroyed;

  public YewLynxModule(Context context) {
    this(context, JNI_NATIVE_CALLS);
  }

  YewLynxModule(Context context, NativeCalls nativeCalls) {
    super(context);
    if (nativeCalls == null) {
      throw new NullPointerException("nativeCalls");
    }
    this.nativeCalls = nativeCalls;
  }

  @LynxMethod
  public synchronized String mount(String rootId) {
    if (destroyed) {
      return ERROR_MODULE_DESTROYED;
    }
    if (nativeSession != 0) {
      return ERROR_ALREADY_MOUNTED;
    }
    if (rootId == null) {
      return ERROR_INVALID_ARGUMENT;
    }
    if (!nativeCalls.isAvailable()) {
      return ERROR_NATIVE_UNAVAILABLE;
    }

    long[] sessionOut = new long[1];
    try {
      byte[] batch = nativeCalls.mount(rootId.getBytes(StandardCharsets.UTF_8), sessionOut);
      nativeSession = sessionOut[0];
      return decode(batch);
    } catch (RuntimeException | LinkageError error) {
      nativeSession = 0;
      return ERROR_NATIVE_FAILURE;
    }
  }

  @LynxMethod
  public synchronized String dispatch(String listenerId, String eventName) {
    if (destroyed) {
      return ERROR_MODULE_DESTROYED;
    }
    if (nativeSession == 0) {
      return ERROR_NOT_MOUNTED;
    }
    if (listenerId == null || eventName == null) {
      return ERROR_INVALID_ARGUMENT;
    }

    try {
      return decode(nativeCalls.dispatch(nativeSession,
          listenerId.getBytes(StandardCharsets.UTF_8),
          eventName.getBytes(StandardCharsets.UTF_8)));
    } catch (RuntimeException | LinkageError error) {
      return ERROR_NATIVE_FAILURE;
    }
  }

  // This is a lifecycle hook, not an MTS-callable method.
  @Override
  public synchronized void destroy() {
    if (destroyed) {
      return;
    }
    destroyed = true;
    destroySessionLocked();
    super.destroy();
  }

  @LynxMethod
  public synchronized String destroySession() {
    if (destroyed) {
      return ERROR_MODULE_DESTROYED;
    }
    return destroySessionLocked();
  }

  private String destroySessionLocked() {
    long session = nativeSession;
    if (session == 0) {
      return ERROR_NOT_MOUNTED;
    }
    if (!nativeCalls.isAvailable()) {
      return ERROR_NATIVE_UNAVAILABLE;
    }

    boolean[] consumedOut = new boolean[1];
    try {
      byte[] response = nativeCalls.destroy(session, consumedOut);
      if (consumedOut[0]) {
        nativeSession = 0;
      }
      return decode(response);
    } catch (RuntimeException | LinkageError error) {
      if (consumedOut[0]) {
        nativeSession = 0;
      }
      return ERROR_NATIVE_FAILURE;
    }
  }

  private static String decode(byte[] utf8) {
    return utf8 == null ? ERROR_NATIVE_FAILURE : new String(utf8, StandardCharsets.UTF_8);
  }

  private static boolean loadNativeLibrary() {
    try {
      System.loadLibrary("yew_lynx_bridge");
      return true;
    } catch (LinkageError error) {
      return false;
    }
  }

  private static native byte[] nativeMount(byte[] rootId, long[] sessionOut);

  private static native byte[] nativeDispatch(
      long session, byte[] listenerId, byte[] eventName);

  private static native byte[] nativeDestroy(long session, boolean[] consumedOut);
}
