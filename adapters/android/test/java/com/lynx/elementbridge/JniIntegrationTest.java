package com.lynx.elementbridge;

public final class JniIntegrationTest {
  public static void main(String[] args) {
    System.loadLibrary("lynx");
    nativeRendererLifecycleUsesResolvedFunctionTable();
    nativeRendererStatusesAreDeterministicExceptions();
  }

  private static void nativeRendererLifecycleUsesResolvedFunctionTable() {
    assertEquals("mock", LynxNativeRendererHost.backendName());
    LynxNativeRendererHost host = new LynxNativeRendererHost();
    assertEquals("mock", host.mount(0x1_0000_0001L));
    assertThrows(IllegalStateException.class, () -> host.mount(0x1_0000_0001L),
        "already mounted");
    host.destroy();
    assertThrows(IllegalStateException.class, host::destroy, "already destroyed");

    LynxNativeRendererHost abandoned = new LynxNativeRendererHost();
    assertEquals("mock", abandoned.mount(0x1_0000_0001L));
    abandoned.abandon();
    assertThrows(IllegalStateException.class, abandoned::abandon, "already destroyed");
    assertThrows(IllegalStateException.class, abandoned::destroy, "already destroyed");
  }

  private static void nativeRendererStatusesAreDeterministicExceptions() {
    assertNativeMountFailure(1, IllegalArgumentException.class, "invalid argument");
    assertNativeMountFailure(2, IllegalStateException.class, "invalid session");
    assertNativeMountFailure(3, IllegalStateException.class, "wrong thread");
    assertNativeMountFailure(4, UnsupportedOperationException.class, "unsupported");
    assertNativeMountFailure(5, IllegalStateException.class, "invalid ownership");
    assertNativeMountFailure(6, IllegalStateException.class, "invalid listener");
    assertNativeMountFailure(7, OutOfMemoryError.class, "resource exhausted");
    assertNativeMountFailure(8, IllegalStateException.class, "host error");
    assertNativeMountFailure(9, RuntimeException.class, "Rust panic");
    assertNativeMountFailure(10, IllegalStateException.class, "internal error");
    assertNativeMountFailure(11, IllegalStateException.class, "unknown status");
  }

  private static void assertNativeMountFailure(
      long host, Class<? extends Throwable> type, String messagePart) {
    assertThrows(type, () -> new LynxNativeRendererHost().mount(host), messagePart);
  }

  private static void assertThrows(
      Class<? extends Throwable> type, Runnable runnable, String messagePart) {
    try {
      runnable.run();
      throw new AssertionError("expected " + type.getName());
    } catch (Throwable error) {
      if (!type.isInstance(error) || error.getMessage() == null
          || !error.getMessage().contains(messagePart)) {
        throw new AssertionError("unexpected exception", error);
      }
    }
  }

  private static void assertEquals(String expected, String actual) {
    if (!expected.equals(actual)) {
      throw new AssertionError("expected " + expected + " but got " + actual);
    }
  }
}
