package com.lynx.jsbridge;

import android.content.Context;

public abstract class LynxModule {
  protected final Context context;

  public LynxModule(Context context) {
    this.context = context;
  }

  public void destroy() {}
}
