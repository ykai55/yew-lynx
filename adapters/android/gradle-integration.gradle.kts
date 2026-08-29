// Copy these blocks into the Android app/library module that already depends
// on stock OSS Lynx. This file is documentation, not a standalone Gradle module.
val elementBridgeAndroidDir = rootProject.file("adapters/android")
val elementBridgeBackend = providers.gradleProperty("lynxElementBridgeBackend")
    .orElse("yew").get()
require(elementBridgeBackend in setOf("yew", "dioxus", "wasm-dioxus", "wasm-yew"))
val elementBridgeWasmMode = elementBridgeBackend.startsWith("wasm-")
val elementBridgeRustPackage = when (elementBridgeBackend) {
  "yew" -> "yew-lynx-counter"
  "dioxus" -> "lynx-element-bridge-dioxus-counter"
  else -> "lynx-element-bridge-wamr-host"
}
val elementBridgeRustArchive = rootProject.file(
    when (elementBridgeBackend) {
      "yew" -> "target/aarch64-linux-android/release/libyew_lynx_counter.a"
      "dioxus" -> "target/aarch64-linux-android/release/liblynx_element_bridge_dioxus_counter.a"
      else -> "target/aarch64-linux-android/release/liblynx_element_bridge_wamr_host.a"
    })
val elementBridgeRustLibDir =
    rootProject.file("target/android-libs/$elementBridgeBackend")
val elementBridgeGeneratedAssets =
    rootProject.file("target/android-assets/$elementBridgeBackend")
val elementBridgeWasmPackage = if (elementBridgeBackend == "wasm-yew") {
  "yew-lynx-counter"
} else {
  "lynx-element-bridge-dioxus-counter"
}
val elementBridgeWasmFile = if (elementBridgeBackend == "wasm-yew") {
  "yew_lynx_counter.wasm"
} else {
  "lynx_element_bridge_dioxus_counter.wasm"
}
val elementBridgeWasmAsset = if (elementBridgeBackend == "wasm-yew") {
  "yew_counter.wasm"
} else {
  "dioxus_counter.wasm"
}
val elementBridgeAndroidSdk = rootProject.file(
    System.getenv("ANDROID_HOME") ?: System.getenv("ANDROID_SDK_ROOT")
        ?: error("ANDROID_HOME or ANDROID_SDK_ROOT must point to the Android SDK"))
val elementBridgeNdkPrebuiltDir = elementBridgeAndroidSdk.resolve(
    "ndk/25.2.9519653/toolchains/llvm/prebuilt")
val elementBridgeNdkHostTags = System.getenv("ANDROID_NDK_HOST_TAG")
    ?.takeIf(String::isNotBlank)
    ?.let(::listOf)
    ?: when {
      System.getProperty("os.name").startsWith("Mac", ignoreCase = true) ->
          if (System.getProperty("os.arch") in setOf("aarch64", "arm64")) {
            listOf("darwin-arm64", "darwin-x86_64")
          } else {
            listOf("darwin-x86_64", "darwin-arm64")
          }
      System.getProperty("os.name").startsWith("Linux", ignoreCase = true) ->
          listOf("linux-x86_64")
      else -> error("Unsupported Android NDK host OS: ${System.getProperty("os.name")}")
    }
val elementBridgeNdkPrebuiltCandidates =
    elementBridgeNdkHostTags.map { elementBridgeNdkPrebuiltDir.resolve(it) }
val elementBridgeAndroidLlvmBin = elementBridgeNdkPrebuiltCandidates
    .firstOrNull { it.isDirectory }
    ?.resolve("bin")
    ?: error(
        "Unable to locate Android NDK prebuilt host directory. Tried:\n" +
            elementBridgeNdkPrebuiltCandidates.joinToString("\n") { "  ${it.absolutePath}" })

val buildElementBridgeRustArm64 by tasks.registering(org.gradle.api.tasks.Exec::class) {
  workingDir(rootProject.projectDir)
  environment("CC_aarch64_linux_android",
      elementBridgeAndroidLlvmBin.resolve("aarch64-linux-android24-clang"))
  environment("AR_aarch64_linux_android", elementBridgeAndroidLlvmBin.resolve("llvm-ar"))
  environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
      elementBridgeAndroidLlvmBin.resolve("aarch64-linux-android24-clang"))
  val command = mutableListOf(
      "cargo", "build", "--locked", "--release",
      "--target", "aarch64-linux-android",
      "--package", elementBridgeRustPackage)
  if (elementBridgeWasmMode) command.addAll(listOf("--features", "wamr"))
  commandLine(command)
}

val buildElementBridgeWasm by tasks.registering(org.gradle.api.tasks.Exec::class) {
  workingDir(rootProject.projectDir)
  commandLine(
      "cargo", "build", "--locked", "--release",
      "--target", "wasm32-wasip1",
      "--package", elementBridgeWasmPackage)
}

val stageElementBridgeWasm by tasks.registering(org.gradle.api.tasks.Copy::class) {
  dependsOn(buildElementBridgeWasm)
  from(rootProject.file(
      "target/wasm32-wasip1/release/$elementBridgeWasmFile"))
  into(elementBridgeGeneratedAssets)
  rename { elementBridgeWasmAsset }
}

val stageElementBridgeRustArm64 by tasks.registering(org.gradle.api.tasks.Copy::class) {
  dependsOn(buildElementBridgeRustArm64)
  from(elementBridgeRustArchive)
  into(elementBridgeRustLibDir.resolve("arm64-v8a"))
  rename { "liblynx_element_bridge_backend.a" }
}

tasks.matching { it.name == "preBuild" || it.name.startsWith("configureCMake") }
    .configureEach {
      dependsOn(stageElementBridgeRustArm64)
      if (elementBridgeWasmMode) dependsOn(stageElementBridgeWasm)
    }

android {
  defaultConfig {
    ndk {
      abiFilters += "arm64-v8a"
    }
    externalNativeBuild {
      cmake {
        arguments += listOf(
            "-DLYNX_ELEMENT_BRIDGE_RUST_LIB_DIR=${elementBridgeRustLibDir.absolutePath}",
            "-DLYNX_ELEMENT_BRIDGE_BACKEND=$elementBridgeBackend")
      }
    }
  }

  externalNativeBuild {
    cmake {
      path = elementBridgeAndroidDir.resolve("CMakeLists.txt")
      version = "3.22.1"
      buildStagingDirectory = rootProject.file(
          "target/android-cxx/$elementBridgeBackend")
    }
  }

  sourceSets.getByName("main") {
    java.srcDir(elementBridgeAndroidDir.resolve("src/main/java"))
    if (elementBridgeWasmMode) assets.srcDir(elementBridgeGeneratedAssets)
  }
}
