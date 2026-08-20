//! Counter application backed by the Yew-Lynx runtime.

mod app;
#[path = "yew-lynx-runtime.rs"]
mod yew_lynx_runtime;

pub use app::Counter;
pub use yew_lynx_runtime::*;
