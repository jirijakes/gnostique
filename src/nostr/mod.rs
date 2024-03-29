pub mod content;
pub mod gnevent;
mod parse;
pub mod preview;
pub mod subscriptions;

pub use std::sync::Arc;

use nostr_sdk::nostr::prelude::*;
use nostr_sdk::nostr::{Event, EventId, Tag};
use nostr_sdk::prelude::rand::rngs::OsRng;
use nostr_sdk::prelude::rand::*;
use once_cell::sync::Lazy;
use relm4::gtk::{gdk, glib};
use vec1::Vec1;

use self::content::DynamicContent;
use self::gnevent::GnEvent;

pub static ANONYMOUS_USER: Lazy<Arc<gdk::Texture>> = Lazy::new(|| {
    Arc::new(
        gdk::Texture::from_bytes(&glib::Bytes::from(include_bytes!(
            "../../resources/user.svg"
        )))
        .unwrap(),
    )
});

/// Event that we received from a particular relay.
#[derive(Clone, Debug)]
pub struct ReceivedEvent {
    pub event: Event,
    pub relay: Url,
}

impl ReceivedEvent {
    pub fn prepare_content(&self) -> DynamicContent {
        parse::parse_content(self)
    }
}

/// Reference to an event with `id` should be found on `relays`.
#[derive(Clone, Debug)]
pub struct EventRef {
    /// ID of event.
    event_id: EventId,

    /// Relays that are expected to know this event ID.
    relays: Vec1<Url>,
}

impl EventRef {
    pub fn new(event_id: EventId, relays: Vec1<Url>) -> EventRef {
        EventRef { event_id, relays }
    }

    pub fn id(&self) -> EventId {
        self.event_id
    }

    pub fn relays(&self) -> &Vec1<Url> {
        &self.relays
    }
}

#[derive(Clone, Debug)]
pub struct Repost(GnEvent);

impl Repost {
    pub fn new(event: GnEvent) -> Repost {
        Repost(event)
    }

    pub fn event(&self) -> &Event {
        self.0.event()
    }

    pub fn author(&self) -> &Persona {
        self.0.author()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct TextNote(GnEvent);

impl TextNote {
    pub fn new(event: GnEvent) -> TextNote {
        TextNote(event)
    }

    pub fn event(&self) -> &Event {
        self.0.event()
    }

    pub fn underlying(self) -> (Arc<Event>, Arc<Persona>) {
        self.0.underlying()
    }

    pub fn author(&self) -> &Persona {
        self.0.author()
    }
}

#[derive(Clone, Debug)]
pub struct Persona {
    // TODO: Add relays?
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub pubkey: XOnlyPublicKey,
    pub avatar: Option<reqwest::Url>,
    pub banner: Option<reqwest::Url>,
    pub about: Option<String>,
    pub nip05: Option<String>,
    pub nip05_preverified: bool,
    pub metadata_json: String,
}

impl std::hash::Hash for Persona {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pubkey.hash(state);
    }
}

impl Eq for Persona {}

impl PartialEq for Persona {
    fn eq(&self, other: &Self) -> bool {
        self.pubkey == other.pubkey
    }
}

impl Persona {
    pub fn new(pubkey: XOnlyPublicKey) -> Persona {
        Persona {
            pubkey,
            name: None,
            display_name: None,
            nip05: None,
            avatar: None,
            banner: None,
            about: None,
            metadata_json: String::new(),
            nip05_preverified: false,
        }
    }

    pub fn from_metadata(pubkey: XOnlyPublicKey, metadata: Metadata) -> Persona {
        let metadata_json = serde_json::to_string(&metadata).unwrap_or_default();
        Persona {
            pubkey,
            name: metadata.name,
            display_name: metadata.display_name,
            avatar: metadata.picture.and_then(|s| s.parse().ok()),
            banner: metadata.banner.and_then(|s| s.parse().ok()),
            about: metadata.about,
            nip05: metadata.nip05,
            metadata_json,
            nip05_preverified: false,
        }
    }

    pub fn format_nip05(&self) -> Option<String> {
        self.nip05
            .clone()
            .map(|n| format!("✅ {}", n.strip_prefix("_@").unwrap_or(&n)))
    }

    fn shortened(s: &str, chars: usize) -> String {
        let (pre, tail) = s.split_at(chars + 5);
        let pre = pre.replace("npub1", r#"<span alpha="50%">npub1</span>"#);
        let (_, post) = tail.split_at(tail.len() - chars);
        format!("{pre}…{post}")
    }

    pub fn short_bech32(&self, chars: usize) -> String {
        Self::shortened(&self.pubkey.to_bech32().unwrap(), chars)
    }

    pub fn short_pubkey(&self, chars: usize) -> String {
        Self::shortened(&self.pubkey.to_string(), chars)
    }

    /// Returns profile's display name or name, whichever is available.
    /// The resulting name is shortened to 60 characters.
    pub fn show_name(&self) -> Option<String> {
        self.display_name
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                self.name
                    .as_ref()
                    .filter(|s| !s.trim().is_empty())
                    .map(|n| format!("@{n}"))
            })
            .map(|mut s| {
                if s.len() > 60 {
                    s.truncate(60);
                    format!("{s}…")
                } else {
                    s
                }
            })
    }

    /// Format author's pubkey according to context (has or has not author name).
    pub fn format_pubkey(&self, short_len: usize, long_len: usize) -> String {
        let chars = if self.name.is_some() {
            short_len
        } else {
            long_len
        };

        self.short_bech32(chars)
    }
}

pub trait EventExt {
    /// Find client that generated the event.
    fn client(&self) -> Option<String>;

    /// Find event ID to which the given event replies according to NIP-10.
    /// Returns `None` if the event is not of kind 1.
    fn replies_to(&self) -> Option<EventId>;

    /// If this event is a text note and part of a thread, finds its root.
    fn thread_root(&self) -> Option<(EventId, Option<Url>)>;

    /// Find event ID to which this event reacts to according to NIP-25.
    /// Returns `None` if the event is not of kind 7.
    fn reacts_to(&self) -> Option<EventId>;

    /// If this event is metadata, tries to parse it.
    fn as_metadata(&self) -> Option<Metadata>;

    fn as_pretty_json(&self) -> String;

    /// Find all relays in this event.
    fn collect_relays(&self) -> Vec<UncheckedUrl>;
}

impl EventExt for Event {
    fn client(&self) -> Option<String> {
        self.tags.iter().find_map(|t| match t {
            Tag::Generic(TagKind::Custom(tag), s) if tag.as_str() == "client" => s.first().cloned(),
            _ => None,
        })
    }

    fn replies_to(&self) -> Option<EventId> {
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

    fn thread_root(&self) -> Option<(EventId, Option<Url>)> {
        if self.kind != Kind::TextNote {
            None
        } else {
            // Marked tags
            self.tags
                .iter()
                .find_map(|t| match t {
                    Tag::Event(id, relay, Some(Marker::Root)) => {
                        Some((*id, relay.as_ref().and_then(|s| s.clone().try_into().ok())))
                    }
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
                        [_, .., Tag::Event(id, relay, _)] => {
                            Some((*id, relay.as_ref().and_then(|s| s.clone().try_into().ok())))
                        }
                        _ => None,
                    }
                })
        }
    }

    fn reacts_to(&self) -> Option<EventId> {
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

    fn collect_relays(&self) -> Vec<UncheckedUrl> {
        self.tags
            .iter()
            .filter_map(|t| match t {
                Tag::Event(_, Some(r), _) => Some(r.clone()),
                Tag::PubKey(_, Some(r)) => Some(r.clone()),
                Tag::ContactList {
                    relay_url: Some(r), ..
                } => Some(r.clone()),
                Tag::Relay(url) => Some(url.clone()),
                _ => None,
            })
            .collect()
    }
}

pub fn mnemonic() {
    let m = Mnemonic::from_entropy(&OsRng.gen::<[u8; 32]>()).unwrap();
    m.word_iter().for_each(|w| print!("{w} "));
    println!();
    let m = Mnemonic::from_entropy(&OsRng.gen::<[u8; 32]>()).unwrap();
    m.word_iter().for_each(|w| print!("{w} "));
    println!();
}
