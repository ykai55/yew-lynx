package com.lynx.elementbridge;

public final class DlsymFailureTest {
  public static void main(String[] args) {
    try {
      new LynxNativeRendererHost().mount(0x1_0000_0001L);
      throw new AssertionError("expected resolver failure");
    } catch (UnsupportedOperationException error) {
      if (!error.getMessage().contains("API export is unavailable")) {
        throw new AssertionError("unexpected resolver error", error);
      }
    }
  }
}
