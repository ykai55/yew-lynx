package com.yew.lynx;

import android.content.Context;

public final class JniIntegrationTest {
  private static final String SUCCESS =
      "{\"version\":1,\"ok\":true,\"operations\":[{\"op\":\"flush\",\"root\":9007199254740991}]}";

  public static void main(String[] args) {
    YewLynxModule module = new YewLynxModule(new Context());
    assertEquals(SUCCESS, module.mount("9007199254740991"));
    assertEquals(SUCCESS, module.dispatch("9007199254740991", "tap"));
    assertEquals(SUCCESS, module.destroySession());
    assertEquals(SUCCESS, module.mount("9007199254740991"));
    module.destroy();
  }

  private static void assertEquals(String expected, String actual) {
    if (!expected.equals(actual)) {
      throw new AssertionError("expected " + expected + " but got " + actual);
    }
  }
}
