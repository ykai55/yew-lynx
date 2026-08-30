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
        <view>
            <text>{format!("Count: {}", *count)}</text>
            <view ontap={increment}>
                <text>{"Increment"}</text>
            </view>
        </view>
    }
}
