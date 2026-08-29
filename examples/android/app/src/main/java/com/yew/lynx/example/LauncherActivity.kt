package com.yew.lynx.example

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.view.Gravity
import android.view.ViewGroup
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import com.lynx.elementbridge.nativebridge.NativeRendererHost

class LauncherActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (intent.getBooleanExtra(EXTRA_OPEN_NATIVE, false)) {
            startActivity(Intent(this, MainActivity::class.java))
            finish()
            return
        }
        val padding = (24 * resources.displayMetrics.density).toInt()
        setContentView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(padding, padding, padding, padding)
            setBackgroundColor(Color.rgb(245, 242, 234))
            addView(TextView(this@LauncherActivity).apply {
                text = "Lynx Element Bridge"
                textSize = 28f
                gravity = Gravity.CENTER
                setTextColor(Color.rgb(35, 42, 38))
            })
            addView(Button(this@LauncherActivity).apply {
                text = "Open Native (${NativeRendererHost.backendName()})"
                setOnClickListener {
                    startActivity(Intent(this@LauncherActivity, MainActivity::class.java))
                }
            }, buttonLayout(padding))
            addView(Button(this@LauncherActivity).apply {
                text = "Open WASM from URL"
                setOnClickListener {
                    startActivity(Intent(this@LauncherActivity, WasmUrlActivity::class.java))
                }
            }, buttonLayout(padding / 2))
        })
    }

    private fun buttonLayout(topMargin: Int) = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.WRAP_CONTENT,
    ).apply { this.topMargin = topMargin }

    companion object {
        const val EXTRA_OPEN_NATIVE = "com.yew.lynx.example.extra.OPEN_NATIVE"
    }
}
