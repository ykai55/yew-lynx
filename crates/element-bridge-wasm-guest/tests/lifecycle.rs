use lynx_element_bridge_core::{BridgeError, CommandBatch, EventMessage, NodeId, Status};
use lynx_element_bridge_wasm_guest::{
    AbiRuntime, EventRequest, GuestApplication, GuestResult, MountRequest, PROTOCOL_VERSION_V2,
    decode_guest_response, encode_event_request, encode_mount_request,
};

struct TestApplication;

impl GuestApplication for TestApplication {
    fn mount(_: MountRequest) -> Result<(Self, CommandBatch), BridgeError> {
        Ok((
            Self,
            CommandBatch {
                sequence: 1,
                commands: Vec::new(),
                final_commit: true,
            },
        ))
    }

    fn dispatch_event(&mut self, _: EventMessage) -> Result<CommandBatch, BridgeError> {
        Ok(CommandBatch {
            sequence: 2,
            commands: Vec::new(),
            final_commit: true,
        })
    }

    fn destroy(self) -> Result<CommandBatch, BridgeError> {
        Ok(CommandBatch {
            sequence: 3,
            commands: Vec::new(),
            final_commit: true,
        })
    }
}

fn put_input(runtime: &mut AbiRuntime<TestApplication>, input: &[u8]) -> usize {
    let pointer = runtime.alloc(input.len() as u32);
    assert_ne!(pointer, 0);
    // SAFETY: `pointer` identifies the live allocation above and the lengths are identical.
    unsafe { std::ptr::copy_nonoverlapping(input.as_ptr(), pointer as *mut u8, input.len()) };
    pointer
}

#[test]
fn abi_buffers_remain_owned_until_the_matching_deallocator_runs() {
    let mut runtime = AbiRuntime::<TestApplication>::new();
    let mount = encode_mount_request(&MountRequest {
        protocol_version: PROTOCOL_VERSION_V2,
        root: NodeId::new(1).unwrap(),
    })
    .unwrap();
    let input = put_input(&mut runtime, &mount);
    let output = runtime.mount(input, mount.len() as u32);

    assert!(matches!(
        decode_guest_response(runtime.output(output).unwrap())
            .unwrap()
            .result,
        GuestResult::Ok(_)
    ));
    assert!(!runtime.dealloc(input, mount.len() as u32 - 1));
    assert!(runtime.dealloc(input, mount.len() as u32));
    assert!(!runtime.dealloc(input, mount.len() as u32));
    assert!(!runtime.output_dealloc(output.pointer, output.length - 1));
    assert!(runtime.output(output).is_some());
    assert!(runtime.output_dealloc(output.pointer, output.length));
    assert!(runtime.output(output).is_none());
    assert!(!runtime.output_dealloc(output.pointer, output.length));
}

#[test]
fn lifecycle_supports_repeated_calls_and_deterministic_errors() {
    let mut runtime = AbiRuntime::<TestApplication>::new();
    let invalid = runtime.dispatch_event(usize::MAX, 4);
    let invalid_response = decode_guest_response(runtime.output(invalid).unwrap()).unwrap();
    assert!(matches!(
        invalid_response.result,
        GuestResult::Err {
            status: Status::InvalidArgument,
            ..
        }
    ));
    assert!(runtime.output_dealloc(invalid.pointer, invalid.length));

    let mount = encode_mount_request(&MountRequest {
        protocol_version: PROTOCOL_VERSION_V2,
        root: NodeId::new(1).unwrap(),
    })
    .unwrap();
    let input = put_input(&mut runtime, &mount);
    let mounted = runtime.mount(input, mount.len() as u32);
    assert!(matches!(
        decode_guest_response(runtime.output(mounted).unwrap())
            .unwrap()
            .result,
        GuestResult::Ok(_)
    ));

    let event = encode_event_request(&EventRequest {
        protocol_version: PROTOCOL_VERSION_V2,
        event: EventMessage {
            listener: lynx_element_bridge_core::ListenerId::new(2).unwrap(),
            callback: lynx_element_bridge_core::CallbackId::new(3).unwrap(),
            content_type: "application/octet-stream".into(),
            payload: Vec::new(),
        },
    })
    .unwrap();
    let event_input = put_input(&mut runtime, &event);
    let dispatched = runtime.dispatch_event(event_input, event.len() as u32);
    assert!(matches!(
        decode_guest_response(runtime.output(dispatched).unwrap())
            .unwrap()
            .result,
        GuestResult::Ok(_)
    ));

    let destroyed = runtime.destroy();
    assert!(matches!(
        decode_guest_response(runtime.output(destroyed).unwrap())
            .unwrap()
            .result,
        GuestResult::Ok(_)
    ));
    let destroyed_again = runtime.destroy();
    assert!(matches!(
        decode_guest_response(runtime.output(destroyed_again).unwrap())
            .unwrap()
            .result,
        GuestResult::Err {
            status: Status::InvalidSession,
            ..
        }
    ));
}
