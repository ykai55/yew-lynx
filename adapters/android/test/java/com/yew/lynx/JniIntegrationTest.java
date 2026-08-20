package com.yew.lynx;

import android.content.Context;
import java.util.Arrays;

public final class JniIntegrationTest {
  private static final byte[] SUCCESS = {20, 0, 0, 0, 'L', 'E', 'B', '2', 0, (byte) 255};

  public static void main(String[] args) {
    YewLynxModule module = new YewLynxModule(new Context());
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
}
