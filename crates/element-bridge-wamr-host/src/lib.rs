#![cfg_attr(not(feature = "wamr"), forbid(unsafe_code))]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "wamr")]
mod wamr;

#[cfg(feature = "wamr")]
pub use wamr::*;
