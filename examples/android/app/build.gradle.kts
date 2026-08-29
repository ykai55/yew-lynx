plugins {
    id("com.android.application")
    kotlin("android")
}

val repositoryRoot = rootProject.projectDir.parentFile.parentFile
buildDir = repositoryRoot.resolve("target/android-build/app")

android {
    namespace = "com.yew.lynx.example"
    compileSdkVersion(33)
    buildToolsVersion = "33.0.1"
    defaultConfig {
        applicationId = "com.yew.lynx.example"
        minSdkVersion(24)
        targetSdkVersion(33)
        versionCode = 1
        versionName = "1.0"
    }
    flavorDimensions.add("nativeFramework")
    productFlavors {
        create("yew") { dimension = "nativeFramework" }
        create("dioxus") { dimension = "nativeFramework" }
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

}

dependencies {
    val lynxVersion: String by rootProject.extra
    implementation(project(":bridge-native"))
    implementation(project(":bridge-wamr"))
    implementation("org.lynxsdk.lynx:lynx-native-renderer:$lynxVersion")
    implementation("com.google.code.gson:gson:2.8.5")
    implementation("com.squareup.okhttp3:okhttp:3.12.13")
}
