use lynx_element_bridge_core::{
    BridgeError, CallbackId, CommandBatch, EventMessage, ListenerId, NodeId, Session, Status,
};
use lynx_element_bridge_wasm_guest::{GuestApplication, MountRequest, export_guest};

struct Fixture {
    session: Session,
    view: NodeId,
    listener: ListenerId,
}

impl GuestApplication for Fixture {
    fn mount(request: MountRequest) -> Result<(Self, CommandBatch), BridgeError> {
        let mut session = Session::create(request.root)?;
        let view = session.create_element("view")?;
        session.set_attribute(view, "data-state", Some("mounted"))?;
        session.insert_before(request.root, view, None)?;
        let listener = session.add_event_listener(view, "tap", CallbackId::new(1)?)?;
        let initial = session.take_batch()?;
        Ok((
            Self {
                session,
                view,
                listener,
            },
            initial,
        ))
    }

    fn dispatch_event(&mut self, event: EventMessage) -> Result<CommandBatch, BridgeError> {
        if event.listener != self.listener || event.callback != CallbackId::new(1)? {
            return Err(BridgeError::new(
                Status::InvalidListener,
                "fixture event identity mismatch",
            ));
        }
        self.session
            .set_attribute(self.view, "data-state", Some("event"))?;
        self.session.take_batch()
    }

    fn destroy(mut self) -> Result<CommandBatch, BridgeError> {
        self.session.destroy()
    }
}

export_guest!(Fixture);

fn main() {}
