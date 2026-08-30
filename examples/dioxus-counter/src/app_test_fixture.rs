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
    let schedule = schedule_update();
    model.schedule.replace(Some(Arc::clone(&schedule)));
    let listener_model = Rc::clone(&model);
    let count = model.count.get();
    rsx! {
        view {
            text { "Count: {count}" }
            view {
                ontap: move |_| {
                    listener_model.count.set(listener_model.count.get() + 1);
                    schedule();
                },
                text { "Increment" }
            }
        }
    }
}
