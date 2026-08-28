use yew::NativeEvent;
use yew::prelude::*;

use crate::INITIAL_COUNT;

pub struct Counter {
    count: u32,
}

pub enum CounterMessage {
    Increment,
}

impl Component for Counter {
    type Message = CounterMessage;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        Self {
            count: INITIAL_COUNT,
        }
    }

    fn update(&mut self, _: &Context<Self>, message: Self::Message) -> bool {
        match message {
            CounterMessage::Increment => self.count += 1,
        }
        true
    }

    fn view(&self, context: &Context<Self>) -> Html {
        let increment = context
            .link()
            .callback(|_: NativeEvent| CounterMessage::Increment);
        html! {
            <view style="height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center;">
                <text id="counter-value" style="font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 32px;">{format!("Count: {}", self.count)}</text>
                <view id="counter-increment" style="height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;" ontap={increment}>
                    <text style="font-size: 28px; font-weight: 600; color: #ffffff;">{"Increment"}</text>
                </view>
            </view>
        }
    }
}
