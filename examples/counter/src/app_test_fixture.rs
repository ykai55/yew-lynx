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
            <view>
                <text>{format!("Count: {}", self.count)}</text>
                <view ontap={increment}>
                    <text>{"Increment"}</text>
                </view>
            </view>
        }
    }
}
