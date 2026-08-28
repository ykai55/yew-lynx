package com.yew.lynx.example

import android.app.Activity
import android.content.Intent
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
            val backend = if (BuildConfig.LYNX_ELEMENT_BRIDGE_WASM) {
                rendererHost.mount(
                    hostToken,
                    readWasmAsset(BuildConfig.LYNX_ELEMENT_BRIDGE_WASM_INITIAL_ASSET),
                )
            } else {
                rendererHost.mount(hostToken)
            }
            rustMounted = true
            lynxView = view
            nativeRendererHost = rendererHost
            nativeHostToken = hostToken
            Log.i(TAG, "Native renderer backend=$backend")
            Log.i(
                TAG,
                "Native renderer diagnostics mode=${if (BuildConfig.LYNX_ELEMENT_BRIDGE_WASM) "wasm" else "native"} bts_runtime=false mts_context=false template=false",
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

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        if (
            BuildConfig.LYNX_ELEMENT_BRIDGE_WASM &&
                intent.getBooleanExtra(EXTRA_REPLACE_WASM_MODULE, false)
        ) {
            replaceWasmModule(
                readWasmAsset(BuildConfig.LYNX_ELEMENT_BRIDGE_WASM_REPLACEMENT_ASSET),
            )
            Log.i(
                TAG,
                "WASM module replacement complete asset=${BuildConfig.LYNX_ELEMENT_BRIDGE_WASM_REPLACEMENT_ASSET}",
            )
        }
    }

    override fun onResume() {
        super.onResume()
        lynxView?.onEnterForeground()
    }

    override fun onPause() {
        lynxView?.onEnterBackground()
        super.onPause()
    }

    /** Test hook for replacing the active module without download or persistence behavior. */
    fun replaceWasmModule(moduleBytes: ByteArray) {
        check(BuildConfig.LYNX_ELEMENT_BRIDGE_WASM) { "App is not built in a WASM mode" }
        nativeRendererHost?.replace(moduleBytes)
            ?: error("WAMR session is not mounted")
    }

    private fun readWasmAsset(assetName: String): ByteArray =
        assets.open(assetName).use { it.readBytes() }

    private companion object {
        const val TAG = "LynxElementBridge"
        const val EXTRA_REPLACE_WASM_MODULE =
            "com.yew.lynx.example.extra.REPLACE_WASM_MODULE"
    }
}
