package com.yew.lynx;

import android.content.Context;
import com.lynx.jsbridge.LynxMethod;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class YewLynxModuleTest {
  private static final String SUCCESS =
      "{\"version\":1,\"ok\":true,\"operations\":[{\"op\":\"flush\",\"root\":1}]}";
  private static final String DESTROY_FAILURE =
      "{\"version\":1,\"ok\":false,\"status\":4,\"error\":\"wrong_thread\",\"operations\":[]}";
  private static final String ALREADY_MOUNTED =
      "{\"version\":1,\"ok\":false,\"status\":12,\"error\":\"already_mounted\",\"operations\":[]}";
  private static final String NOT_MOUNTED =
      "{\"version\":1,\"ok\":false,\"status\":3,\"error\":\"not_mounted\",\"operations\":[]}";
  private static final String MODULE_DESTROYED =
      "{\"version\":1,\"ok\":false,\"status\":3,\"error\":\"module_destroyed\",\"operations\":[]}";
  private static final String INVALID_ARGUMENT =
      "{\"version\":1,\"ok\":false,\"status\":1,\"error\":\"invalid_argument\",\"operations\":[]}";
  private static final String NATIVE_UNAVAILABLE =
      "{\"version\":1,\"ok\":false,\"status\":12,\"error\":\"native_bridge_unavailable\",\"operations\":[]}";
  private static final String NATIVE_FAILURE =
      "{\"version\":1,\"ok\":false,\"status\":12,\"error\":\"native_bridge_failure\",\"operations\":[]}";

  private static final class FakeNativeCalls implements YewLynxModule.NativeCalls {
    final List<String> calls = new ArrayList<>();
    boolean available = true;
    boolean consumeOnDestroy = true;
    boolean throwOnDestroy;
    String destroyResponse = SUCCESS;
    long nextSession = 100;

    @Override
    public boolean isAvailable() {
      return available;
    }

    @Override
    public byte[] mount(byte[] rootId, long[] sessionOut) {
      calls.add("mount:" + new String(rootId, StandardCharsets.UTF_8));
      sessionOut[0] = nextSession++;
      return SUCCESS.getBytes(StandardCharsets.UTF_8);
    }

    @Override
    public byte[] dispatch(long session, byte[] listenerId, byte[] eventName) {
      calls.add("dispatch:" + session + ":"
          + new String(listenerId, StandardCharsets.UTF_8) + ":"
          + new String(eventName, StandardCharsets.UTF_8));
      return SUCCESS.getBytes(StandardCharsets.UTF_8);
    }

    @Override
    public byte[] destroy(long session, boolean[] consumedOut) {
      calls.add("destroy:" + session);
      consumedOut[0] = consumeOnDestroy;
      if (throwOnDestroy) {
        throw new RuntimeException("mock destroy failure");
      }
      return destroyResponse.getBytes(StandardCharsets.UTF_8);
    }
  }

  public static void main(String[] args) throws Exception {
    methodSchemaMatchesTheStockMtsBridge();
    decimalStringsCrossJavaUnchanged();
    destroySessionPermitsRemountOnlyWhenConsumed();
    consumedIsHonoredWhenNativeThrows();
    inheritedDestroyPermanentlyDestroysTheModule();
    fallbacksUseTheExactProtocolEnvelope();
  }

  private static void methodSchemaMatchesTheStockMtsBridge() throws Exception {
    Method mount = YewLynxModule.class.getMethod("mount", String.class);
    Method dispatch = YewLynxModule.class.getMethod("dispatch", String.class, String.class);
    Method destroySession = YewLynxModule.class.getMethod("destroySession");
    Method destroy = YewLynxModule.class.getMethod("destroy");

    assertEquals(String.class, mount.getReturnType());
    assertEquals(String.class, dispatch.getReturnType());
    assertEquals(String.class, destroySession.getReturnType());
    assertEquals(void.class, destroy.getReturnType());
    assertNotNull(mount.getAnnotation(LynxMethod.class));
    assertNotNull(dispatch.getAnnotation(LynxMethod.class));
    assertNotNull(destroySession.getAnnotation(LynxMethod.class));
    assertEquals(null, destroy.getAnnotation(LynxMethod.class));
  }

  private static void decimalStringsCrossJavaUnchanged() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    YewLynxModule module = new YewLynxModule(new Context(), nativeCalls);

    assertEquals(SUCCESS, module.mount("9007199254740991"));
    assertEquals(SUCCESS, module.dispatch("9007199254740991", "tap"));
    assertEquals(Arrays.asList(
        "mount:9007199254740991",
        "dispatch:100:9007199254740991:tap"), nativeCalls.calls);
  }

  private static void destroySessionPermitsRemountOnlyWhenConsumed() {
    FakeNativeCalls consumed = new FakeNativeCalls();
    YewLynxModule remountable = new YewLynxModule(new Context(), consumed);
    assertEquals(SUCCESS, remountable.mount("1"));
    assertEquals(SUCCESS, remountable.destroySession());
    assertEquals(SUCCESS, remountable.mount("1"));

    FakeNativeCalls retained = new FakeNativeCalls();
    retained.consumeOnDestroy = false;
    retained.destroyResponse = DESTROY_FAILURE;
    YewLynxModule stillMounted = new YewLynxModule(new Context(), retained);
    assertEquals(SUCCESS, stillMounted.mount("1"));
    assertEquals(DESTROY_FAILURE, stillMounted.destroySession());
    assertEquals(ALREADY_MOUNTED, stillMounted.mount("1"));
  }

  private static void consumedIsHonoredWhenNativeThrows() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    nativeCalls.throwOnDestroy = true;
    nativeCalls.consumeOnDestroy = true;
    YewLynxModule module = new YewLynxModule(new Context(), nativeCalls);

    assertEquals(SUCCESS, module.mount("1"));
    assertEquals(NATIVE_FAILURE, module.destroySession());
    assertEquals(SUCCESS, module.mount("1"));
  }

  private static void inheritedDestroyPermanentlyDestroysTheModule() {
    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    YewLynxModule module = new YewLynxModule(new Context(), nativeCalls);
    assertEquals(SUCCESS, module.mount("1"));

    module.destroy();

    assertEquals(MODULE_DESTROYED, module.mount("1"));
    assertEquals(MODULE_DESTROYED, module.dispatch("1", "tap"));
    assertEquals(MODULE_DESTROYED, module.destroySession());
    assertEquals(1L, nativeCalls.calls.stream().filter(call -> call.startsWith("destroy:")).count());
    module.destroy();
    assertEquals(1L, nativeCalls.calls.stream().filter(call -> call.startsWith("destroy:")).count());
  }

  private static void fallbacksUseTheExactProtocolEnvelope() {
    FakeNativeCalls unavailable = new FakeNativeCalls();
    unavailable.available = false;
    YewLynxModule unavailableModule = new YewLynxModule(new Context(), unavailable);
    assertEquals(NATIVE_UNAVAILABLE, unavailableModule.mount("1"));

    FakeNativeCalls nativeCalls = new FakeNativeCalls();
    YewLynxModule module = new YewLynxModule(new Context(), nativeCalls);
    assertEquals(INVALID_ARGUMENT, module.mount(null));
    assertEquals(NOT_MOUNTED, module.dispatch("1", "tap"));
    assertEquals(NOT_MOUNTED, module.destroySession());
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
