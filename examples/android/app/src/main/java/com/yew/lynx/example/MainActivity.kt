package com.yew.lynx.example

import android.app.Activity
import android.graphics.Color
import android.os.Bundle
import android.util.Log
import android.view.ViewGroup
import com.lynx.elementbridge.LynxNativeRendererHost
import com.lynx.tasm.LynxView
import com.lynx.tasm.LynxViewBuilder
import com.lynx.tasm.ThreadStrategyForRendering

class MainActivity : Activity() {
    private var lynxView: LynxView? = null
    private var nativeRendererHost: LynxNativeRendererHost? = null
    private var nativeHostToken = 0L

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Log.i(TAG, "MainActivity onCreate")

        var view: LynxView? = null
        var rendererHost: LynxNativeRendererHost? = null
        var hostToken = 0L
        var hostRegistered = false
        var rustMounted = false
        try {
            val builder = LynxViewBuilder().apply {
                setEnableJSRuntime(false)
                setThreadStrategyForRendering(ThreadStrategyForRendering.ALL_ON_UI)
            }
            view = builder.build(this)
            view.setBackgroundColor(Color.rgb(245, 242, 234))
            hostToken = view.registerNativeRendererHost()
            hostRegistered = true
            rendererHost = LynxNativeRendererHost()
            setContentView(
                view,
                ViewGroup.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.MATCH_PARENT,
                ),
            )
            val backend = rendererHost.mount(hostToken)
            rustMounted = true
            lynxView = view
            nativeRendererHost = rendererHost
            nativeHostToken = hostToken
            Log.i(TAG, "Native renderer backend=$backend")
            Log.i(
                TAG,
                "Native renderer diagnostics mode=native bts_runtime=false mts_context=false template=false",
            )
        } catch (error: Throwable) {
            lynxView = null
            nativeRendererHost = null
            nativeHostToken = 0

            if (rustMounted) {
                try {
                    rendererHost?.destroy()
                } catch (cleanupError: Throwable) {
                    error.addSuppressed(cleanupError)
                    try {
                        rendererHost?.abandon()
                    } catch (abandonError: Throwable) {
                        cleanupError.addSuppressed(abandonError)
                    }
                }
            }
            if (hostRegistered) {
                try {
                    view?.unregisterNativeRendererHost(hostToken)
                } catch (cleanupError: Throwable) {
                    error.addSuppressed(cleanupError)
                }
            }
            try {
                view?.destroy()
            } catch (cleanupError: Throwable) {
                error.addSuppressed(cleanupError)
            }
            throw error
        }
    }

    override fun onDestroy() {
        val view = lynxView
        val rendererHost = nativeRendererHost
        val hostToken = nativeHostToken
        lynxView = null
        nativeRendererHost = null
        nativeHostToken = 0

        var failure: Throwable? = null
        fun recordFailure(error: Throwable) {
            val current = failure
            if (current == null) {
                failure = error
            } else if (current !== error) {
                current.addSuppressed(error)
            }
        }

        try {
            rendererHost?.destroy()
        } catch (error: Throwable) {
            recordFailure(error)
            try {
                rendererHost?.abandon()
            } catch (cleanupError: Throwable) {
                error.addSuppressed(cleanupError)
            }
        }
        try {
            if (view != null && hostToken != 0L) {
                view.unregisterNativeRendererHost(hostToken)
            }
        } catch (error: Throwable) {
            recordFailure(error)
        }
        try {
            view?.destroy()
        } catch (error: Throwable) {
            recordFailure(error)
        }
        try {
            super.onDestroy()
        } catch (error: Throwable) {
            recordFailure(error)
        }

        val lifecycleFailure = failure
        if (lifecycleFailure != null) {
            Log.e(TAG, "MainActivity onDestroy failed", lifecycleFailure)
            throw lifecycleFailure
        }
        Log.i(TAG, "MainActivity onDestroy complete")
    }

    private companion object {
        const val TAG = "LynxElementBridge"
    }
}
