#[macro_export]
macro_rules! include_lynx_style_sheet {
    ($name:literal) => {
        include_bytes!(concat!(env!("OUT_DIR"), "/", $name))
    };
}

#[cfg(feature = "yew")]
pub mod yew {
    pub use lynx_yew_runtime::prelude;
    pub use lynx_yew_runtime::{launch, launch_with_style_sheets};
}

#[cfg(feature = "dioxus")]
pub mod dioxus {
    pub use lynx_dioxus_runtime::launch;
    pub use lynx_dioxus_runtime::prelude;
}
