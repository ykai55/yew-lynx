package com.yew.lynx.example

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ListView
import android.widget.ProgressBar
import android.widget.TextView
import java.io.BufferedInputStream
import java.io.FileOutputStream
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.Executors
import java.util.concurrent.Future
import org.json.JSONArray

class WasmUrlActivity : Activity() {
    private val executor = Executors.newSingleThreadExecutor()
    private val history = mutableListOf<String>()
    private lateinit var historyAdapter: ArrayAdapter<String>
    private lateinit var urlInput: EditText
    private lateinit var openButton: Button
    private lateinit var progress: ProgressBar
    private lateinit var status: TextView
    private var download: Future<*>? = null
    @Volatile private var activeConnection: HttpURLConnection? = null
    @Volatile private var stopped = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        title = "Open WASM page"
        setContentView(buildContentView())
        refreshHistory()
    }

    override fun onDestroy() {
        stopped = true
        activeConnection?.disconnect()
        download?.cancel(true)
        executor.shutdownNow()
        super.onDestroy()
    }

    private fun buildContentView(): View {
        val padding = dp(20)
        val content = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(padding, padding, padding, padding)
            setBackgroundColor(Color.rgb(245, 242, 234))
        }
        content.addView(TextView(this).apply {
            text = "Open a WASM page"
            textSize = 28f
            setTextColor(Color.rgb(35, 42, 38))
        })
        content.addView(TextView(this).apply {
            text = "Enter an HTTP or HTTPS URL for a Lynx Element Bridge wasm32-wasip1 module."
            textSize = 15f
            setTextColor(Color.rgb(75, 80, 76))
            setPadding(0, dp(8), 0, dp(16))
        })

        urlInput = EditText(this).apply {
            hint = "https://example.com/page.wasm"
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
            isSingleLine = true
        }
        content.addView(
            urlInput,
            LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val actions = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        openButton = Button(this).apply {
            text = "Download and open"
            setOnClickListener { confirmUrl() }
        }
        actions.addView(openButton, LinearLayout.LayoutParams(0, dp(52), 1f))
        actions.addView(Button(this).apply {
            text = "Clear history"
            setOnClickListener { clearHistory() }
        })
        content.addView(
            actions,
            LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ).apply { topMargin = dp(10) },
        )

        progress = ProgressBar(this).apply { visibility = View.GONE }
        status = TextView(this).apply {
            textSize = 14f
            setTextColor(Color.rgb(128, 48, 40))
            setPadding(dp(8), 0, 0, 0)
        }
        content.addView(LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            addView(progress, LinearLayout.LayoutParams(dp(32), dp(32)))
            addView(status, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f))
        })

        content.addView(TextView(this).apply {
            text = "History"
            textSize = 18f
            setTextColor(Color.rgb(35, 42, 38))
            setPadding(0, dp(18), 0, dp(6))
        })
        historyAdapter = ArrayAdapter(this, android.R.layout.simple_list_item_1, history)
        val emptyHistory = TextView(this).apply {
            text = "Confirmed URLs will appear here."
        }
        val historyList = ListView(this).apply {
            adapter = historyAdapter
            emptyView = emptyHistory
            setOnItemClickListener { _, _, position, _ ->
                urlInput.setText(history[position])
                urlInput.setSelection(urlInput.text.length)
            }
            setOnItemLongClickListener { _, _, position, _ ->
                confirmDeleteHistory(history[position])
                true
            }
        }
        content.addView(emptyHistory)
        content.addView(
            historyList,
            LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f),
        )
        return content
    }

    private fun confirmUrl() {
        val value = urlInput.text.toString().trim()
        if (value.isEmpty()) {
            status.text = "Enter a URL first."
            return
        }
        recordHistory(value)

        val initialUrl = try {
            validateUrl(URL(value))
        } catch (error: Exception) {
            status.text = error.message ?: "Invalid URL"
            return
        }
        setDownloading(true, "Downloading…")
        download = executor.submit {
            val result = runCatching { download(initialUrl) }
            runOnUiThread {
                if (isDestroyed || isFinishing) {
                    result.getOrNull()?.delete()
                    return@runOnUiThread
                }
                result.fold(
                    onSuccess = { file ->
                        try {
                            startActivity(
                                Intent(this, WasmActivity::class.java)
                                    .putExtra(WasmActivity.EXTRA_WASM_CACHE_FILE, file.name)
                                    .putExtra(WasmActivity.EXTRA_WASM_SOURCE_URL, value),
                            )
                            setDownloading(false, "")
                        } catch (error: Throwable) {
                            file.delete()
                            setDownloading(false, error.message ?: "Unable to open WASM page")
                        }
                    },
                    onFailure = { error ->
                        setDownloading(false, error.message ?: "Download failed")
                    },
                )
            }
        }
    }

    private fun download(initialUrl: URL): java.io.File {
        var currentUrl = initialUrl
        repeat(MAX_REDIRECTS + 1) { redirectCount ->
            if (stopped || Thread.currentThread().isInterrupted) {
                throw IOException("WASM download cancelled")
            }
            val connection = currentUrl.openConnection() as HttpURLConnection
            activeConnection = connection
            try {
                if (stopped) throw IOException("WASM download cancelled")
                connection.instanceFollowRedirects = false
                connection.connectTimeout = CONNECT_TIMEOUT_MS
                connection.readTimeout = READ_TIMEOUT_MS
                connection.setRequestProperty("Accept", "application/wasm, application/octet-stream")
                when (val statusCode = connection.responseCode) {
                    in 200..299 -> {
                        val contentLength = connection.contentLengthLong
                        if (contentLength > WasmModuleFile.MAX_BYTES) {
                            throw IOException("WASM module exceeds the 16 MiB limit")
                        }
                        val temporary = java.io.File.createTempFile(
                            "wasm-download-",
                            ".tmp",
                            cacheDir,
                        )
                        try {
                            BufferedInputStream(connection.inputStream).use { input ->
                                FileOutputStream(temporary).use { sink ->
                                    val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
                                    var total = 0
                                    while (true) {
                                        if (stopped || Thread.currentThread().isInterrupted) {
                                            throw IOException("WASM download cancelled")
                                        }
                                        val count = input.read(buffer)
                                        if (count == -1) break
                                        total += count
                                        if (total > WasmModuleFile.MAX_BYTES) {
                                            throw IOException("WASM module exceeds the 16 MiB limit")
                                        }
                                        sink.write(buffer, 0, count)
                                    }
                                    if (total == 0) throw IOException("Downloaded WASM module is empty")
                                }
                            }
                            val output = WasmModuleFile.create(this)
                            if (!temporary.renameTo(output)) {
                                try {
                                    temporary.copyTo(output)
                                    temporary.delete()
                                } catch (error: Throwable) {
                                    output.delete()
                                    throw error
                                }
                            }
                            return output
                        } catch (error: Throwable) {
                            temporary.delete()
                            throw error
                        }
                    }
                    HttpURLConnection.HTTP_MOVED_PERM,
                    HttpURLConnection.HTTP_MOVED_TEMP,
                    HttpURLConnection.HTTP_SEE_OTHER,
                    307,
                    308,
                    -> {
                        if (redirectCount == MAX_REDIRECTS) {
                            throw IOException("WASM download exceeded $MAX_REDIRECTS redirects")
                        }
                        val location = connection.getHeaderField("Location")
                            ?: throw IOException("WASM redirect is missing Location")
                        currentUrl = validateUrl(URL(currentUrl, location))
                    }
                    else -> throw IOException("WASM download failed with HTTP $statusCode")
                }
            } finally {
                connection.disconnect()
                if (activeConnection === connection) activeConnection = null
            }
        }
        throw IOException("WASM download exceeded $MAX_REDIRECTS redirects")
    }

    private fun validateUrl(url: URL): URL {
        require(url.protocol == "http" || url.protocol == "https") {
            "Only HTTP and HTTPS URLs are supported"
        }
        require(url.host.isNotBlank()) { "URL must include a host" }
        return url
    }

    private fun setDownloading(downloading: Boolean, message: String) {
        openButton.isEnabled = !downloading
        urlInput.isEnabled = !downloading
        progress.visibility = if (downloading) View.VISIBLE else View.GONE
        status.text = message
    }

    private fun refreshHistory() {
        history.clear()
        val encoded = getSharedPreferences(HISTORY_PREFERENCES, MODE_PRIVATE)
            .getString(HISTORY_KEY, null)
        if (encoded != null) {
            runCatching {
                val values = JSONArray(encoded)
                repeat(minOf(values.length(), HISTORY_LIMIT)) { index ->
                    values.optString(index).takeIf(String::isNotBlank)?.let(history::add)
                }
            }.onFailure {
                getSharedPreferences(HISTORY_PREFERENCES, MODE_PRIVATE).edit().clear().apply()
            }
        }
        historyAdapter.notifyDataSetChanged()
    }

    private fun recordHistory(url: String) {
        history.remove(url)
        history.add(0, url)
        while (history.size > HISTORY_LIMIT) history.removeAt(history.lastIndex)
        saveHistory()
    }

    private fun confirmDeleteHistory(url: String) {
        AlertDialog.Builder(this)
            .setTitle("Remove URL?")
            .setMessage(url)
            .setNegativeButton("Cancel", null)
            .setPositiveButton("Remove") { _, _ ->
                history.remove(url)
                saveHistory()
            }
            .show()
    }

    private fun clearHistory() {
        history.clear()
        saveHistory()
    }

    private fun saveHistory() {
        getSharedPreferences(HISTORY_PREFERENCES, MODE_PRIVATE)
            .edit()
            .putString(HISTORY_KEY, JSONArray(history).toString())
            .apply()
        historyAdapter.notifyDataSetChanged()
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    private companion object {
        const val CONNECT_TIMEOUT_MS = 15_000
        const val READ_TIMEOUT_MS = 30_000
        const val MAX_REDIRECTS = 5
        const val HISTORY_LIMIT = 20
        const val HISTORY_PREFERENCES = "wasm_url_history"
        const val HISTORY_KEY = "urls"
    }
}
