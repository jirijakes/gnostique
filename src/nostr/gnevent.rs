use std::sync::Arc;

use nostr_sdk::Event;

use super::Persona;

/// An event with its author. If the profile of the author is known at the time
/// of creation it will be included, otherwise only pubkey will be there.
#[derive(Clone, Debug)]
pub struct GnEvent(Arc<Event>, Arc<Persona>);

impl GnEvent {
    /// Creats new `GnEvent` from event and, if available, author's profile.
    pub fn new(event: Event, author: Option<Persona>) -> GnEvent {
        let author = author.unwrap_or(Persona::new(event.pubkey));
        GnEvent(Arc::new(event), Arc::new(author))
    }

    pub fn underlying(self) -> (Arc<Event>, Arc<Persona>) {
        (self.0, self.1)
    }

    /// Returns `Event` associated with this `GnEvent`.
    pub fn event(&self) -> &Event {
        &self.0
    }

    /// Returns this `GnEvent`'s author.
    pub fn author(&self) -> &Persona {
        &self.1
    }
}
