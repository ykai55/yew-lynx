package com.lynx.elementbridge;

import android.content.Context;
import java.util.Arrays;

public final class JniIntegrationTest {
  private static final byte[] SUCCESS = {20, 0, 0, 0, 'L', 'E', 'B', '2', 0, (byte) 255};

  public static void main(String[] args) {
    assertEquals("mock", LynxElementBridgeModule.backendName());
    LynxElementBridgeModule module = new LynxElementBridgeModule(new Context());
    assertEquals(SUCCESS, module.mount(0xffff_ffffL));
    assertEquals(SUCCESS, module.dispatchEvent(SUCCESS));
    assertEquals(SUCCESS, module.completeBatch(SUCCESS));
    assertEquals(SUCCESS, module.destroySession());
    assertEquals(SUCCESS, module.mount(0xffff_ffffL));
    module.destroy();
  }

  private static void assertEquals(byte[] expected, byte[] actual) {
    if (!Arrays.equals(expected, actual)) {
      throw new AssertionError("byte arrays differ");
    }
  }

  private static void assertEquals(String expected, String actual) {
    if (!expected.equals(actual)) {
      throw new AssertionError("expected " + expected + " but got " + actual);
    }
  }
}
