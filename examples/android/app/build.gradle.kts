plugins {
    id("com.android.application")
    kotlin("android")
}

val repositoryRoot = rootProject.projectDir.parentFile.parentFile
val androidAdapterDir = repositoryRoot.resolve("adapters/android")
val elementBridgeBackend = providers.gradleProperty("lynxElementBridgeBackend")
    .orElse("yew")
    .get()
require(elementBridgeBackend in setOf("yew", "dioxus", "wasm-dioxus", "wasm-yew")) {
    "lynxElementBridgeBackend must be yew, dioxus, wasm-dioxus, or wasm-yew"
}
val wasmMode = elementBridgeBackend.startsWith("wasm-")
val rustPackage = when (elementBridgeBackend) {
    "yew" -> "yew-lynx-counter"
    "dioxus" -> "lynx-element-bridge-dioxus-counter"
    else -> "lynx-element-bridge-wamr-host"
}
val rustArchiveName = when (elementBridgeBackend) {
    "yew" -> "libyew_lynx_counter.a"
    "dioxus" -> "liblynx_element_bridge_dioxus_counter.a"
    else -> "liblynx_element_bridge_wamr_host.a"
}
val rustArchive = repositoryRoot.resolve(
    "target/aarch64-linux-android/release/$rustArchiveName"
)
val stagedRustDirectory = repositoryRoot.resolve("target/android-libs/$elementBridgeBackend")
val wasmGuestPackage = if (elementBridgeBackend == "wasm-yew") {
    "yew-lynx-counter"
} else {
    "lynx-element-bridge-dioxus-counter"
}
val wasmGuestFile = if (elementBridgeBackend == "wasm-yew") {
    "yew_lynx_counter.wasm"
} else {
    "lynx_element_bridge_dioxus_counter.wasm"
}
val wasmInitialAssetName = if (elementBridgeBackend == "wasm-yew") {
    "yew_counter.wasm"
} else {
    "dioxus_counter.wasm"
}
val wasmReplacementAssetName = if (elementBridgeBackend == "wasm-yew") {
    "yew_counter_replacement.wasm"
} else {
    "dioxus_counter_replacement.wasm"
}
val wasmTargetDirectory = repositoryRoot.resolve("target/wasm-guests/$elementBridgeBackend")
val initialWasmGuest = wasmTargetDirectory.resolve("initial/wasm32-wasip1/release/$wasmGuestFile")
val replacementWasmGuest = wasmTargetDirectory.resolve(
    "replacement/wasm32-wasip1/release/$wasmGuestFile"
)
val generatedAssetsDirectory = repositoryRoot.resolve("target/android-assets/$elementBridgeBackend")
val androidSdkDirectory = file(
    System.getenv("ANDROID_HOME") ?: System.getenv("ANDROID_SDK_ROOT")
        ?: error("ANDROID_HOME or ANDROID_SDK_ROOT must point to the Android SDK")
)
val androidNdkPrebuiltDirectory = androidSdkDirectory.resolve(
    "ndk/25.2.9519653/toolchains/llvm/prebuilt"
)
val androidNdkHostTags = System.getenv("ANDROID_NDK_HOST_TAG")
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
val androidNdkPrebuiltCandidates = androidNdkHostTags.map { androidNdkPrebuiltDirectory.resolve(it) }
val androidLlvmBin = androidNdkPrebuiltCandidates.firstOrNull { it.isDirectory }
    ?.resolve("bin")
    ?: error(
        "Unable to locate Android NDK prebuilt host directory. Tried:\n" +
            androidNdkPrebuiltCandidates.joinToString("\n") { "  ${it.absolutePath}" }
    )
buildDir = repositoryRoot.resolve("target/android-build/$elementBridgeBackend/app")
val offlineBuild = providers.gradleProperty("lynxElementBridgeOffline")
    .map(String::toBoolean)
    .orElse(false)

android {
    namespace = "com.yew.lynx.example"
    compileSdkVersion(33)
    buildToolsVersion = "33.0.1"
    ndkVersion = "25.2.9519653"

    defaultConfig {
        applicationId = "com.yew.lynx.example"
        minSdkVersion(24)
        targetSdkVersion(33)
        versionCode = 1
        versionName = "1.0"
        manifestPlaceholders["nativeLauncherEnabled"] = (!wasmMode).toString()
        manifestPlaceholders["wasmUrlLauncherEnabled"] = wasmMode.toString()
        manifestPlaceholders["wasmUrlCleartextEnabled"] = wasmMode.toString()
        buildConfigField("boolean", "LYNX_ELEMENT_BRIDGE_WASM", wasmMode.toString())
        buildConfigField(
            "String",
            "LYNX_ELEMENT_BRIDGE_WASM_INITIAL_ASSET",
            "\"$wasmInitialAssetName\""
        )
        buildConfigField(
            "String",
            "LYNX_ELEMENT_BRIDGE_WASM_REPLACEMENT_ASSET",
            "\"$wasmReplacementAssetName\""
        )

        ndk {
            abiFilters.add("arm64-v8a")
        }
        externalNativeBuild {
            cmake {
                arguments.addAll(listOf(
                    "-DLYNX_ELEMENT_BRIDGE_RUST_LIB_DIR=${stagedRustDirectory.absolutePath}",
                    "-DLYNX_ELEMENT_BRIDGE_BACKEND=$elementBridgeBackend"
                ))
            }
        }
    }

    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }

    externalNativeBuild {
        cmake {
            path = androidAdapterDir.resolve("CMakeLists.txt")
            version = "3.22.1"
            buildStagingDirectory = repositoryRoot.resolve(
                "target/android-cxx/$elementBridgeBackend"
            )
        }
    }

    sourceSets.getByName("main") {
        java.srcDir(androidAdapterDir.resolve("src/main/java"))
        if (wasmMode) {
            assets.srcDir(generatedAssetsDirectory)
        }
    }
}

val buildLynxElementBridgeRustArm64 by tasks.registering(Exec::class) {
    workingDir(repositoryRoot)
    environment("CC_aarch64_linux_android", androidLlvmBin.resolve("aarch64-linux-android24-clang"))
    environment("AR_aarch64_linux_android", androidLlvmBin.resolve("llvm-ar"))
    environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", androidLlvmBin.resolve("aarch64-linux-android24-clang"))
    val command = mutableListOf(
        "cargo",
        "build",
        "--locked"
    )
    if (offlineBuild.get()) {
        command.add("--offline")
    }
    command.addAll(
        listOf(
            "--release",
            "--target",
            "aarch64-linux-android",
            "--package",
            rustPackage
        )
    )
    if (wasmMode) {
        command.addAll(listOf("--features", "wamr"))
    }
    commandLine(command)
}

val buildInitialWasmGuest by tasks.registering(Exec::class) {
    workingDir(repositoryRoot)
    environment("CARGO_TARGET_DIR", wasmTargetDirectory.resolve("initial"))
    val command = mutableListOf("cargo", "build", "--locked")
    if (offlineBuild.get()) {
        command.add("--offline")
    }
    command.addAll(listOf(
        "--release",
        "--target", "wasm32-wasip1",
        "--package", wasmGuestPackage,
    ))
    commandLine(command)
}

val buildReplacementWasmGuest by tasks.registering(Exec::class) {
    workingDir(repositoryRoot)
    environment("CARGO_TARGET_DIR", wasmTargetDirectory.resolve("replacement"))
    val command = mutableListOf("cargo", "build", "--locked")
    if (offlineBuild.get()) {
        command.add("--offline")
    }
    command.addAll(listOf(
        "--release",
        "--target", "wasm32-wasip1",
        "--package", wasmGuestPackage,
        "--features", "replacement-fixture",
    ))
    commandLine(command)
}

val stageWasmGuests by tasks.registering(Copy::class) {
    dependsOn(buildInitialWasmGuest, buildReplacementWasmGuest)
    from(initialWasmGuest) {
        rename { wasmInitialAssetName }
    }
    from(replacementWasmGuest) {
        rename { wasmReplacementAssetName }
    }
    into(generatedAssetsDirectory)
}

val stageLynxElementBridgeRustArm64 by tasks.registering(Copy::class) {
    dependsOn(buildLynxElementBridgeRustArm64)
    from(rustArchive)
    into(stagedRustDirectory.resolve("arm64-v8a"))
    rename { "liblynx_element_bridge_backend.a" }
}

tasks.named("preBuild") {
    dependsOn(stageLynxElementBridgeRustArm64)
    if (wasmMode) {
        dependsOn(stageWasmGuests)
    }
}
tasks.matching { it.name.startsWith("configureCMake") }.configureEach {
    dependsOn(stageLynxElementBridgeRustArm64)
}

dependencies {
    val lynxVersion: String by rootProject.extra
    implementation("org.lynxsdk.lynx:lynx-native-renderer:$lynxVersion")
    implementation("com.google.code.gson:gson:2.8.5")
}
