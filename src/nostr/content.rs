use std::{fmt::Debug, ops::Range};

use nostr_sdk::prelude::*;

use super::Persona;

struct Void;

#[derive(Default)]
pub struct Content {
    profiles: Vec<Hole<Persona>>,
    events: Vec<Hole<Event>>,
    other: Vec<Hole<Void>>,
}

impl Content {
    /// Augments `original` content with whatever is available at the moment.
    pub fn augment(&self, original: &str) -> String {
        let mut ranges: Vec<(Range<usize>, &String)> = Vec::new();

        for p in &self.profiles {
            ranges.push((p.range.clone(), &p.replace_with));
        }

        for e in &self.events {
            ranges.push((e.range.clone(), &e.replace_with));
        }

        for v in &self.other {
            ranges.push((v.range.clone(), &v.replace_with));
        }

        // from end to start
        ranges.sort_by_key(|p| -(p.0.start as i32));

        let mut out = original.to_string();

        for (range, text) in ranges {
            out.replace_range(range, text);
        }

        out
    }

    pub fn provide<T: Slot<Content>, R: AsRef<T>>(&mut self, filler: &R) {
        for hole in <T as Slot<Content>>::holes(self) {
            if let Some(rendered) = hole.anchor.accept(filler.as_ref()) {
                hole.replace_with = rendered;
            }
        }
    }

    /// Adds replaceable value that can be replaced by providing new value.
    pub fn add<A, S>(&mut self, range: Range<usize>, placeholder: String, anchor: A)
    where
        A: Anchor<S> + 'static,
        S: Slot<Content>,
    {
        let holes = <S as Slot<Content>>::holes(self);
        let hole = Hole {
            range,
            replace_with: placeholder,
            anchor: Box::new(anchor),
        };
        holes.push(hole);
    }

    /// Adds non-replaceable value into content.
    pub fn add_fixed(&mut self, range: Range<usize>, replace_with: String) {
        self.other.push(Hole {
            range,
            replace_with,
            anchor: Box::new(Void),
        });
    }
}

impl Debug for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Content").finish()
    }
}

pub struct Hole<T> {
    pub range: Range<usize>,
    pub replace_with: String,
    pub anchor: Box<dyn Anchor<T>>,
}

/// Trait for types that can be a slot in content of type `C`, meaning
/// that we can send values of this type into `C` and it will know what
/// to do with them.
pub trait Slot<C>
where
    Self: Sized,
{
    /// Gives mutabale access to holes in the content `C`.
    fn holes(content: &mut C) -> &mut Vec<Hole<Self>>;
}

/// Trait for types that are holes in content and are waiting for
/// `What` to be filled by.
pub trait Anchor<What> {
    fn accept(&self, what: &What) -> Option<String>;
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
}

impl Anchor<Event> for Nip19Event {
    fn accept(&self, what: &Event) -> Option<String> {
        if self.event_id == what.id {
            Some(format!("{what:?}"))
        } else {
            None
        }
    }
}

impl Slot<Content> for Persona {
    fn holes(content: &mut Content) -> &mut Vec<Hole<Self>> {
        &mut content.profiles
    }
}

impl Slot<Content> for Event {
    fn holes(content: &mut Content) -> &mut Vec<Hole<Self>> {
        &mut content.events
    }
}
