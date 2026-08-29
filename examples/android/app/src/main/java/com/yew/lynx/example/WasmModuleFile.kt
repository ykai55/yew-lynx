package com.yew.lynx.example

import android.content.Context
import java.io.File
import java.util.UUID

object WasmModuleFile {
    const val MAX_BYTES = 16 * 1024 * 1024

    private val fileNamePattern = Regex("wasm-[0-9a-f-]{36}\\.wasm")

    fun create(context: Context): File =
        File(context.cacheDir, "wasm-${UUID.randomUUID()}.wasm")

    fun resolve(context: Context, fileName: String): File {
        require(fileNamePattern.matches(fileName)) { "Invalid cached WASM module name" }
        return File(context.cacheDir, fileName).also { file ->
            require(file.canonicalFile.parentFile == context.cacheDir.canonicalFile) {
                "Cached WASM module is outside the application cache"
            }
        }
    }

    fun read(context: Context, fileName: String): ByteArray {
        val file = resolve(context, fileName)
        require(file.isFile) { "Downloaded WASM module is no longer available" }
        require(file.length() in 1..MAX_BYTES.toLong()) {
            "Downloaded WASM module must be between 1 byte and 16 MiB"
        }
        return file.readBytes()
    }
}
