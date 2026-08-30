#![deny(unsafe_code)]

mod app;
const INITIAL_COUNT: u32 = 0;

lynx::dioxus::launch!(app::App);
