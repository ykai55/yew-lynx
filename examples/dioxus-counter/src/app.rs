use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use dioxus_core::{Element, schedule_update};
use lynx_element_bridge_dioxus::prelude::*;

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

pub(crate) fn counter(model: Rc<CounterModel>) -> Element {
    let schedule = schedule_update();
    model.schedule.replace(Some(Arc::clone(&schedule)));
    let listener_model = Rc::clone(&model);
    let count = model.count.get();
    rsx! {
        view {
            style: "height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center;",
            text {
                id: "counter-value",
                style: "font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 32px;",
                "Dioxus ❎ Lynx"
            }
            text {
                id: "counter-value",
                style: "font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 32px;",
                "Count: {count}"
            }
            view {
                id: "counter-increment",
                style: "height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;",
                ontap: move |_| {
                    listener_model.count.set(listener_model.count.get() + 1);
                    schedule();
                },
                text {
                    style: "font-size: 28px; font-weight: 600; color: #ffffff;",
                    "Increment"
                }
            }
        }
    }
}
