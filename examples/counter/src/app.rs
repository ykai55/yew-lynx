use yew::NativeEvent;
use yew::prelude::*;

pub struct Counter {
    count: u32,
    timer_fired: bool,
}

pub enum CounterMessage {
    Increment,
    TimerFired,
}

impl Component for Counter {
    type Message = CounterMessage;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        Self {
            count: 0,
            timer_fired: false,
        }
    }

    fn update(&mut self, _: &Context<Self>, message: Self::Message) -> bool {
        match message {
            CounterMessage::Increment => self.count += 1,
            CounterMessage::TimerFired => self.timer_fired = true,
        }
        true
    }

    fn view(&self, context: &Context<Self>) -> Html {
        let increment = context
            .link()
            .callback(|_: NativeEvent| CounterMessage::Increment);
        let timer_status = if self.timer_fired {
            "Timer: fired"
        } else {
            "Timer: pending"
        };

        html! {
            <view style="height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center;">
                <text id="counter-value" style="font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 20px;">{format!("Count: {}", self.count)}</text>
                <text id="timer-status" style="font-size: 28px; font-weight: 500; color: #5f665f; margin-bottom: 32px;">{timer_status}</text>
                <view id="counter-increment" style="height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;" ontap={increment}>
                    <text style="font-size: 28px; font-weight: 600; color: #ffffff;">{"Increment"}</text>
                </view>
            </view>
        }
    }
}
