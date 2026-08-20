package com.lynx.elementbridge;

import android.content.Context;
import com.lynx.jsbridge.LynxMethod;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class LynxElementBridgeModuleTest {
  private static final byte[] SUCCESS = {20, 0, 0, 0, 76, 69, 66, 50};

  private static final class FakeNativeCalls implements LynxElementBridgeModule.NativeCalls {
    final List<String> calls = new ArrayList<>();
    boolean available = true;
    boolean consumeOnDestroy = true;
    boolean throwOnDestroy;
    byte[] destroyResponse = SUCCESS;
    long nextSession = 100;

    @Override
    public boolean isAvailable() {
      return available;
    }

    @Override
    public byte[] mount(long rootId, long[] sessionOut) {
      calls.add("mount:" + rootId);
      sessionOut[0] = nextSession++;
      return SUCCESS;
    }

    @Override
    public byte[] dispatchEvent(long session, byte[] event) {
      calls.add("dispatchEvent:" + session + ":" + event.length);
      return SUCCESS;
    }

    @Override
    public byte[] completeBatch(long session, byte[] response) {
      calls.add("complete:" + session + ":" + response.length);
      return response;
    }

    @Override
    public byte[] destroy(long session, boolean[] consumedOut) {
      calls.add("destroy:" + session);
      consumedOut[0] = consumeOnDestroy;
      if (throwOnDestroy) {
        throw new RuntimeException("mock destroy failure");
      }
      return destroyResponse;
    }
  }

  public static void main(String[] args) throws Exception {
    methodSchemaUsesByteArraysAndNumericIds();
    numericIdsCrossJavaUnchanged();
    destroySessionPermitsRemountOnlyWhenConsumed();
    consumedIsHonoredWhenNativeThrows();
    inheritedDestroyPermanentlyDestroysTheModule();
    fallbacksAreFlatBuffersV2ResultEnvelopes();
  }

  private static void methodSchemaUsesByteArraysAndNumericIds() throws Exception {
    Method mount = LynxElementBridgeModule.class.getMethod("mount", long.class);
    Method dispatchEvent = LynxElementBridgeModule.class.getMethod("dispatchEvent", byte[].class);
    Method completeBatch = LynxElementBridgeModule.class.getMethod("completeBatch", byte[].class);
    Method destroySession = LynxElementBridgeModule.class.getMethod("destroySession");
    Method destroy = LynxElementBridgeModule.class.getMethod("destroy");

    assertEquals(byte[].class, mount.getReturnType());
    assertEquals(byte[].class, dispatchEvent.getReturnType());
    assertEquals(byte[].class, completeBatch.getReturnType());
    assertEquals(byte[].class, destroySession.getReturnType());
    assertEquals(void.class, destroy.getReturnType());
    assertNotNull(mount.getAnnotation(LynxMethod.class));
    assertNotNull(dispatchEvent.getAnnotation(LynxMethod.class));
    assertNotNull(completeBatch.getAnnotation(LynxMethod.class));
    assertNotNull(destroySession.getAnnotation(LynxMethod.class));
    assertEquals(null, destroy.getAnnotation(LynxMethod.class));
  }

  private static void numericIdsCrossJavaUnchanged() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    LynxElementBridgeModule module = new LynxElementBridgeModule(new Context(), nativeCalls);

    assertArrayEquals(SUCCESS, module.mount(0xffff_ffffL));
    assertArrayEquals(SUCCESS, module.dispatchEvent(SUCCESS));
    assertArrayEquals(SUCCESS, module.completeBatch(SUCCESS));
    assertEquals(Arrays.asList(
        "mount:4294967295",
        "dispatchEvent:100:8",
        "complete:100:8"), nativeCalls.calls);
  }

  private static void destroySessionPermitsRemountOnlyWhenConsumed() {
    FakeNativeCalls consumed = new FakeNativeCalls();
    LynxElementBridgeModule remountable = new LynxElementBridgeModule(new Context(), consumed);
    assertArrayEquals(SUCCESS, remountable.mount(1));
    assertArrayEquals(SUCCESS, remountable.destroySession());
    assertArrayEquals(SUCCESS, remountable.mount(1));

    FakeNativeCalls retained = new FakeNativeCalls();
    retained.consumeOnDestroy = false;
    retained.destroyResponse = failure(3, "wrong_thread");
    LynxElementBridgeModule stillMounted = new LynxElementBridgeModule(new Context(), retained);
    assertArrayEquals(SUCCESS, stillMounted.mount(1));
    assertArrayEquals(retained.destroyResponse, stillMounted.destroySession());
    assertFailure(stillMounted.mount(1), 1, "already_mounted");
  }

  private static void consumedIsHonoredWhenNativeThrows() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    nativeCalls.throwOnDestroy = true;
    nativeCalls.consumeOnDestroy = true;
    LynxElementBridgeModule module = new LynxElementBridgeModule(new Context(), nativeCalls);

    assertArrayEquals(SUCCESS, module.mount(1));
    assertFailure(module.destroySession(), 8, "native_bridge_failure");
    assertArrayEquals(SUCCESS, module.mount(1));
  }

  private static void inheritedDestroyPermanentlyDestroysTheModule() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    LynxElementBridgeModule module = new LynxElementBridgeModule(new Context(), nativeCalls);
    assertArrayEquals(SUCCESS, module.mount(1));
    module.destroy();

    assertFailure(module.mount(1), 2, "module_destroyed");
    assertFailure(module.dispatchEvent(SUCCESS), 2, "module_destroyed");
    assertFailure(module.destroySession(), 2, "module_destroyed");
    assertEquals(1L, nativeCalls.calls.stream().filter(call -> call.startsWith("destroy:")).count());
    module.destroy();
    assertEquals(1L, nativeCalls.calls.stream().filter(call -> call.startsWith("destroy:")).count());
  }

  private static void fallbacksAreFlatBuffersV2ResultEnvelopes() {
    FakeNativeCalls unavailable = new FakeNativeCalls();
    unavailable.available = false;
    assertFailure(new LynxElementBridgeModule(new Context(), unavailable).mount(1),
        10, "native_bridge_unavailable");

    LynxElementBridgeModule module =
        new LynxElementBridgeModule(new Context(), new FakeNativeCalls());
    assertFailure(module.mount(0), 1, "invalid_argument");
    assertFailure(module.mount(0x1_0000_0000L), 1, "invalid_argument");
    assertFailure(module.dispatchEvent(SUCCESS), 2, "not_mounted");
    assertFailure(module.completeBatch(SUCCESS), 2, "not_mounted");
    assertFailure(module.destroySession(), 2, "not_mounted");
  }

  private static void assertFailure(byte[] response, int status, String message) {
    assertEquals((byte) 'L', response[4]);
    assertEquals((byte) 'E', response[5]);
    assertEquals((byte) 'B', response[6]);
    assertEquals((byte) '2', response[7]);
    assertEquals(status, (response[54] & 0xff) | ((response[55] & 0xff) << 8));
    int length = (response[56] & 0xff)
        | ((response[57] & 0xff) << 8)
        | ((response[58] & 0xff) << 16)
        | ((response[59] & 0xff) << 24);
    assertEquals(message, new String(response, 60, length, StandardCharsets.UTF_8));
  }

  private static byte[] failure(int status, String message) {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    nativeCalls.available = false;
    byte[] response = new LynxElementBridgeModule(new Context(), nativeCalls).mount(1);
    response[54] = (byte) status;
    byte[] utf8 = message.getBytes(StandardCharsets.UTF_8);
    response = Arrays.copyOf(response, 60 + ((utf8.length + 4) & ~3));
    response[56] = (byte) utf8.length;
    response[57] = (byte) (utf8.length >>> 8);
    response[58] = (byte) (utf8.length >>> 16);
    response[59] = (byte) (utf8.length >>> 24);
    System.arraycopy(utf8, 0, response, 60, utf8.length);
    return response;
  }

  private static void assertArrayEquals(byte[] expected, byte[] actual) {
    if (!Arrays.equals(expected, actual)) {
      throw new AssertionError("byte arrays differ");
    }
  }

  private static void assertEquals(Object expected, Object actual) {
    if (expected == null ? actual != null : !expected.equals(actual)) {
      throw new AssertionError("expected " + expected + " but got " + actual);
    }
  }

  private static void assertNotNull(Object value) {
    if (value == null) {
      throw new AssertionError("expected a non-null value");
    }
  }
}
