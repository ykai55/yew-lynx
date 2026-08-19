// Copy these blocks into the Android app/library module that already depends
// on stock OSS Lynx. This file is documentation, not a standalone Gradle module.
val yewLynxAndroidDir = rootProject.file("adapters/android")
val yewLynxRustLibDir = rootProject.file("target/android-libs")
val yewLynxRustArchive = rootProject.file(
    "target/aarch64-linux-android/release/libyew_lynx_counter.a")

val buildYewLynxRustArm64 by tasks.registering(org.gradle.api.tasks.Exec::class) {
  workingDir(rootProject.projectDir)
  commandLine(
      "cargo", "build", "--locked", "--release",
      "--target", "aarch64-linux-android",
      "--package", "yew-lynx-counter")
}

val stageYewLynxRustArm64 by tasks.registering(org.gradle.api.tasks.Copy::class) {
  dependsOn(buildYewLynxRustArm64)
  from(yewLynxRustArchive)
  into(yewLynxRustLibDir.resolve("arm64-v8a"))
}

tasks.matching { it.name == "preBuild" || it.name.startsWith("configureCMake") }
    .configureEach {
      dependsOn(stageYewLynxRustArm64)
    }

android {
  defaultConfig {
    ndk {
      abiFilters += "arm64-v8a"
    }
    externalNativeBuild {
      cmake {
        arguments += "-DYEW_LYNX_RUST_LIB_DIR=${yewLynxRustLibDir.absolutePath}"
      }
    }
  }

  externalNativeBuild {
    cmake {
      path = yewLynxAndroidDir.resolve("CMakeLists.txt")
      version = "3.22.1"
    }
  }

  sourceSets.getByName("main") {
    java.srcDir(yewLynxAndroidDir.resolve("src/main/java"))
  }
}
