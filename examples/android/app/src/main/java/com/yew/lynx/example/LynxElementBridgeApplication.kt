package com.yew.lynx.example

import android.app.Application
import android.util.Log
import com.lynx.tasm.LynxEnv

class LynxElementBridgeApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        LynxEnv.inst().init(this, null, null, null)
        check(LynxEnv.inst().isInitCompleted) {
            "LynxEnv initialization failed; verify the pinned arm64 native libraries"
        }
        Log.i("LynxElementBridge", "LynxEnv initialized")
    }
}
