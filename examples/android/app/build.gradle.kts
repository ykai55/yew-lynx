plugins {
    id("com.android.application")
    kotlin("android")
}

val repositoryRoot = rootProject.projectDir.parentFile.parentFile
val androidAdapterDir = repositoryRoot.resolve("adapters/android")
val elementBridgeBackend = providers.gradleProperty("lynxElementBridgeBackend")
    .orElse("yew")
    .get()
require(elementBridgeBackend == "yew" || elementBridgeBackend == "dioxus") {
    "lynxElementBridgeBackend must be yew or dioxus"
}
val rustPackage = if (elementBridgeBackend == "yew") {
    "yew-lynx-counter"
} else {
    "lynx-element-bridge-dioxus-counter"
}
val rustArchiveName = if (elementBridgeBackend == "yew") {
    "libyew_lynx_counter.a"
} else {
    "liblynx_element_bridge_dioxus_counter.a"
}
val rustArchive = repositoryRoot.resolve(
    "target/aarch64-linux-android/release/$rustArchiveName"
)
val stagedRustDirectory = repositoryRoot.resolve("target/android-libs/$elementBridgeBackend")
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
    }
}

val buildLynxElementBridgeRustArm64 by tasks.registering(Exec::class) {
    workingDir(repositoryRoot)
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
    commandLine(command)
}

val stageLynxElementBridgeRustArm64 by tasks.registering(Copy::class) {
    dependsOn(buildLynxElementBridgeRustArm64)
    from(rustArchive)
    into(stagedRustDirectory.resolve("arm64-v8a"))
    rename { "liblynx_element_bridge_backend.a" }
}

tasks.named("preBuild") {
    dependsOn(stageLynxElementBridgeRustArm64)
}
tasks.matching { it.name.startsWith("configureCMake") }.configureEach {
    dependsOn(stageLynxElementBridgeRustArm64)
}

dependencies {
    val lynxVersion: String by rootProject.extra
    implementation("org.lynxsdk.lynx:lynx-native-renderer:$lynxVersion")
    implementation("com.google.code.gson:gson:2.8.5")
}
