#![deny(unsafe_code)]

#[cfg(not(test))]
mod app;
#[cfg(test)]
#[path = "app_test_fixture.rs"]
mod app;
#[path = "dioxus-lynx-runtime.rs"]
mod dioxus_lynx_runtime;
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
mod wasm;

#[cfg(not(feature = "replacement-fixture"))]
const INITIAL_COUNT: u32 = 0;
#[cfg(feature = "replacement-fixture")]
const INITIAL_COUNT: u32 = 100;

pub use dioxus_lynx_runtime::*;
