plugins {
    id("com.android.application")
    kotlin("android")
}

val repositoryRoot = rootProject.projectDir.parentFile.parentFile
val androidAdapterDir = repositoryRoot.resolve("adapters/android")
val mtsAdapterDir = repositoryRoot.resolve("adapters/mts")
val rustArchive = repositoryRoot.resolve(
    "target/aarch64-linux-android/release/libyew_lynx_counter.a"
)
val stagedRustDirectory = repositoryRoot.resolve("target/android-libs")
val generatedAssetsDirectory = buildDir.resolve("generated/yewLynxAssets")
val templateBundle = mtsAdapterDir.resolve("dist/yew-lynx-counter.lynx.bundle")
val offlineBuild = providers.gradleProperty("yewLynxOffline").map(String::toBoolean).orElse(false)

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
                arguments.add(
                    "-DYEW_LYNX_RUST_LIB_DIR=${stagedRustDirectory.absolutePath}"
                )
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
        }
    }

    sourceSets.getByName("main") {
        java.srcDir(androidAdapterDir.resolve("src/main/java"))
        assets.srcDir(generatedAssetsDirectory)
    }
}

val installMtsDependencies by tasks.registering(Exec::class) {
    workingDir(mtsAdapterDir)
    commandLine(if (offlineBuild.get()) listOf("npm", "ci", "--offline") else listOf("npm", "ci"))
    inputs.files(
        mtsAdapterDir.resolve("package.json"),
        mtsAdapterDir.resolve("package-lock.json")
    )
    outputs.dir(mtsAdapterDir.resolve("node_modules"))
    outputs.upToDateWhen { !offlineBuild.get() }
}

val buildYewLynxTemplate by tasks.registering(Exec::class) {
    dependsOn(installMtsDependencies)
    workingDir(mtsAdapterDir)
    commandLine("npm", "run", "build")
    inputs.file(mtsAdapterDir.resolve("scripts/build-template.mjs"))
    inputs.dir(mtsAdapterDir.resolve("src"))
    inputs.dir(mtsAdapterDir.resolve("template"))
    outputs.file(templateBundle)
}

val stageYewLynxTemplate by tasks.registering(Copy::class) {
    dependsOn(buildYewLynxTemplate)
    from(templateBundle)
    into(generatedAssetsDirectory)
}

val buildYewLynxRustArm64 by tasks.registering(Exec::class) {
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
            "yew-lynx-counter"
        )
    )
    commandLine(command)
}

val stageYewLynxRustArm64 by tasks.registering(Copy::class) {
    dependsOn(buildYewLynxRustArm64)
    from(rustArchive)
    into(stagedRustDirectory.resolve("arm64-v8a"))
}

tasks.named("preBuild") {
    dependsOn(stageYewLynxTemplate, stageYewLynxRustArm64)
}
tasks.matching { it.name.startsWith("configureCMake") }.configureEach {
    dependsOn(stageYewLynxRustArm64)
}

dependencies {
    val lynxVersion: String by rootProject.extra
    implementation("org.lynxsdk.lynx:lynx:$lynxVersion")
    implementation("com.google.code.gson:gson:2.8.5")
}
