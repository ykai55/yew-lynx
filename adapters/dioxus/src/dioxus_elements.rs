#![allow(non_upper_case_globals)]

pub type AttributeDescription = (&'static str, Option<&'static str>, bool);

pub mod global_attributes {
    use super::AttributeDescription;

    pub const id: AttributeDescription = ("id", None, false);
    pub const class: AttributeDescription = ("class", None, false);
    pub const style: AttributeDescription = ("style", None, false);
}

pub mod elements {
    pub mod view {
        pub use super::super::global_attributes::*;

        pub const TAG_NAME: &str = "view";
        pub const NAME_SPACE: Option<&str> = None;
    }

    pub mod text {
        pub use super::super::global_attributes::*;

        pub const TAG_NAME: &str = "text";
        pub const NAME_SPACE: Option<&str> = None;
    }
}

pub mod events {
    use dioxus_core::{Attribute, ListenerCallback, SuperInto};

    pub fn ontap<Marker>(
        event_handler: impl SuperInto<ListenerCallback<Vec<u8>>, Marker>,
    ) -> Attribute {
        let event_handler: ListenerCallback<Vec<u8>> = event_handler.super_into();
        Attribute::new("ontap", event_handler, None, false)
    }

    pub mod ontap {
        use dioxus_core::{Attribute, Event, SpawnIfAsync};

        pub fn call_with_explicit_closure<Marker, Return>(
            event_handler: impl FnMut(Event<Vec<u8>>) -> Return + 'static,
        ) -> Attribute
        where
            Return: SpawnIfAsync<Marker> + 'static,
        {
            super::ontap(event_handler)
        }
    }
}

pub use elements::{text, view};
