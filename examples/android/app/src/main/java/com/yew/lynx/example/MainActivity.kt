package com.yew.lynx.example

import android.app.Activity
import android.graphics.Color
import android.os.Bundle
import android.util.Log
import android.view.ViewGroup
import com.lynx.elementbridge.nativebridge.NativeRendererHost
import com.lynx.tasm.LynxView
import com.lynx.tasm.LynxViewBuilder
import com.lynx.tasm.ThreadStrategyForRendering

class MainActivity : Activity() {
    private var lynxView: LynxView? = null
    private var rendererHost: NativeRendererHost? = null
    private var nativeHostToken = 0L

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Log.i(TAG, "MainActivity onCreate")
        var view: LynxView? = null
        var host: NativeRendererHost? = null
        var hostToken = 0L
        var hostRegistered = false
        var rustMounted = false
        try {
            view = LynxViewBuilder().apply {
                setEnableJSRuntime(false)
                setThreadStrategyForRendering(ThreadStrategyForRendering.ALL_ON_UI)
            }.build(this)
            view.setBackgroundColor(Color.rgb(245, 242, 234))
            hostToken = view.registerNativeRendererHost()
            hostRegistered = true
            host = NativeRendererHost()
            setContentView(
                view,
                ViewGroup.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.MATCH_PARENT,
                ),
            )
            val backend = host.mount(hostToken)
            rustMounted = true
            lynxView = view
            rendererHost = host
            nativeHostToken = hostToken
            Log.i(TAG, "Native renderer backend=$backend")
            Log.i(TAG, "Native renderer diagnostics mode=native bts_runtime=false mts_context=false template=false")
        } catch (error: Throwable) {
            if (rustMounted) {
                try {
                    host?.destroy()
                } catch (cleanupError: Throwable) {
                    error.addSuppressed(cleanupError)
                    runCatching { host?.abandon() }.exceptionOrNull()?.let(cleanupError::addSuppressed)
                }
            }
            if (hostRegistered) {
                runCatching { view?.unregisterNativeRendererHost(hostToken) }
                    .exceptionOrNull()?.let(error::addSuppressed)
            }
            runCatching { view?.destroy() }.exceptionOrNull()?.let(error::addSuppressed)
            throw error
        }
    }

    override fun onDestroy() {
        val view = lynxView
        val host = rendererHost
        val hostToken = nativeHostToken
        lynxView = null
        rendererHost = null
        nativeHostToken = 0
        var failure: Throwable? = null
        fun record(error: Throwable) {
            failure?.addSuppressed(error) ?: run { failure = error }
        }
        try {
            host?.destroy()
        } catch (error: Throwable) {
            record(error)
            runCatching { host?.abandon() }.exceptionOrNull()?.let(error::addSuppressed)
        }
        runCatching {
            if (view != null && hostToken != 0L) view.unregisterNativeRendererHost(hostToken)
        }.exceptionOrNull()?.let(::record)
        runCatching { view?.destroy() }.exceptionOrNull()?.let(::record)
        try {
            super.onDestroy()
        } catch (error: Throwable) {
            record(error)
        }
        failure?.let {
            Log.e(TAG, "MainActivity onDestroy failed", it)
            throw it
        }
        Log.i(TAG, "MainActivity onDestroy complete")
    }

    override fun onResume() {
        super.onResume()
        lynxView?.onEnterForeground()
    }

    override fun onPause() {
        lynxView?.onEnterBackground()
        super.onPause()
    }

    companion object {
        const val TAG = "LynxElementBridge"
    }
}
