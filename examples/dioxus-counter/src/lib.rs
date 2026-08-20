#![forbid(unsafe_code)]

use std::cell::Cell;
use std::rc::Rc;

use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, Element, Event, Template, TemplateAttribute,
    TemplateNode, VNode, VText, VirtualDom, schedule_update, use_hook,
};
use lynx_element_bridge_core::{CommandBatch, ListenerId, NodeId, SessionId};
use lynx_element_bridge_dioxus::{DioxusAdapter, DioxusAdapterError};

static VALUE_CHILDREN: &[TemplateNode] = &[TemplateNode::Dynamic { id: 0 }];
static VALUE_ATTRIBUTES: &[TemplateAttribute] = &[
    TemplateAttribute::Static {
        name: "id",
        value: "counter-value",
        namespace: None,
    },
    TemplateAttribute::Static {
        name: "style",
        value: "font-size: 36px; font-weight: 700; color: #18201b; margin-bottom: 32px;",
        namespace: None,
    },
];
static BUTTON_ATTRIBUTES: &[TemplateAttribute] = &[
    TemplateAttribute::Static {
        name: "id",
        value: "counter-increment",
        namespace: None,
    },
    TemplateAttribute::Static {
        name: "style",
        value: "height: 96px; border-radius: 20px; background-color: #176b51; display: flex; align-items: center; justify-content: center;",
        namespace: None,
    },
    TemplateAttribute::Dynamic { id: 0 },
];
static BUTTON_CHILDREN: &[TemplateNode] = &[TemplateNode::Element {
    tag: "text",
    namespace: None,
    attrs: &[TemplateAttribute::Static {
        name: "style",
        value: "font-size: 28px; font-weight: 600; color: #ffffff;",
        namespace: None,
    }],
    children: &[TemplateNode::Text { text: "Increment" }],
}];
static ROOT_CHILDREN: &[TemplateNode] = &[
    TemplateNode::Element {
        tag: "text",
        namespace: None,
        attrs: VALUE_ATTRIBUTES,
        children: VALUE_CHILDREN,
    },
    TemplateNode::Element {
        tag: "view",
        namespace: None,
        attrs: BUTTON_ATTRIBUTES,
        children: BUTTON_CHILDREN,
    },
];
static ROOTS: &[TemplateNode] = &[TemplateNode::Element {
    tag: "view",
    namespace: None,
    attrs: &[TemplateAttribute::Static {
        name: "style",
        value: "height: 100%; padding: 64px 40px; background-color: #f5f2ea; display: flex; flex-direction: column; justify-content: center;",
        namespace: None,
    }],
    children: ROOT_CHILDREN,
}];
static COUNTER_TEMPLATE: Template = Template {
    roots: ROOTS,
    node_paths: &[&[0, 0, 0]],
    attr_paths: &[&[0, 1]],
};

fn counter() -> Element {
    let count = use_hook(|| Rc::new(Cell::new(0_u32)));
    let schedule = schedule_update();
    let listener_count = Rc::clone(&count);
    let listener = Attribute::new(
        "ontap",
        AttributeValue::listener(move |_: Event<Vec<u8>>| {
            listener_count.set(listener_count.get() + 1);
            schedule();
        }),
        None,
        false,
    );
    Ok(VNode::new(
        None,
        COUNTER_TEMPLATE,
        vec![DynamicNode::Text(VText::new(format!(
            "Count: {}",
            count.get()
        )))]
        .into_boxed_slice(),
        vec![vec![listener].into_boxed_slice()].into_boxed_slice(),
    ))
}

pub struct DioxusCounter {
    dom: VirtualDom,
    adapter: DioxusAdapter,
}

impl DioxusCounter {
    pub fn mount(
        session: SessionId,
        root: NodeId,
    ) -> Result<(Self, CommandBatch), DioxusAdapterError> {
        let mut adapter = DioxusAdapter::new(session, root)?;
        let mut dom = VirtualDom::new(counter);
        dom.rebuild(&mut adapter);
        let batch = adapter.take_batch()?;
        Ok((Self { dom, adapter }, batch))
    }

    pub fn dispatch(
        &mut self,
        listener: ListenerId,
        name: &'static str,
        content_type: &str,
        payload: Vec<u8>,
    ) -> Result<CommandBatch, DioxusAdapterError> {
        let event =
            self.adapter
                .event_for_listener(listener, name, content_type, payload.clone())?;
        let target = self.adapter.event_target(&event)?;
        self.dom
            .runtime()
            .handle_event(name, Event::new(Rc::new(payload), true), target);
        self.dom.render_immediate(&mut self.adapter);
        self.adapter.take_batch()
    }

    pub fn destroy(mut self) -> Result<CommandBatch, DioxusAdapterError> {
        drop(self.dom);
        self.adapter.destroy()
    }
}

#[cfg(test)]
mod tests {
    use lynx_element_bridge_core::{Command, HostFake, Status};

    use super::*;

    #[test]
    fn real_virtual_dom_mounts_updates_and_destroys_the_counter() {
        let session = SessionId::new(1).unwrap();
        let root = NodeId::new(1).unwrap();
        let (mut counter, mounted) = DioxusCounter::mount(session, root).unwrap();
        let listener = mounted
            .commands
            .iter()
            .find_map(|item| match item.command {
                Command::AddEventListener { listener, .. } => Some(listener),
                _ => None,
            })
            .unwrap();
        let mut host = HostFake::new(session, root);
        assert_eq!(host.apply(&mounted).status, Status::Ok);
        assert_eq!(
            host.snapshot().children[0].children[0].children[0]
                .text
                .as_deref(),
            Some("Count: 0")
        );

        let updated = counter
            .dispatch(listener, "tap", "application/vnd.lynx.tap", vec![0, 255])
            .unwrap();
        assert_eq!(host.apply(&updated).status, Status::Ok);
        assert_eq!(
            host.snapshot().children[0].children[0].children[0]
                .text
                .as_deref(),
            Some("Count: 1")
        );

        let destroyed = counter.destroy().unwrap();
        assert_eq!(host.apply(&destroyed).status, Status::Ok);
        assert!(host.snapshot().children.is_empty());
        assert_eq!(host.listener_count(), 0);
    }
}
