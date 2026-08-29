import java.util.Locale

plugins {
    id("com.android.library")
}

val repositoryRoot = rootProject.projectDir.parentFile.parentFile
val androidAdapterDir = repositoryRoot.resolve("adapters/android")
val androidNdkVersion = "25.2.9519653"
val offlineBuild = providers.gradleProperty("lynxElementBridgeOffline")
    .map(String::toBoolean)
    .orElse(false)
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

buildDir = repositoryRoot.resolve("target/android-build/bridge-native")

android {
    namespace = "com.lynx.elementbridge.nativebridge"
    compileSdkVersion(33)
    buildToolsVersion = "33.0.1"
    ndkVersion = androidNdkVersion

    defaultConfig {
        minSdkVersion(24)
        ndk { abiFilters.add("arm64-v8a") }
    }
    flavorDimensions.add("nativeFramework")
    productFlavors {
        create("yew") {
            dimension = "nativeFramework"
            externalNativeBuild.cmake.arguments.addAll(listOf(
                "-DLYNX_ELEMENT_BRIDGE_RUST_LIB_DIR=${repositoryRoot.resolve("target/android-libs/native-yew").absolutePath}",
                "-DLYNX_ELEMENT_BRIDGE_BACKEND=yew",
                "-DLYNX_ELEMENT_BRIDGE_LIBRARY_NAME=lynx_element_bridge_native",
                "-DLYNX_ELEMENT_BRIDGE_JNI_CLASS=Java_com_lynx_elementbridge_nativebridge_NativeRendererHost_",
            ))
        }
        create("dioxus") {
            dimension = "nativeFramework"
            externalNativeBuild.cmake.arguments.addAll(listOf(
                "-DLYNX_ELEMENT_BRIDGE_RUST_LIB_DIR=${repositoryRoot.resolve("target/android-libs/native-dioxus").absolutePath}",
                "-DLYNX_ELEMENT_BRIDGE_BACKEND=dioxus",
                "-DLYNX_ELEMENT_BRIDGE_LIBRARY_NAME=lynx_element_bridge_native",
                "-DLYNX_ELEMENT_BRIDGE_JNI_CLASS=Java_com_lynx_elementbridge_nativebridge_NativeRendererHost_",
            ))
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    externalNativeBuild {
        cmake {
            path = androidAdapterDir.resolve("CMakeLists.txt")
            version = "3.22.1"
            buildStagingDirectory = repositoryRoot.resolve("target/android-cxx/bridge-native")
        }
    }
}

mapOf(
    "Yew" to Pair("yew-lynx-counter", "libyew_lynx_counter.a"),
    "Dioxus" to Pair(
        "lynx-element-bridge-dioxus-counter",
        "liblynx_element_bridge_dioxus_counter.a",
    ),
).forEach { (variant, rust) ->
    val backend = variant.toLowerCase(Locale.ROOT)
    val buildRust = tasks.register<Exec>("build${variant}NativeRustArm64") {
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
            "--release", "--target", "aarch64-linux-android", "--package", rust.first,
        ))
        commandLine(command)
    }
    val stageRust = tasks.register<Copy>("stage${variant}NativeRustArm64") {
        dependsOn(buildRust)
        from(repositoryRoot.resolve("target/aarch64-linux-android/release/${rust.second}"))
        into(repositoryRoot.resolve("target/android-libs/native-$backend/arm64-v8a"))
        rename { "liblynx_element_bridge_backend.a" }
    }
    tasks.matching {
        it.name.startsWith("pre$variant") || it.name.startsWith("configureCMake$variant")
    }.configureEach {
        dependsOn(stageRust)
    }
}
