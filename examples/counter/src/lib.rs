//! Counter application backed by the Yew-Lynx native renderer runtime.

#[cfg(not(test))]
mod app;
#[cfg(test)]
#[path = "app_test_fixture.rs"]
mod app;
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
mod wasm;
#[path = "yew-lynx-runtime.rs"]
mod yew_lynx_runtime;

#[cfg(not(feature = "replacement-fixture"))]
const INITIAL_COUNT: u32 = 0;
#[cfg(feature = "replacement-fixture")]
const INITIAL_COUNT: u32 = 100;

pub use app::Counter;
pub use yew_lynx_runtime::*;
