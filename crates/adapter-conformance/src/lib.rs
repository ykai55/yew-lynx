#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use dioxus_core::{
        AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, WriteMutations,
    };
    use lynx_element_bridge_core::{HostFake, NodeId, SessionId, TreeSnapshot};
    use lynx_element_bridge_dioxus::DioxusAdapter;
    use lynx_element_bridge_yew::YewAdapter;
    use yew::{NativeListener, NativeNode, NativeRendererBackend};

    static BUTTON_ATTRIBUTES: &[TemplateAttribute] = &[TemplateAttribute::Static {
        name: "id",
        value: "counter",
        namespace: None,
    }];
    static TEXT_CHILDREN: &[TemplateNode] = &[TemplateNode::Text { text: "Count: 0" }];
    static BUTTON_CHILDREN: &[TemplateNode] = &[TemplateNode::Element {
        tag: "text",
        namespace: None,
        attrs: &[],
        children: TEXT_CHILDREN,
    }];
    static ROOTS: &[TemplateNode] = &[TemplateNode::Element {
        tag: "view",
        namespace: None,
        attrs: BUTTON_ATTRIBUTES,
        children: BUTTON_CHILDREN,
    }];
    static COUNTER_TEMPLATE: Template = Template {
        roots: ROOTS,
        node_paths: &[],
        attr_paths: &[],
    };

    struct ScenarioResult {
        mounted: TreeSnapshot,
        updated: TreeSnapshot,
        event_payload: Vec<u8>,
        destroyed: TreeSnapshot,
    }

    #[test]
    fn yew_and_dioxus_conform_at_the_command_batch_boundary() {
        let yew = run_yew();
        let dioxus = run_dioxus();

        assert_eq!(yew.mounted, dioxus.mounted);
        assert_eq!(yew.updated, dioxus.updated);
        assert_eq!(yew.event_payload, dioxus.event_payload);
        assert_eq!(yew.destroyed, dioxus.destroyed);
        assert!(yew.destroyed.children.is_empty());
    }

    fn run_yew() -> ScenarioResult {
        let session = SessionId::new(1).unwrap();
        let root = NodeId::new(1).unwrap();
        let adapter = YewAdapter::new(session, root).unwrap();
        let button = adapter.create_element("view");
        adapter.set_attribute(button, "id", Some("counter"));
        let text = adapter.create_element("text");
        let raw = adapter.create_text("Count: 0");
        adapter.insert_before(NativeNode(1), button, None);
        adapter.insert_before(button, text, None);
        adapter.insert_before(text, raw, None);
        let tapped = Rc::new(Cell::new(false));
        let listener = adapter.add_event_listener(button, "tap", {
            let tapped = Rc::clone(&tapped);
            Box::new(move |_| tapped.set(true))
        });
        adapter.flush(NativeNode(1));
        let mut host = HostFake::new(session, root);
        host.apply(&adapter.take_batch().unwrap()).unwrap();
        let mounted = host.snapshot();

        let event = adapter
            .event(listener, "tap", "application/vnd.lynx.tap", vec![0, 255, 7])
            .unwrap();
        adapter.dispatch_event(&event).unwrap();
        assert!(tapped.get());
        adapter.set_attribute(button, "data-count", Some("1"));
        host.apply(&adapter.take_batch().unwrap()).unwrap();
        let updated = host.snapshot();

        host.apply(&adapter.destroy().unwrap()).unwrap();
        assert_eq!(host.listener_count(), 0);
        ScenarioResult {
            mounted,
            updated,
            event_payload: event.payload,
            destroyed: host.snapshot(),
        }
    }

    fn run_dioxus() -> ScenarioResult {
        let session = SessionId::new(1).unwrap();
        let root = NodeId::new(1).unwrap();
        let mut adapter = DioxusAdapter::new(session, root).unwrap();
        adapter.load_template(COUNTER_TEMPLATE, 0, ElementId(1));
        adapter.append_children(ElementId(0), 1);
        adapter.create_event_listener("tap", ElementId(1));
        let mut host = HostFake::new(session, root);
        host.apply(&adapter.take_batch().unwrap()).unwrap();
        let mounted = host.snapshot();

        let event = adapter
            .event(
                ElementId(1),
                "tap",
                "application/vnd.lynx.tap",
                vec![0, 255, 7],
            )
            .unwrap();
        assert_eq!(
            adapter.resolve_event(&event).unwrap(),
            (ElementId(1), "tap")
        );
        adapter.set_attribute(
            "data-count",
            None,
            &AttributeValue::Text("1".into()),
            ElementId(1),
        );
        host.apply(&adapter.take_batch().unwrap()).unwrap();
        let updated = host.snapshot();

        host.apply(&adapter.destroy().unwrap()).unwrap();
        assert_eq!(host.listener_count(), 0);
        ScenarioResult {
            mounted,
            updated,
            event_payload: event.payload,
            destroyed: host.snapshot(),
        }
    }

    #[allow(dead_code)]
    fn _listener_is_opaque(listener: NativeListener) -> u64 {
        listener.0
    }
}
