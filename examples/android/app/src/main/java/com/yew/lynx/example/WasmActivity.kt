package com.yew.lynx.example

import android.app.Activity
import android.graphics.Color
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Gravity
import android.view.ViewGroup
import android.widget.TextView
import com.lynx.elementbridge.wamr.WamrRendererHost
import com.lynx.tasm.LynxView
import com.lynx.tasm.LynxViewBuilder
import com.lynx.tasm.ThreadStrategyForRendering
import java.io.IOException
import java.net.URL
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import okhttp3.HttpUrl
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import org.json.JSONObject

class WasmActivity : Activity() {
    private data class MountedSession(
        val view: LynxView,
        val rendererHost: WamrRendererHost,
        val nativeHostToken: Long,
        val cacheFileName: String,
    )

    private data class ReloadArtifact(
        val sha256: String,
        val size: Long,
    )

    private val executor = Executors.newSingleThreadExecutor()
    private val mainHandler = Handler(Looper.getMainLooper())
    private val reloadClient = OkHttpClient.Builder()
        .readTimeout(0, TimeUnit.MILLISECONDS)
        .pingInterval(15, TimeUnit.SECONDS)
        .followRedirects(false)
        .followSslRedirects(false)
        .build()
    private var session: MountedSession? = null
    @Volatile private var sourceUrl: URL? = null
    @Volatile private var currentSha256: String? = null
    @Volatile private var reloadSocket: WebSocket? = null
    @Volatile private var reconnectPending = false
    private var reloadInFlight = false
    private var pendingReload: ReloadArtifact? = null
    private var initialCacheFileName: String? = null
    private var pendingInitialModule: Pair<ByteArray, String>? = null
    private var reloadEpoch = 0L
    @Volatile private var activeDownloader: WasmModuleDownloader? = null
    @Volatile private var started = false
    @Volatile private var resumed = false
    @Volatile private var destroyed = false
    @Volatile private var initialModuleLoaded = false
    @Volatile private var stopped = false

    private val reconnect = Runnable {
        reconnectPending = false
        if (started && !destroyed && reloadSocket == null) connectReloadSocket()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val fileName = intent.getStringExtra(EXTRA_WASM_CACHE_FILE)
        val source = intent.getStringExtra(EXTRA_WASM_SOURCE_URL)
        if (fileName == null || source == null) {
            showError(IllegalArgumentException("A downloaded WASM module and source URL are required"))
            return
        }
        try {
            sourceUrl = WasmModuleDownloader.validateUrl(URL(source))
        } catch (error: Throwable) {
            Log.e(MainActivity.TAG, "Unable to open downloaded WASM module", error)
            showError(error)
            return
        }
        initialCacheFileName = fileName
        showLoading()
        executor.submit {
            val loaded = runCatching {
                WasmModuleFile.read(this, fileName).let { bytes ->
                    bytes to WasmModuleFile.sha256(bytes)
                }
            }
            runOnUiThread {
                loaded.fold(
                    onSuccess = { (bytes, sha256) ->
                        if (stopped) {
                            pendingInitialModule = bytes to sha256
                        } else {
                            mountInitialModule(fileName, bytes, sha256)
                        }
                    },
                    onFailure = { error ->
                        if (!destroyed) {
                            Log.e(MainActivity.TAG, "Unable to read downloaded WASM module", error)
                            showError(error)
                        }
                    },
                )
            }
        }
    }

    override fun onStart() {
        super.onStart()
        stopped = false
        started = true
        reloadEpoch += 1
        pendingInitialModule?.let { (bytes, sha256) ->
            pendingInitialModule = null
            initialCacheFileName?.let { fileName ->
                mountInitialModule(fileName, bytes, sha256)
            }
        }
        if (initialModuleLoaded) connectReloadSocket()
    }

    override fun onStop() {
        stopped = true
        started = false
        reloadEpoch += 1
        pendingReload = null
        reconnectPending = false
        mainHandler.removeCallbacks(reconnect)
        reloadSocket?.cancel()
        reloadSocket = null
        activeDownloader?.cancel()
        super.onStop()
    }

    override fun onDestroy() {
        destroyed = true
        started = false
        reloadSocket?.cancel()
        reloadSocket = null
        activeDownloader?.cancel()
        executor.shutdownNow()
        pendingInitialModule = null
        session?.let(::destroySession)
        session = null
        if (isFinishing) {
            intent.getStringExtra(EXTRA_WASM_CACHE_FILE)?.let { fileName ->
                runCatching { WasmModuleFile.resolve(this, fileName).delete() }
            }
        }
        reloadClient.dispatcher().executorService().shutdown()
        reloadClient.connectionPool().evictAll()
        super.onDestroy()
    }

    override fun onResume() {
        super.onResume()
        resumed = true
        session?.view?.onEnterForeground()
    }

    override fun onPause() {
        session?.view?.onEnterBackground()
        resumed = false
        super.onPause()
    }

    private fun mount(fileName: String, moduleBytes: ByteArray): MountedSession {
        val view = LynxViewBuilder().apply {
            setEnableJSRuntime(false)
            setThreadStrategyForRendering(ThreadStrategyForRendering.ALL_ON_UI)
        }.build(this)
        view.setBackgroundColor(Color.rgb(245, 242, 234))
        val hostToken = view.registerNativeRendererHost()
        var host: WamrRendererHost? = null
        try {
            val rendererHost = WamrRendererHost()
            host = rendererHost
            setContentView(view, ViewGroup.LayoutParams(-1, -1))
            val backend = rendererHost.mount(hostToken, moduleBytes)
            Log.i(MainActivity.TAG, "Native renderer backend=$backend")
            Log.i(
                MainActivity.TAG,
                "Native renderer diagnostics mode=wasm bts_runtime=false mts_context=false template=false",
            )
            return MountedSession(view, rendererHost, hostToken, fileName)
        } catch (error: Throwable) {
            runCatching { host?.destroy() }.onFailure {
                runCatching { host?.abandon() }
            }
            runCatching { view.unregisterNativeRendererHost(hostToken) }
            runCatching { view.destroy() }
            throw error
        }
    }

    private fun mountInitialModule(fileName: String, moduleBytes: ByteArray, sha256: String) {
        if (destroyed || isFinishing || initialModuleLoaded) return
        try {
            currentSha256 = sha256
            session = mount(fileName, moduleBytes)
            initialModuleLoaded = true
            if (resumed) session?.view?.onEnterForeground()
            if (started) connectReloadSocket()
        } catch (error: Throwable) {
            Log.e(MainActivity.TAG, "Unable to open downloaded WASM module", error)
            showError(error)
        }
    }

    private fun destroySession(mounted: MountedSession) {
        runCatching { mounted.rendererHost.destroy() }.onFailure { error ->
            runCatching { mounted.rendererHost.abandon() }.exceptionOrNull()?.let(error::addSuppressed)
            Log.e(MainActivity.TAG, "WASM session destroy failed", error)
        }
        runCatching { mounted.view.unregisterNativeRendererHost(mounted.nativeHostToken) }
        runCatching { mounted.view.destroy() }
    }

    private fun connectReloadSocket() {
        val source = sourceUrl ?: return
        if (!started || destroyed || reloadSocket != null) return
        val protocol = when (source.protocol) {
            "http" -> "ws"
            "https" -> "wss"
            else -> return
        }
        val websocketUrl = HttpUrl.parse(source.toString())
            ?.newBuilder()
            ?.scheme(protocol)
            ?.encodedPath(RELOAD_PATH)
            ?.query(null)
            ?.fragment(null)
            ?.build()
            ?: return
        reloadSocket = reloadClient.newWebSocket(
            Request.Builder().url(websocketUrl).build(),
            object : WebSocketListener() {
                override fun onMessage(webSocket: WebSocket, text: String) {
                    mainHandler.post {
                        if (started && reloadSocket === webSocket) {
                            parseReload(text)?.let(::requestReload)
                        }
                    }
                }

                override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                    mainHandler.post { socketEnded(webSocket) }
                }

                override fun onFailure(webSocket: WebSocket, error: Throwable, response: Response?) {
                    mainHandler.post {
                        if (started) Log.w(MainActivity.TAG, "WASM reload socket failed", error)
                        socketEnded(webSocket)
                    }
                }
            },
        )
    }

    private fun socketEnded(socket: WebSocket) {
        if (reloadSocket !== socket) return
        reloadSocket = null
        if (started && !destroyed && !reconnectPending) {
            reconnectPending = true
            mainHandler.postDelayed(reconnect, RECONNECT_DELAY_MS)
        }
    }

    private fun parseReload(message: String): ReloadArtifact? = runCatching {
        val source = sourceUrl ?: return null
        val root = JSONObject(message)
        if (root.getInt("v") != 1) return null
        val artifacts = root.getJSONArray("artifacts")
        repeat(artifacts.length()) { index ->
            val artifact = artifacts.getJSONObject(index)
            if (artifact.getString("path") != source.path) return@repeat
            val sha256 = artifact.getString("sha256")
            val size = artifact.getLong("size")
            if (!SHA256.matches(sha256) || size !in 1..WasmModuleFile.MAX_BYTES.toLong()) {
                return null
            }
            return ReloadArtifact(sha256, size)
        }
        null
    }.onFailure { error ->
        Log.w(MainActivity.TAG, "Ignoring invalid WASM reload message", error)
    }.getOrNull()

    @Synchronized
    private fun requestReload(artifact: ReloadArtifact) {
        if (!started || destroyed) return
        if (reloadInFlight) {
            pendingReload = artifact
            return
        }
        if (artifact.sha256 == currentSha256) return
        reloadInFlight = true
        val requestEpoch = reloadEpoch
        val expectedOrigin = sourceUrl ?: return completeReload()
        executor.submit {
            if (!started || requestEpoch != reloadEpoch) {
                runOnUiThread { completeReload() }
                return@submit
            }
            val downloader = WasmModuleDownloader(this)
            activeDownloader = downloader
            val result = runCatching {
                downloader.download(
                    expectedOrigin,
                    expectedSha256 = artifact.sha256,
                    requiredOrigin = expectedOrigin,
                ).let { module ->
                    if (module.file.length() != artifact.size) {
                        module.file.delete()
                        throw IOException("Downloaded WASM module does not match the announced size")
                    }
                    module to WasmModuleFile.read(this, module.file.name)
                }
            }
            if (activeDownloader === downloader) activeDownloader = null
            runOnUiThread {
                result.fold(
                    onSuccess = { (module, bytes) ->
                        if (!started || requestEpoch != reloadEpoch || destroyed || isFinishing) {
                            module.file.delete()
                        } else {
                            replaceSession(module, bytes)
                        }
                    },
                    onFailure = { error ->
                        if (started && !destroyed) {
                            Log.e(MainActivity.TAG, "WASM automatic reload failed", error)
                        }
                    },
                )
                completeReload()
            }
        }
    }

    private fun replaceSession(module: DownloadedWasmModule, moduleBytes: ByteArray) {
        val previous = session
        if (previous == null) {
            module.file.delete()
            return
        }
        if (resumed) previous.view.onEnterBackground()
        val replacement = try {
            mount(module.file.name, moduleBytes)
        } catch (error: Throwable) {
            setContentView(previous.view, ViewGroup.LayoutParams(-1, -1))
            if (resumed) previous.view.onEnterForeground()
            module.file.delete()
            Log.e(MainActivity.TAG, "Unable to mount reloaded WASM module", error)
            return
        }
        session = replacement
        sourceUrl = module.sourceUrl
        currentSha256 = module.sha256
        intent.putExtra(EXTRA_WASM_CACHE_FILE, module.file.name)
        intent.putExtra(EXTRA_WASM_SOURCE_URL, module.sourceUrl.toString())
        if (resumed) replacement.view.onEnterForeground()
        destroySession(previous)
        runCatching { WasmModuleFile.resolve(this, previous.cacheFileName).delete() }
        Log.i(MainActivity.TAG, "Reloaded WASM module sha256=${module.sha256}")
    }

    private fun completeReload() {
        val next = synchronized(this) {
            reloadInFlight = false
            pendingReload.also { pendingReload = null }
        }
        if (next != null && started && !destroyed) requestReload(next)
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

    private fun showLoading() {
        setContentView(TextView(this).apply {
            text = "Loading WASM page…"
            textSize = 17f
            gravity = Gravity.CENTER
            setBackgroundColor(Color.rgb(245, 242, 234))
            setTextColor(Color.rgb(35, 42, 38))
        })
    }

    companion object {
        const val EXTRA_WASM_CACHE_FILE = "com.yew.lynx.example.extra.WASM_CACHE_FILE"
        const val EXTRA_WASM_SOURCE_URL = "com.yew.lynx.example.extra.WASM_SOURCE_URL"
        private const val RELOAD_PATH = "/.well-known/yew-lynx/reload"
        private const val RECONNECT_DELAY_MS = 2_000L
        private val SHA256 = Regex("[0-9a-f]{64}")
    }
}
