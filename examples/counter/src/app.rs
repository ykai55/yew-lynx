use lynx::yew::prelude::*;

use crate::INITIAL_COUNT;

#[function_component(App)]
pub(crate) fn app() -> Html {
    let count = use_state(|| INITIAL_COUNT);
    let increment = {
        let count = count.clone();
        Callback::from(move |_: NativeEvent| count.set(*count + 1))
    };
    html! {
        <view style="height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center;">
            <text id="counter-title" style="font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 32px;">{"Yew ❎ Lynx"}</text>
            <text id="counter-value" style="font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 32px;">{format!("Count: {}", *count)}</text>
            <view id="counter-increment" style="height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;" ontap={increment}>
                <text style="font-size: 28px; font-weight: 600; color: #ffffff;">{"Increment"}</text>
            </view>
        </view>
    }
}
