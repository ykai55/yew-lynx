#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use dioxus_core::{
        AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, WriteMutations,
    };
    use lynx_element_bridge_core::{
        CapabilityRequest, CommandResult, HostFake, NodeId, ResultSlot, ResultValue, SessionId,
        Status, TreeSnapshot,
    };
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
        query: CommandResult,
        unsupported: CommandResult,
        event_payload: Vec<u8>,
        destroyed: TreeSnapshot,
    }

    #[test]
    fn yew_and_dioxus_conform_at_the_command_batch_boundary() {
        let yew = run_yew();
        let dioxus = run_dioxus();

        assert_eq!(yew.mounted, dioxus.mounted);
        assert_eq!(yew.updated, dioxus.updated);
        assert_eq!(yew.query, dioxus.query);
        assert_eq!(yew.unsupported, dioxus.unsupported);
        assert_eq!(yew.event_payload, dioxus.event_payload);
        assert_eq!(yew.destroyed, dioxus.destroyed);
        assert_eq!(yew.query.value, Some(ResultValue::String("view".into())));
        assert_eq!(yew.unsupported.status, Status::Unsupported);
        assert!(yew.destroyed.children.is_empty());
    }

    fn run_yew() -> ScenarioResult {
        let session = SessionId::new(1).unwrap();
        let root = NodeId::new(1).unwrap();
        let adapter = YewAdapter::new_with_capabilities(
            session,
            root,
            &[CapabilityRequest::optional("set_static_style")],
        )
        .unwrap();
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
        let mounted_response = host.apply(&adapter.take_batch().unwrap());
        assert_eq!(mounted_response.status, Status::Ok);
        let mounted = host.snapshot();

        let event = adapter
            .event(listener, "tap", "application/vnd.lynx.tap", vec![0, 255, 7])
            .unwrap();
        adapter.dispatch_event(&event).unwrap();
        assert!(tapped.get());
        adapter.set_attribute(button, "data-count", Some("1"));
        adapter.query_tag(button, ResultSlot::new(0)).unwrap();
        adapter
            .invoke_optional("set_static_style", ResultSlot::new(1))
            .unwrap();
        let response = host.apply(&adapter.take_batch().unwrap());
        let updated = host.snapshot();
        let query = result(&response.results, 0);
        let unsupported = result(&response.results, 1);

        let destroy_response = host.apply(&adapter.destroy().unwrap());
        assert_eq!(destroy_response.status, Status::Ok);
        assert_eq!(host.listener_count(), 0);
        ScenarioResult {
            mounted,
            updated,
            query,
            unsupported,
            event_payload: event.payload,
            destroyed: host.snapshot(),
        }
    }

    fn run_dioxus() -> ScenarioResult {
        let session = SessionId::new(1).unwrap();
        let root = NodeId::new(1).unwrap();
        let mut adapter = DioxusAdapter::new_with_capabilities(
            session,
            root,
            &[CapabilityRequest::optional("set_static_style")],
        )
        .unwrap();
        adapter.load_template(COUNTER_TEMPLATE, 0, ElementId(1));
        adapter.append_children(ElementId(0), 1);
        adapter.create_event_listener("tap", ElementId(1));
        let mut host = HostFake::new(session, root);
        let mounted_response = host.apply(&adapter.take_batch().unwrap());
        assert_eq!(mounted_response.status, Status::Ok);
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
        adapter.query_tag(ElementId(1), ResultSlot::new(0)).unwrap();
        adapter
            .invoke_optional("set_static_style", ResultSlot::new(1))
            .unwrap();
        let response = host.apply(&adapter.take_batch().unwrap());
        let updated = host.snapshot();
        let query = result(&response.results, 0);
        let unsupported = result(&response.results, 1);

        let destroy_response = host.apply(&adapter.destroy().unwrap());
        assert_eq!(destroy_response.status, Status::Ok);
        assert_eq!(host.listener_count(), 0);
        ScenarioResult {
            mounted,
            updated,
            query,
            unsupported,
            event_payload: event.payload,
            destroyed: host.snapshot(),
        }
    }

    fn result(results: &[CommandResult], slot: u32) -> CommandResult {
        results
            .iter()
            .find(|result| result.slot == Some(ResultSlot::new(slot)))
            .cloned()
            .unwrap()
    }

    #[allow(dead_code)]
    fn _listener_is_opaque(listener: NativeListener) -> u64 {
        listener.0
    }
}
