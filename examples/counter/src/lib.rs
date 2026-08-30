//! Counter application backed by the Yew-Lynx native renderer runtime.

mod app;
const INITIAL_COUNT: u32 = 0;

lynx::yew::launch!(app::App);

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(unsafe_code)]
#[path = "../../native_lifecycle_tests.rs"]
mod native_lifecycle_tests;

#[cfg(test)]
#[path = "../wasm_guest_lifecycle_tests.rs"]
mod wasm_guest_lifecycle_tests;
