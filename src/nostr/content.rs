use std::fmt::Debug;
use std::ops::Range;
use std::sync::Arc;

use nostr_sdk::prelude::*;

use super::preview::Preview;
use super::Persona;

#[derive(Clone)]
struct Void;

/// Dynamic part of a content of a text note.

/// This /dynamic content/ can be modified depending on a reality known at a time.
/// For example, at the time of receiving a text note, we know nothing about referenced
/// pubkeys. We only know the pubkey itself (and we do not even know whether it is
/// used by somebody). So at that moment, we can only show the pubkey. However,
/// `DynamicContent` will be aware of this reference and when later we receive full
/// profile belonging to this pubkey, we can /augment/ the content by showing up-to-date
/// data (i. e. name of the pubkey's profile).
///
/// Additionally, `DynamicContent` also holds list of all references that may be
/// accompanied by rich content in UI. Every dynamic content can be a reference.
///
/// It is important to understand that `DynamicContent` does not hold the content
/// itself, only list of locations in the original content that may be dynamically
/// modified. One has to provide the original content to render final result
/// (most likely from `Event` object).
#[derive(Clone, Default)]
pub struct DynamicContent {
    profiles: Vec<Hole<Persona>>,
    events: Vec<Hole<Event>>,
    urls: Vec<Hole<Preview>>,
    other: Vec<Hole<Void>>,
    references: Vec<Reference>,
}

impl DynamicContent {
    /// Creates new, empty content.
    pub fn new() -> DynamicContent {
        Default::default()
    }

    /// Augments `original` content with whatever is available at the moment.
    pub fn augment(&self, original: &str) -> String {
        let mut ranges: Vec<(Range<usize>, &str)> = Vec::new();

        for p in &self.profiles {
            ranges.push((p.range.clone(), if p.hidden { "" } else { &p.replace_with }));
        }

        for e in &self.events {
            ranges.push((e.range.clone(), if e.hidden { "" } else { &e.replace_with }));
        }

        for e in &self.urls {
            ranges.push((e.range.clone(), if e.hidden { "" } else { &e.replace_with }));
        }

        for v in &self.other {
            ranges.push((v.range.clone(), if v.hidden { "" } else { &v.replace_with }));
        }

        // from end to start
        ranges.sort_by_key(|p| -(p.0.start as i32));

        let mut out = original.to_string();

        for (range, text) in ranges {
            out.replace_range(range, text);
        }

        out
    }

    /// Offers `filler` to this content to replace holes, if they exist.
    pub fn provide<T: Slot<DynamicContent>, R: AsRef<T>>(&mut self, filler: &R) {
        for hole in <T as Slot<DynamicContent>>::holes(self) {
            if let Some(rendered) = hole.anchor.accept(filler.as_ref()) {
                hole.replace_with = rendered;
            }
        }
    }

    pub fn hide<T: Slot<DynamicContent>, R: AsRef<T>>(&mut self, filler: &R) {
        for hole in <T as Slot<DynamicContent>>::holes(self) {
            if hole.anchor.accept(filler.as_ref()).is_some() {
                hole.hidden = true;
            }
        }
    }

    /// Adds replaceable value that can be replaced by providing new value of type `A`.
    /// Until this value is received, `placeholder` will be shown.
    pub fn add<A, S>(&mut self, range: Range<usize>, placeholder: String, anchor: A)
    where
        A: Anchor<S> + 'static + Send + Sync,
        S: Slot<DynamicContent>,
    {
        if let Some(r) = &anchor.reference() {
            self.references.push(r.clone())
        }

        let hole = Hole {
            range,
            replace_with: placeholder,
            hidden: false,
            anchor: Arc::new(anchor),
        };
        <S as Slot<DynamicContent>>::holes(self).push(hole);
    }

    /// Adds non-replaceable value into content.
    pub fn add_fixed(&mut self, range: Range<usize>, replace_with: String) {
        self.other.push(Hole {
            range,
            replace_with,
            hidden: false,
            anchor: Arc::new(Void),
        });
    }

    /// Answers the question whether this dynamic content references given argument.
    pub fn has_reference<T: ToReference>(&self, t: T) -> bool {
        self.references.contains(&t.to_reference())
    }

    /// Returns all references of this dynamic content.
    pub fn references(&self) -> &[Reference] {
        self.references.as_ref()
    }
}

impl Debug for DynamicContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicContent")
            .field("profiles", &format!("{} holes", self.profiles.len()))
            .field("events", &format!("{} holes", self.events.len()))
            .field("other", &format!("{} holes", self.other.len()))
            .field("references", &self.references)
            .finish()
    }
}

/// Marker of a portion of original content which can be overwritten by
/// a new content generated by `anchor`, which can use some object of
/// type `WaitingFor`.

// Because each hole may be held by multiple note views (e. g. in multiple
// lanes), it has to be shareable across threads, therefore all the markers.
// Theoretically, range could be also shared but it is so small that perhaps
// it does not make sense.
#[derive(Clone)]
pub struct Hole<WaitingFor>
where
    WaitingFor: ?Sized,
{
    /// Byte range in the original content.
    range: Range<usize>,

    /// Text the hole will be filled with.
    replace_with: String,

    /// Whether the hole is supposed to be hidden.
    hidden: bool,

    /// Anchor generating replacement text of this hole from
    /// some object of type `WaitingFor`.
    // Hole is processed in different threads, so this has
    // to be able to cross thread boundaries.
    anchor: Arc<dyn Anchor<WaitingFor> + Send + Sync>,
}

/// Trait for types that can be a slot in content of type `C`, meaning
/// that we can send values of this type into `C` and it will know what
/// to do with them.
pub trait Slot<C> {
    /// Gives mutabale access to holes in the content `C`.
    fn holes(content: &mut C) -> &mut Vec<Hole<Self>>;
}

/// Trait for types can fill holes in content and are waiting for
/// `What` to produce new text of the hole.
pub trait Anchor<What> {
    fn accept(&self, what: &What) -> Option<String>;

    /// Speciifies to what other object this anchor references,
    /// if anything at all.
    fn reference(&self) -> Option<Reference> {
        None
    }
}

impl Anchor<Void> for Void {
    fn accept(&self, _what: &Void) -> Option<String> {
        None
    }
}

impl Anchor<Persona> for XOnlyPublicKey {
    fn accept(&self, what: &Persona) -> Option<String> {
        if *self == what.pubkey {
            Some(format!(
                r#"<a href="nostr:{}">@{}</a>"#,
                what.pubkey.to_bech32().unwrap(),
                what.display_name
                    .as_ref()
                    .or(what.name.as_ref())
                    .unwrap_or(&"?".to_string())
            ))
        } else {
            None
        }
    }

    fn reference(&self) -> Option<Reference> {
        Some(Reference::Profile(*self, None))
    }
}

impl Anchor<Persona> for Profile {
    fn accept(&self, what: &Persona) -> Option<String> {
        if self.public_key == what.pubkey {
            Some(format!(
                r#"<a href="nostr:{}">@{}</a>"#,
                what.pubkey.to_bech32().unwrap(),
                what.display_name
                    .as_ref()
                    .or(what.name.as_ref())
                    .unwrap_or(&"?".to_string())
            ))
        } else {
            None
        }
    }

    fn reference(&self) -> Option<Reference> {
        Some(Reference::Profile(
            self.public_key,
            Some(self.relays.clone()),
        ))
    }
}

impl Anchor<Event> for Nip19Event {
    fn accept(&self, what: &Event) -> Option<String> {
        if self.event_id == what.id {
            Some(format!("{what:?}"))
        } else {
            None
        }
    }

    fn reference(&self) -> Option<Reference> {
        Some(Reference::Event(self.event_id))
    }
}

impl Anchor<Event> for (Kind, EventId) {
    fn accept(&self, what: &Event) -> Option<String> {
        if self.0 == what.kind && self.1 == what.id {
            let nip19 = what.id.to_bech32().unwrap();
            Some(format!(
                r#"<a href="nostr:{}">{}…</a>"#,
                nip19,
                &nip19[..24]
            ))
        } else {
            None
        }
    }

    fn reference(&self) -> Option<Reference> {
        Some(Reference::Event(self.1))
    }
}

impl Anchor<Preview> for reqwest::Url {
    fn accept(&self, what: &Preview) -> Option<String> {
        if self == what.url() {
            what.title().map(|s| s.to_string()).or_else(|| {
                let safe = html_escape::encode_text(what.url().as_str());
                Some(format!(r#"<a href="{safe}" title="{safe}">{safe}</a>"#))
            })
        } else {
            None
        }
    }

    fn reference(&self) -> Option<Reference> {
        Some(Reference::Url(self.clone()))
    }
}

impl Slot<DynamicContent> for Persona {
    fn holes(content: &mut DynamicContent) -> &mut Vec<Hole<Self>> {
        &mut content.profiles
    }
}

impl Slot<DynamicContent> for Event {
    fn holes(content: &mut DynamicContent) -> &mut Vec<Hole<Self>> {
        &mut content.events
    }
}

impl Slot<DynamicContent> for Preview {
    fn holes(content: &mut DynamicContent) -> &mut Vec<Hole<Self>> {
        &mut content.urls
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Reference {
    Event(EventId),
    Profile(XOnlyPublicKey, Option<Vec<String>>),
    Url(reqwest::Url),
}

pub trait ToReference {
    fn to_reference(&self) -> Reference;
}

impl ToReference for EventId {
    fn to_reference(&self) -> Reference {
        Reference::Event(*self)
    }
}

impl ToReference for reqwest::Url {
    fn to_reference(&self) -> Reference {
        Reference::Url(self.clone())
    }
}

#[allow(non_upper_case_globals)]
#[cfg(test)]
mod tests {
    use nostr_sdk::prelude::*;
    use nostr_sdk::secp256k1::SecretKey;

    use crate::nostr::content::Reference;
    use crate::nostr::parse::parse_content;

    lazy_static::lazy_static! {
        static ref keys: Keys = Keys::new(SecretKey::from_hashed_data::<sha256::Hash>(
            "test".as_bytes(),
        ));

        static ref event1: Event = EventBuilder::new_text_note("Hello.", &[])
            .to_event(&keys)
            .unwrap();

        static ref event2: Event = EventBuilder::new_text_note(format!("Look: {}", event1.id.to_bech32().unwrap()), &[])
            .to_event(&keys)
            .unwrap();
    }

    #[test]
    fn content_parsed_ok() {
        let content = parse_content(&event2);

        assert!(content.profiles.is_empty());
        assert!(content.other.is_empty());

        assert!(content.events.len() == 1);
        assert!(content.references.len() == 1);

        assert!(content.references.contains(&Reference::Event(event1.id)));

        assert!(content.has_reference(event1.id));
    }

    #[test]
    fn content_augmented_ok() {
        let mut content = parse_content(&event2);

        assert!(content
            .augment(&event2.content)
            .contains(&event1.id.to_bech32().unwrap()));

        content.provide(&Box::new(event1.clone()));

        assert!(content
            .augment(&event2.content)
            .contains(&event1.id.to_bech32().unwrap()));

        content.hide(&Box::new(event1.clone()));

        assert_eq!(content.augment(&event2.content), "Look: ");
    }
}
