plugins {
    id("com.android.library")
}

val repositoryRoot = rootProject.projectDir.parentFile.parentFile
val androidAdapterDir = repositoryRoot.resolve("adapters/android")
val androidNdkVersion = "25.2.9519653"
val offlineBuild = providers.gradleProperty("lynxElementBridgeOffline")
    .map(String::toBoolean)
    .orElse(false)
val stagedRustDirectory = repositoryRoot.resolve("target/android-libs/wamr")
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
val androidLlvmBin = androidComponents.sdkComponents.sdkDirectory.map { sdkDirectory ->
    val prebuiltDirectory = sdkDirectory.asFile.resolve(
        "ndk/$androidNdkVersion/toolchains/llvm/prebuilt"
    )
    val candidates = androidNdkHostTags.map(prebuiltDirectory::resolve)
    candidates.firstOrNull(File::isDirectory)
        ?.resolve("bin")
        ?: error(
            "Unable to locate Android NDK prebuilt host directory. Tried:\n" +
                candidates.joinToString("\n") { "  ${it.absolutePath}" }
        )
}

buildDir = repositoryRoot.resolve("target/android-build/bridge-wamr")

android {
    namespace = "com.lynx.elementbridge.wamr"
    compileSdkVersion(33)
    buildToolsVersion = "33.0.1"
    ndkVersion = androidNdkVersion
    defaultConfig {
        minSdkVersion(24)
        ndk { abiFilters.add("arm64-v8a") }
        externalNativeBuild.cmake.arguments.addAll(listOf(
            "-DLYNX_ELEMENT_BRIDGE_RUST_LIB_DIR=${stagedRustDirectory.absolutePath}",
            "-DLYNX_ELEMENT_BRIDGE_BACKEND=wasm",
            "-DLYNX_ELEMENT_BRIDGE_LIBRARY_NAME=lynx_element_bridge_wamr",
            "-DLYNX_ELEMENT_BRIDGE_JNI_CLASS=Java_com_lynx_elementbridge_wamr_WamrRendererHost_",
        ))
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    externalNativeBuild {
        cmake {
            path = androidAdapterDir.resolve("CMakeLists.txt")
            version = "3.22.1"
            buildStagingDirectory = repositoryRoot.resolve("target/android-cxx/bridge-wamr")
        }
    }
}

val buildWamrRustArm64 by tasks.registering(Exec::class) {
    workingDir(repositoryRoot)
    val llvmBin = androidLlvmBin.get()
    environment("CC_aarch64_linux_android", llvmBin.resolve("aarch64-linux-android24-clang"))
    environment("AR_aarch64_linux_android", llvmBin.resolve("llvm-ar"))
    environment(
        "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
        llvmBin.resolve("aarch64-linux-android24-clang"),
    )
    val command = mutableListOf("cargo", "build", "--locked")
    if (offlineBuild.get()) command.add("--offline")
    command.addAll(listOf(
        "--release", "--target", "aarch64-linux-android", "--package",
        "lynx-element-bridge-wamr-host", "--features", "wamr",
    ))
    commandLine(command)
}

val stageWamrRustArm64 by tasks.registering(Copy::class) {
    dependsOn(buildWamrRustArm64)
    from(repositoryRoot.resolve(
        "target/aarch64-linux-android/release/liblynx_element_bridge_wamr_host.a"
    ))
    into(stagedRustDirectory.resolve("arm64-v8a"))
    rename { "liblynx_element_bridge_backend.a" }
}

tasks.named("preBuild") { dependsOn(stageWamrRustArm64) }
tasks.matching { it.name.startsWith("configureCMake") }.configureEach {
    dependsOn(stageWamrRustArm64)
}
