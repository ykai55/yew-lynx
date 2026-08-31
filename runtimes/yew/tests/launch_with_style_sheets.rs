use lynx_yew_runtime::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! { <view class="fixture" /> }
}

lynx_yew_runtime::launch_with_style_sheets!(
    App,
    [&[0x43, 0x53, 0x53, 0x31], &[0x43, 0x53, 0x53, 0x32],]
);

#[test]
fn styled_launch_macro_compiles() {}
