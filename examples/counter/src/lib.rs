//! Counter application backed by the Yew-Lynx native renderer runtime.

mod app;

lynx::yew::launch!(app::App);
