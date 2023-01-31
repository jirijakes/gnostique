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
    pub avatar: Option<Url>,
    pub banner: Option<Url>,
    pub about: Option<String>,
    pub nip05: Option<String>,
    pub nip05_verified: bool,
    pub metadata_json: String,
}

impl Persona {
    pub fn new(pubkey: XOnlyPublicKey) -> Persona {
        Persona {
            pubkey,
            name: None,
            nip05: None,
            avatar: None,
            banner: None,
            about: None,
            metadata_json: String::new(),
            nip05_verified: false,
        }
    }

    pub fn from_metadata(pubkey: XOnlyPublicKey, metadata: Metadata) -> Persona {
        let metadata_json = serde_json::to_string(&metadata).unwrap_or_default();
        Persona {
            pubkey,
            name: metadata.name,
            avatar: metadata.picture.and_then(|s| s.parse().ok()),
            banner: metadata.banner.and_then(|s| s.parse().ok()),
            about: metadata.about,
            nip05: metadata.nip05,
            metadata_json,
            nip05_verified: false,
        }
    }

    pub fn format_nip05(&self) -> Option<String> {
        self.nip05
            .clone()
            .map(|n| format!("✅ {}", n.strip_prefix("_@").unwrap_or(&n)))
    }

    pub fn show_nip05(&self) -> bool {
        self.nip05.is_some() && self.nip05_verified
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

    /// Format author's pubkey according to context (has or has not author name).
    pub fn format_pubkey(&self, short_len: usize, long_len: usize) -> String {
        let chars = if self.name.is_some() {
            short_len
        } else {
            long_len
        };

        self.short_bech32(chars)
    }

    pub fn tooltip(&self) -> String {
        format!(
            r###"<span alpha="70%">Pubkey hex:</span> <span color="yellow">{}</span>
<span alpha="70%">Pubkey bech32:</span> <span color="#00FF00">{}</span>
<span alpha="70%">Name:</span> <b>{}</b>
<span alpha="70%">NIP-05:</span> <span color="cyan">{}</span>
<span alpha="70%">NIP-05 verified: </span> {}"###,
            self.pubkey,
            self.pubkey.to_bech32().unwrap_or("?".to_string()),
            self.name.as_ref().unwrap_or(&"?".to_string()),
            self.nip05.as_ref().unwrap_or(&"?".to_string()),
            self.nip05_verified
        )
    }
}

pub trait EventExt {
    /// Find client that generated the event.
    fn client(&self) -> Option<String>;

    /// Find event ID to which the given event replies according to NIP-10.
    /// Returns `None` if the event is not of kind 1.
    fn replies_to(&self) -> Option<Sha256Hash>;

    fn thread_root(&self) -> Option<Sha256Hash>;

    /// Find event ID to which this event reacts to according to NIP-25.
    /// Returns `None` if the event is not of kind 7.
    fn reacts_to(&self) -> Option<Sha256Hash>;

    fn as_metadata(&self) -> Option<Metadata>;

    fn as_pretty_json(&self) -> String;

    fn augment_content(&self) -> String;

    fn collect_relays(&self) -> Vec<Url>;
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

    fn thread_root(&self) -> Option<Sha256Hash> {
        if self.kind != Kind::TextNote {
            None
        } else {
            // Marked tags
            self.tags
                .iter()
                .find_map(|t| match t {
                    Tag::Event(id, _, Some(Marker::Root)) => Some(*id),
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
                        [_, .., Tag::Event(id, _, _)] => Some(*id),
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

    /// Takes content of the event and replaces links by appropriate `<a>` tags.
    ///
    /// The method does not care about the content of the content. It is responsibility
    /// of the caller to assure that the content is supposed to be plain text.
    fn augment_content(&self) -> String {
        use linkify::*;

        let content = &self.content;

        // Replace web links by normal web link.
        let www: String = LinkFinder::new()
            .spans(&html_escape::encode_text(content.trim()))
            .map(|span| {
                let s = span.as_str();
                match span.kind() {
                    Some(LinkKind::Url) => {
                        format!(r#"<a href="{s}" title="{s}">{s}</a>"#)
                    }
                    _ => s.to_string(),
                }
            })
            .collect();

        // Replace hashtags by internal nostr URL.
        let tags = regex::Regex::new("#(?P<tag>[a-zA-Z0-9]+)")
            .unwrap()
            .replace_all(&www, |caps: &regex::Captures| {
                format!(
                    r###"<a href="nostr:search?t={}" title="#{}">#{}</a>"###,
                    caps["tag"].to_lowercase(),
                    caps["tag"].to_lowercase(),
                    &caps["tag"]
                )
            });

        // Replace mentions. This has to be made much more sofisticated.
        regex::Regex::new("#\\[(?P<idx>\\d+)\\]")
            .unwrap()
            .replace_all(&tags, |caps: &regex::Captures| {
                let idx: usize = caps["idx"].parse().unwrap();
                let id = match self.tags.get(idx) {
                    Some(Tag::Event(id, _, _)) => id.to_string(),
                    Some(Tag::PubKey(pubkey, _)) => pubkey.to_string(),
                    _ => caps["idx"].to_string(),
                };
                format!(r#"<a href="nostr:{id}">{id}</a>"#)
            })
            .into()
    }

    fn collect_relays(&self) -> Vec<Url> {
        self.tags
            .iter()
            .filter_map(|t| match t {
                Tag::Event(_, Some(r), _) => r.parse().ok(),
                Tag::PubKey(_, Some(r)) => r.parse().ok(),
                Tag::ContactList {
                    relay_url: Some(r), ..
                } => r.parse().ok(),
                Tag::Relay(url) => Some(url.clone()),
                _ => None,
            })
            .collect()
    }
}
