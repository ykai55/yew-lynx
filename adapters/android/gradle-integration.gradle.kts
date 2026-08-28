// Copy these blocks into the Android app/library module that already depends
// on stock OSS Lynx. This file is documentation, not a standalone Gradle module.
val elementBridgeAndroidDir = rootProject.file("adapters/android")
val elementBridgeBackend = providers.gradleProperty("lynxElementBridgeBackend")
    .orElse("yew").get()
require(elementBridgeBackend == "yew" || elementBridgeBackend == "dioxus")
val elementBridgeRustPackage = if (elementBridgeBackend == "yew") {
  "yew-lynx-counter"
} else {
  "lynx-element-bridge-dioxus-counter"
}
val elementBridgeRustArchive = rootProject.file(
    if (elementBridgeBackend == "yew") {
      "target/aarch64-linux-android/release/libyew_lynx_counter.a"
    } else {
      "target/aarch64-linux-android/release/liblynx_element_bridge_dioxus_counter.a"
    })
val elementBridgeRustLibDir =
    rootProject.file("target/android-libs/$elementBridgeBackend")

val buildElementBridgeRustArm64 by tasks.registering(org.gradle.api.tasks.Exec::class) {
  workingDir(rootProject.projectDir)
  commandLine(
      "cargo", "build", "--locked", "--release",
      "--target", "aarch64-linux-android",
      "--package", elementBridgeRustPackage)
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
  }
}
