package com.yew.lynx.example

import android.content.Context
import java.io.BufferedInputStream
import java.io.FileOutputStream
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest

data class DownloadedWasmModule(
    val file: java.io.File,
    val sourceUrl: URL,
    val sha256: String,
)

class WasmModuleDownloader(private val context: Context) {
    @Volatile private var activeConnection: HttpURLConnection? = null
    @Volatile private var cancelled = false

    fun cancel() {
        cancelled = true
        activeConnection?.disconnect()
    }

    fun download(
        initialUrl: URL,
        expectedSha256: String? = null,
        requiredOrigin: URL? = null,
    ): DownloadedWasmModule {
        var currentUrl = validateUrl(initialUrl)
        repeat(MAX_REDIRECTS + 1) { redirectCount ->
            checkActive()
            if (requiredOrigin != null && !sameOrigin(requiredOrigin, currentUrl)) {
                throw IOException("WASM reload redirected to a different origin")
            }
            val connection = currentUrl.openConnection() as HttpURLConnection
            activeConnection = connection
            try {
                checkActive()
                connection.instanceFollowRedirects = false
                connection.connectTimeout = CONNECT_TIMEOUT_MS
                connection.readTimeout = READ_TIMEOUT_MS
                connection.useCaches = false
                connection.setRequestProperty("Accept", "application/wasm, application/octet-stream")
                connection.setRequestProperty("Cache-Control", "no-cache")
                when (val statusCode = connection.responseCode) {
                    in 200..299 -> return readModule(connection, currentUrl, expectedSha256)
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

    private fun readModule(
        connection: HttpURLConnection,
        sourceUrl: URL,
        expectedSha256: String?,
    ): DownloadedWasmModule {
        val contentLength = connection.contentLengthLong
        if (contentLength > WasmModuleFile.MAX_BYTES) {
            throw IOException("WASM module exceeds the 16 MiB limit")
        }
        val temporary = java.io.File.createTempFile("wasm-download-", ".tmp", context.cacheDir)
        try {
            val digest = MessageDigest.getInstance("SHA-256")
            var total = 0
            BufferedInputStream(connection.inputStream).use { input ->
                FileOutputStream(temporary).use { sink ->
                    val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
                    while (true) {
                        checkActive()
                        val count = input.read(buffer)
                        if (count == -1) break
                        total += count
                        if (total > WasmModuleFile.MAX_BYTES) {
                            throw IOException("WASM module exceeds the 16 MiB limit")
                        }
                        digest.update(buffer, 0, count)
                        sink.write(buffer, 0, count)
                    }
                }
            }
            if (total == 0) throw IOException("Downloaded WASM module is empty")
            val sha256 = digest.digest().joinToString("") {
                (it.toInt() and 0xff).toString(16).padStart(2, '0')
            }
            if (expectedSha256 != null && sha256 != expectedSha256) {
                throw IOException("Downloaded WASM module does not match the announced SHA-256")
            }
            val output = WasmModuleFile.create(context)
            if (!temporary.renameTo(output)) {
                try {
                    temporary.copyTo(output)
                    temporary.delete()
                } catch (error: Throwable) {
                    output.delete()
                    throw error
                }
            }
            return DownloadedWasmModule(output, sourceUrl, sha256)
        } catch (error: Throwable) {
            temporary.delete()
            throw error
        }
    }

    private fun checkActive() {
        if (cancelled || Thread.currentThread().isInterrupted) {
            throw IOException("WASM download cancelled")
        }
    }

    companion object {
        const val CONNECT_TIMEOUT_MS = 15_000
        const val READ_TIMEOUT_MS = 30_000
        const val MAX_REDIRECTS = 5

        fun validateUrl(url: URL): URL {
            require(url.protocol == "http" || url.protocol == "https") {
                "Only HTTP and HTTPS URLs are supported"
            }
            require(url.host.isNotBlank()) { "URL must include a host" }
            return url
        }

        private fun sameOrigin(left: URL, right: URL): Boolean =
            left.protocol == right.protocol &&
                left.host.equals(right.host, ignoreCase = true) &&
                left.effectivePort() == right.effectivePort()

        private fun URL.effectivePort(): Int = if (port == -1) defaultPort else port
    }
}
