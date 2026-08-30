use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use lynx::dioxus::prelude::dioxus_core::schedule_update;
use lynx::dioxus::prelude::*;

pub(crate) struct CounterModel {
    count: Cell<u32>,
    schedule: RefCell<Option<Arc<dyn Fn() + Send + Sync>>>,
}

impl CounterModel {
    pub(crate) fn new(count: u32) -> Self {
        Self {
            count: Cell::new(count),
            schedule: RefCell::new(None),
        }
    }
}

#[allow(non_snake_case)]
pub(crate) fn App() -> Element {
    let model = use_hook(|| Rc::new(CounterModel::new(crate::INITIAL_COUNT)));
    let listener_model = Rc::clone(&model);
    let count = model.count.get();
    rsx! {
        view {
            style: "height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center; gap: 24px;",
            text {
                id: "counter-value",
                style: "font-size: 36px; font-weight: 700; color: #18201b;",
                "Dioxus ❎ Lynx"
            }
            text {
                id: "counter-value",
                style: "font-size: 36px; font-weight: 700; color: #18201b;",
                "Count: {count}"
            }
            view {
                id: "counter-increment",
                style: "height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;",
                ontap: {
                    let model = listener_model.clone();
                    move |_| {
                        model.count.set(model.count.get() + 1);
                        schedule_update()();
                    }
                },
                text {
                    style: "font-size: 28px; font-weight: 600; color: #ffffff;",
                    "Increment"
                }
            }
            view {
                id: "counter-decrement",
                style: "height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;",
                ontap: {
                    let model = listener_model.clone();
                    move |_| {
                        model.count.set(model.count.get() - 1);
                        schedule_update()();
                    }
                },
                text {
                    style: "font-size: 28px; font-weight: 600; color: #ffffff;",
                    "Decrement"
                }
            }
        }
    }
}
