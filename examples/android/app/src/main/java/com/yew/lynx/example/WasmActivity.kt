package com.yew.lynx.example

import android.app.Activity
import android.graphics.Color
import android.os.Bundle
import android.util.Log
import android.view.Gravity
import android.view.ViewGroup
import android.widget.TextView
import com.lynx.elementbridge.wamr.WamrRendererHost
import com.lynx.tasm.LynxView
import com.lynx.tasm.LynxViewBuilder
import com.lynx.tasm.ThreadStrategyForRendering

class WasmActivity : Activity() {
    private var lynxView: LynxView? = null
    private var rendererHost: WamrRendererHost? = null
    private var nativeHostToken = 0L
    private var cachedWasmFileName: String? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        cachedWasmFileName = intent.getStringExtra(EXTRA_WASM_CACHE_FILE)
        val fileName = cachedWasmFileName
        if (fileName == null) {
            showError(IllegalArgumentException("A downloaded WASM module is required"))
            return
        }
        var view: LynxView? = null
        var host: WamrRendererHost? = null
        var hostToken = 0L
        var hostRegistered = false
        try {
            view = LynxViewBuilder().apply {
                setEnableJSRuntime(false)
                setThreadStrategyForRendering(ThreadStrategyForRendering.ALL_ON_UI)
            }.build(this)
            view.setBackgroundColor(Color.rgb(245, 242, 234))
            hostToken = view.registerNativeRendererHost()
            hostRegistered = true
            host = WamrRendererHost()
            setContentView(view, ViewGroup.LayoutParams(-1, -1))
            val backend = host.mount(hostToken, WasmModuleFile.read(this, fileName))
            lynxView = view
            rendererHost = host
            nativeHostToken = hostToken
            Log.i(MainActivity.TAG, "Native renderer backend=$backend")
            Log.i(MainActivity.TAG, "Native renderer diagnostics mode=wasm bts_runtime=false mts_context=false template=false")
        } catch (error: Throwable) {
            try {
                host?.destroy()
            } catch (cleanupError: Throwable) {
                runCatching { host?.abandon() }.exceptionOrNull()?.let(cleanupError::addSuppressed)
                error.addSuppressed(cleanupError)
            }
            if (hostRegistered) runCatching { view?.unregisterNativeRendererHost(hostToken) }
            runCatching { view?.destroy() }
            Log.e(MainActivity.TAG, "Unable to open downloaded WASM module", error)
            showError(error)
        }
    }

    override fun onDestroy() {
        val view = lynxView
        val host = rendererHost
        val hostToken = nativeHostToken
        lynxView = null
        rendererHost = null
        nativeHostToken = 0
        runCatching { host?.destroy() }.onFailure { error ->
            runCatching { host?.abandon() }.exceptionOrNull()?.let(error::addSuppressed)
            Log.e(MainActivity.TAG, "WASM session destroy failed", error)
        }
        runCatching {
            if (view != null && hostToken != 0L) view.unregisterNativeRendererHost(hostToken)
        }
        runCatching { view?.destroy() }
        if (isFinishing) {
            cachedWasmFileName?.let { runCatching { WasmModuleFile.resolve(this, it).delete() } }
        }
        super.onDestroy()
    }

    override fun onResume() {
        super.onResume()
        lynxView?.onEnterForeground()
    }

    override fun onPause() {
        lynxView?.onEnterBackground()
        super.onPause()
    }

    private fun showError(error: Throwable) {
        setContentView(TextView(this).apply {
            text = "Unable to open WASM page\n\n${error.message ?: error.javaClass.simpleName}"
            textSize = 17f
            gravity = Gravity.CENTER
            setPadding(48, 48, 48, 48)
            setBackgroundColor(Color.rgb(245, 242, 234))
            setTextColor(Color.rgb(128, 48, 40))
        })
    }

    companion object {
        const val EXTRA_WASM_CACHE_FILE = "com.yew.lynx.example.extra.WASM_CACHE_FILE"
        const val EXTRA_WASM_SOURCE_URL = "com.yew.lynx.example.extra.WASM_SOURCE_URL"
    }
}
