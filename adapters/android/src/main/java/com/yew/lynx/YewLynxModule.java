package com.yew.lynx;

import android.content.Context;
import com.lynx.jsbridge.LynxMethod;
import com.lynx.jsbridge.LynxModule;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;

public final class YewLynxModule extends LynxModule {
  public static final String NAME = "YewLynx";

  private static final long MAX_PROTOCOL_ID = 0xffff_ffffL;
  private static final byte[] FAILURE_PREFIX = {
      20, 0, 0, 0, 76, 69, 66, 50, 12, 0, 12, 0, 0, 0, 11, 0,
      10, 0, 4, 0, 12, 0, 0, 0, 20, 0, 0, 0, 0, 0, 4, 2,
      12, 0, 12, 0, 0, 0, 0, 0, 10, 0, 4, 0, 12, 0, 0, 0,
      8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
  };

  interface NativeCalls {
    boolean isAvailable();

    byte[] mount(long rootId, long[] sessionOut);

    byte[] dispatchEvent(long session, byte[] event);

    byte[] complete(long session, byte[] response);

    byte[] destroy(long session, boolean[] consumedOut);
  }

  private static final NativeCalls JNI_NATIVE_CALLS = new NativeCalls() {
    private final boolean available = loadNativeLibrary();

    @Override
    public boolean isAvailable() {
      return available;
    }

    @Override
    public byte[] mount(long rootId, long[] sessionOut) {
      return nativeMount(rootId, sessionOut);
    }

    @Override
    public byte[] dispatchEvent(long session, byte[] event) {
      return nativeDispatchEvent(session, event);
    }

    @Override
    public byte[] complete(long session, byte[] response) {
      return nativeComplete(session, response);
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
  public synchronized byte[] mount(long rootId) {
    if (destroyed) {
      return failure(2, "module_destroyed");
    }
    if (nativeSession != 0) {
      return failure(1, "already_mounted");
    }
    if (!isProtocolId(rootId)) {
      return failure(1, "invalid_argument");
    }
    if (!nativeCalls.isAvailable()) {
      return failure(10, "native_bridge_unavailable");
    }

    long[] sessionOut = new long[1];
    try {
      byte[] batch = nativeCalls.mount(rootId, sessionOut);
      nativeSession = sessionOut[0];
      return nativeResponse(batch);
    } catch (RuntimeException | LinkageError error) {
      nativeSession = 0;
      return failure(8, "native_bridge_failure");
    }
  }

  @LynxMethod
  public synchronized byte[] dispatchEvent(byte[] event) {
    if (destroyed) {
      return failure(2, "module_destroyed");
    }
    if (nativeSession == 0) {
      return failure(2, "not_mounted");
    }
    if (event == null) {
      return failure(1, "invalid_argument");
    }

    try {
      return nativeResponse(nativeCalls.dispatchEvent(nativeSession, event));
    } catch (RuntimeException | LinkageError error) {
      return failure(8, "native_bridge_failure");
    }
  }

  @LynxMethod
  public synchronized byte[] completeBatch(byte[] response) {
    if (destroyed) {
      return failure(2, "module_destroyed");
    }
    if (nativeSession == 0) {
      return failure(2, "not_mounted");
    }
    if (response == null) {
      return failure(1, "invalid_argument");
    }
    try {
      return nativeResponse(nativeCalls.complete(nativeSession, response));
    } catch (RuntimeException | LinkageError error) {
      return failure(8, "native_bridge_failure");
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
  public synchronized byte[] destroySession() {
    if (destroyed) {
      return failure(2, "module_destroyed");
    }
    return destroySessionLocked();
  }

  private byte[] destroySessionLocked() {
    long session = nativeSession;
    if (session == 0) {
      return failure(2, "not_mounted");
    }
    if (!nativeCalls.isAvailable()) {
      return failure(10, "native_bridge_unavailable");
    }

    boolean[] consumedOut = new boolean[1];
    try {
      byte[] response = nativeCalls.destroy(session, consumedOut);
      if (consumedOut[0]) {
        nativeSession = 0;
      }
      return nativeResponse(response);
    } catch (RuntimeException | LinkageError error) {
      if (consumedOut[0]) {
        nativeSession = 0;
      }
      return failure(8, "native_bridge_failure");
    }
  }

  private static boolean isProtocolId(long value) {
    return value > 0 && value <= MAX_PROTOCOL_ID;
  }

  private static byte[] nativeResponse(byte[] response) {
    return response == null ? failure(8, "native_bridge_failure") : response;
  }

  private static byte[] failure(int status, String message) {
    byte[] utf8 = message.getBytes(StandardCharsets.UTF_8);
    int paddedLength = (utf8.length + 4) & ~3;
    byte[] response = Arrays.copyOf(FAILURE_PREFIX, FAILURE_PREFIX.length + paddedLength);
    response[54] = (byte) status;
    response[55] = (byte) (status >>> 8);
    response[56] = (byte) utf8.length;
    response[57] = (byte) (utf8.length >>> 8);
    response[58] = (byte) (utf8.length >>> 16);
    response[59] = (byte) (utf8.length >>> 24);
    System.arraycopy(utf8, 0, response, FAILURE_PREFIX.length, utf8.length);
    return response;
  }

  private static boolean loadNativeLibrary() {
    try {
      System.loadLibrary("yew_lynx_bridge");
      return true;
    } catch (LinkageError error) {
      return false;
    }
  }

  private static native byte[] nativeMount(long rootId, long[] sessionOut);

  private static native byte[] nativeDispatchEvent(long session, byte[] event);

  private static native byte[] nativeComplete(long session, byte[] response);

  private static native byte[] nativeDestroy(long session, boolean[] consumedOut);
}
