#![deny(unsafe_code)]

mod app;

lynx::dioxus::launch!(app::App);
