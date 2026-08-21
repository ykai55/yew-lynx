package com.lynx.elementbridge;

import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class LynxNativeRendererHostTest {
  private static final class FakeNativeCalls implements LynxNativeRendererHost.NativeCalls {
    final List<String> calls = new ArrayList<>();
    boolean available = true;
    boolean consumeOnDestroy = true;
    boolean consumeOnAbandon = true;
    boolean throwOnDestroy;
    boolean throwOnAbandon;
    long nextSession = 91;

    @Override
    public boolean isAvailable() {
      return available;
    }

    @Override
    public long mount(long host) {
      calls.add("mount:" + Long.toUnsignedString(host));
      return nextSession;
    }

    @Override
    public void destroy(long session, boolean[] consumedOut) {
      calls.add("destroy:" + session);
      consumedOut[0] = consumeOnDestroy;
      if (throwOnDestroy) {
        throw new IllegalStateException("mock destroy failure");
      }
    }

    @Override
    public void abandon(long session, boolean[] consumedOut) {
      calls.add("abandon:" + session);
      consumedOut[0] = consumeOnAbandon;
      if (throwOnAbandon) {
        throw new IllegalStateException("mock abandon failure");
      }
    }

    @Override
    public String backend() {
      calls.add("backend");
      return "mock";
    }
  }

  public static void main(String[] args) throws Exception {
    lifecycleExposesOnlyNativeControlValues();
    mountReturnsBackendAndPreservesTokenBits();
    duplicateAndOutOfOrderCallsAreRejected();
    consumedDestroyFailureClosesTheOwner();
    unconsumedDestroyFailureCanBeRetried();
    unconsumedDestroyFailureCanBeAbandoned();
    consumedAbandonFailureClosesTheOwner();
    unconsumedAbandonFailureCanBeRetried();
    unavailableBridgeIsRejectedBeforeNativeCalls();
  }

  private static void lifecycleExposesOnlyNativeControlValues() throws Exception {
    Method mount = LynxNativeRendererHost.class.getMethod("mount", long.class);
    Method destroy = LynxNativeRendererHost.class.getMethod("destroy");
    Method abandon = LynxNativeRendererHost.class.getMethod("abandon");
    assertEquals(String.class, mount.getReturnType());
    assertEquals(void.class, destroy.getReturnType());
    assertEquals(void.class, abandon.getReturnType());
    for (Method method : LynxNativeRendererHost.class.getDeclaredMethods()) {
      assertEquals(false, method.getReturnType().isArray()
          && method.getReturnType().getComponentType() == byte.class);
      for (Class<?> parameter : method.getParameterTypes()) {
        assertEquals(false, parameter.isArray() && parameter.getComponentType() == byte.class);
      }
    }
  }

  private static void mountReturnsBackendAndPreservesTokenBits() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    LynxNativeRendererHost host = new LynxNativeRendererHost(nativeCalls);
    long token = Long.MIN_VALUE | 7;

    assertEquals("mock", host.mount(token));
    assertEquals(Arrays.asList("backend", "mount:9223372036854775815"), nativeCalls.calls);
    host.destroy();
    assertEquals("destroy:91", nativeCalls.calls.get(2));
  }

  private static void duplicateAndOutOfOrderCallsAreRejected() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    LynxNativeRendererHost host = new LynxNativeRendererHost(nativeCalls);
    assertThrows(IllegalStateException.class, host::destroy, "not mounted");
    assertThrows(IllegalArgumentException.class, () -> host.mount(0), "must not be zero");
    assertEquals("mock", host.mount(1));
    assertThrows(IllegalStateException.class, () -> host.mount(2), "already mounted");
    host.destroy();
    assertThrows(IllegalStateException.class, host::destroy, "already destroyed");
    assertThrows(IllegalStateException.class, host::abandon, "already destroyed");
    assertThrows(IllegalStateException.class, () -> host.mount(1), "destroyed");
  }

  private static void consumedDestroyFailureClosesTheOwner() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    nativeCalls.throwOnDestroy = true;
    LynxNativeRendererHost host = new LynxNativeRendererHost(nativeCalls);
    host.mount(1);

    assertThrows(IllegalStateException.class, host::destroy, "mock destroy failure");
    assertThrows(IllegalStateException.class, host::destroy, "already destroyed");
  }

  private static void unconsumedDestroyFailureCanBeRetried() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    nativeCalls.consumeOnDestroy = false;
    LynxNativeRendererHost host = new LynxNativeRendererHost(nativeCalls);
    host.mount(1);

    assertThrows(IllegalStateException.class, host::destroy, "did not consume");
    nativeCalls.consumeOnDestroy = true;
    nativeCalls.throwOnDestroy = false;
    host.destroy();
    assertEquals(2L, nativeCalls.calls.stream().filter(call -> call.startsWith("destroy:")).count());
  }

  private static void unconsumedDestroyFailureCanBeAbandoned() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    nativeCalls.consumeOnDestroy = false;
    nativeCalls.throwOnDestroy = true;
    LynxNativeRendererHost host = new LynxNativeRendererHost(nativeCalls);
    host.mount(1);

    assertThrows(IllegalStateException.class, host::destroy, "mock destroy failure");
    host.abandon();
    assertEquals(Arrays.asList("backend", "mount:1", "destroy:91", "abandon:91"),
        nativeCalls.calls);
    assertThrows(IllegalStateException.class, host::destroy, "already destroyed");
    assertThrows(IllegalStateException.class, host::abandon, "already destroyed");
  }

  private static void consumedAbandonFailureClosesTheOwner() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    nativeCalls.throwOnAbandon = true;
    LynxNativeRendererHost host = new LynxNativeRendererHost(nativeCalls);
    host.mount(1);

    assertThrows(IllegalStateException.class, host::abandon, "mock abandon failure");
    assertThrows(IllegalStateException.class, host::destroy, "already destroyed");
    assertThrows(IllegalStateException.class, host::abandon, "already destroyed");
  }

  private static void unconsumedAbandonFailureCanBeRetried() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    nativeCalls.consumeOnAbandon = false;
    LynxNativeRendererHost host = new LynxNativeRendererHost(nativeCalls);
    host.mount(1);

    assertThrows(IllegalStateException.class, host::abandon, "did not consume");
    nativeCalls.consumeOnAbandon = true;
    host.abandon();
    assertEquals(2L, nativeCalls.calls.stream().filter(call -> call.startsWith("abandon:")).count());
  }

  private static void unavailableBridgeIsRejectedBeforeNativeCalls() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    nativeCalls.available = false;
    LynxNativeRendererHost host = new LynxNativeRendererHost(nativeCalls);
    assertThrows(IllegalStateException.class, () -> host.mount(1), "unavailable");
    assertEquals(0, nativeCalls.calls.size());
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

  private static void assertEquals(Object expected, Object actual) {
    if (expected == null ? actual != null : !expected.equals(actual)) {
      throw new AssertionError("expected " + expected + " but got " + actual);
    }
  }
}
