package com.yew.lynx.example

import android.app.Activity
import android.graphics.Color
import android.os.Bundle
import android.util.Log
import android.view.ViewGroup
import com.lynx.tasm.LynxError
import com.lynx.tasm.LynxView
import com.lynx.tasm.LynxViewBuilder
import com.lynx.tasm.LynxViewClient
import com.lynx.tasm.ThreadStrategyForRendering
import com.lynx.elementbridge.LynxElementBridgeModule

class MainActivity : Activity() {
    private var lynxView: LynxView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Log.i(TAG, "MainActivity onCreate")
        Log.i(TAG, "LynxElementBridge backend=${LynxElementBridgeModule.backendName()}")

        val template = assets.open(TEMPLATE_ASSET).use { it.readBytes() }
        val builder = LynxViewBuilder().apply {
            setEnableJSRuntime(false)
            setEnableMTSModule(true)
            setThreadStrategyForRendering(ThreadStrategyForRendering.ALL_ON_UI)
            registerModule(
                LynxElementBridgeModule.NAME,
                LynxElementBridgeModule::class.java,
            )
        }
        val view = builder.build(this).apply {
            setBackgroundColor(Color.rgb(245, 242, 234))
            addLynxViewClient(object : LynxViewClient() {
                override fun onLoadSuccess() {
                    Log.i(TAG, "Lynx template loaded")
                }

                override fun onFirstScreen() {
                    Log.i(TAG, "Lynx first screen rendered")
                }

                override fun onReceivedError(error: LynxError) {
                    Log.e(
                        TAG,
                        "Lynx error ${error.subCode}: ${error.summaryMessage}",
                    )
                }
            })
        }
        lynxView = view
        setContentView(
            view,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        view.renderTemplateWithBaseUrl(template, emptyMap(), "asset:///$TEMPLATE_ASSET")
    }

    override fun onDestroy() {
        lynxView?.destroy()
        lynxView = null
        super.onDestroy()
        Log.i(TAG, "MainActivity onDestroy complete")
    }

    private companion object {
        const val TAG = "LynxElementBridge"
        const val TEMPLATE_ASSET = "lynx-element-bridge-counter.lynx.bundle"
    }
}
