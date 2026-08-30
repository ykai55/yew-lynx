//! Counter application backed by the Yew-Lynx native renderer runtime.

mod app;
const INITIAL_COUNT: u32 = 0;

lynx::yew::launch!(app::App);
