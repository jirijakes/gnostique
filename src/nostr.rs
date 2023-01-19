use std::sync::Arc;

use nostr_sdk::nostr::prelude::*;
use nostr_sdk::nostr::{Event, Sha256Hash, Tag};
use once_cell::sync::Lazy;
use relm4::gtk::{gdk, glib};

/// Find event ID to which the given event replies according to NIP-10.
pub fn replies(event: &Event) -> Option<Sha256Hash> {
    // Marked tags
    event
        .tags
        .iter()
        .find_map(|t| match t {
            Tag::Event(id, _, Some(Marker::Reply)) => Some(*id),
            _ => None,
        })
        .or_else(|| {
            // Positional tags
            let only_events = event
                .tags
                .iter()
                .filter(|t| matches!(t, Tag::Event(_, _, None)))
                .collect::<Vec<_>>();

            match only_events.as_slice() {
                [Tag::Event(id, _, _)] => Some(*id),
                [Tag::Event(_, _, _), .., Tag::Event(id, _, _)] => Some(*id),
                _ => None,
            }
        })
}

pub static ANONYMOUS_USER: Lazy<Arc<gdk::Texture>> = Lazy::new(|| {
    Arc::new(gdk::Texture::from_bytes(&glib::Bytes::from(include_bytes!("../resources/user.svg"))).unwrap())
});
