use std::sync::Arc;

use nostr_sdk::nostr::prelude::*;
use nostr_sdk::nostr::{Event, Sha256Hash, Tag};
use once_cell::sync::Lazy;
use relm4::gtk::{gdk, glib};

pub static ANONYMOUS_USER: Lazy<Arc<gdk::Texture>> = Lazy::new(|| {
    Arc::new(
        gdk::Texture::from_bytes(&glib::Bytes::from(include_bytes!("../resources/user.svg")))
            .unwrap(),
    )
});

#[derive(Clone, Debug)]
pub struct Persona {
    pub name: Option<String>,
    pub pubkey: XOnlyPublicKey,
    pub nip05: Option<String>,
}

impl Persona {
    pub fn new(pubkey: XOnlyPublicKey) -> Persona {
        Persona {
            pubkey,
            name: None,
            nip05: None,
        }
    }

    pub fn format_nip05(&self) -> Option<String> {
        self.nip05
            .clone()
            .map(|n| format!("✅ {}", n.strip_prefix("_@").unwrap_or(&n)))
    }

    /// Format author's pubkey according to context (has or has not author name).
    pub fn format_pubkey(&self, short_len: usize, long_len: usize) -> String {
        let chars = if self.name.is_some() {
            short_len
        } else {
            long_len
        };

        let s = self.pubkey.to_string();
        let (pre, tail) = s.split_at(chars);
        let (_, post) = tail.split_at(tail.len() - chars);
        format!("{pre}…{post}")
    }
}

pub trait EventExt {
    /// Find client that generated the event.
    fn client(&self) -> Option<String>;

    /// Find event ID to which the given event replies according to NIP-10.
    /// Returns `None` if the event is not of kind 1.
    fn replies_to(&self) -> Option<Sha256Hash>;

    /// Find event ID to which this event reacts to according to NIP-25.
    /// Returns `None` if the event is not of kind 7.
    fn reacts_to(&self) -> Option<Sha256Hash>;

    fn as_metadata(&self) -> Option<Metadata>;

    fn as_pretty_json(&self) -> String;
}

impl EventExt for Event {
    fn client(&self) -> Option<String> {
        self.tags.iter().find_map(|t| match t {
            Tag::Generic(TagKind::Custom(tag), s) if tag.as_str() == "client" => s.first().cloned(),
            _ => None,
        })
    }

    fn replies_to(&self) -> Option<Sha256Hash> {
        if self.kind != Kind::TextNote {
            None
        } else {
            // Marked tags
            self.tags
                .iter()
                .find_map(|t| match t {
                    Tag::Event(id, _, Some(Marker::Reply)) => Some(*id),
                    _ => None,
                })
                .or_else(|| {
                    // Positional tags
                    let only_events = self
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
    }

    fn reacts_to(&self) -> Option<Sha256Hash> {
        if self.kind != Kind::Reaction {
            None
        } else {
            self.tags.iter().rev().find_map(|t| match t {
                Tag::Event(hash, _, _) => Some(*hash),
                _ => None,
            })
        }
    }

    fn as_metadata(&self) -> Option<Metadata> {
        Metadata::from_json(&self.content).ok()
    }

    fn as_pretty_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Could not serialize Event?")
    }
}
