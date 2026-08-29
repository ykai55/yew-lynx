pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

rootProject.name = "LynxElementBridgeAndroid"
include(":app")
include(":bridge-native")
include(":bridge-wamr")
