#![deny(unsafe_code)]

#[cfg(not(test))]
mod app;
#[cfg(test)]
#[path = "app_test_fixture.rs"]
mod app;
#[cfg(not(feature = "replacement-fixture"))]
const INITIAL_COUNT: u32 = 0;
#[cfg(feature = "replacement-fixture")]
const INITIAL_COUNT: u32 = 100;

lynx::dioxus::launch!(app::App);

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(unsafe_code)]
#[path = "../../native_lifecycle_tests.rs"]
mod native_lifecycle_tests;

#[cfg(test)]
#[path = "../wasm_guest_lifecycle_tests.rs"]
mod wasm_guest_lifecycle_tests;
