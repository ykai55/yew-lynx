// Reference integration for an application that consumes both runtime modules.
// The concrete Cargo, CMake, NDK, and staging configuration lives in
// examples/android/bridge-native and examples/android/bridge-wamr.

android {
  flavorDimensions.add("nativeFramework")
  productFlavors {
    create("yew") { dimension = "nativeFramework" }
    create("dioxus") { dimension = "nativeFramework" }
  }
}

dependencies {
  implementation(project(":bridge-native"))
  implementation(project(":bridge-wamr"))
  implementation("org.lynxsdk.lynx:lynx-native-renderer:0.0.1-0df14207")
}
