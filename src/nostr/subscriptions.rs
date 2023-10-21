use std::collections::HashSet;

use nostr_sdk::prelude::{Filter, Tag, Url, XOnlyPublicKey};
use nostr_sdk::relay::ActiveSubscription;
use nostr_sdk::{Event, EventId, Timestamp};

use super::EventExt;

#[derive(Debug, Clone)]
pub enum Subscription {
    Sink,
    Hashtag(String),
    Profile(XOnlyPublicKey, Vec<Url>),
    Id(EventId),
    Event(EventId),
    // And(Box<Subscriptions>, Box<Subscriptions>),
    Or(Box<Subscription>, Box<Subscription>),
}

impl Subscription {
    /// Creates new hashtag subscription from given `tag`.
    pub fn hashtag<S: Into<String>>(tag: S) -> Subscription {
        Subscription::Hashtag(tag.into())
    }

    /// Creates new subscription for a pubkey.
    pub fn profile(pubkey: XOnlyPublicKey, relays: Vec<Url>) -> Subscription {
        Subscription::Profile(pubkey, relays)
    }

    /// Creates new subscription for a thread containing the event
    /// itself and all events referencing it.
    pub fn thread(event: EventId) -> Subscription {
        Subscription::Or(
            Box::new(Subscription::Event(event)),
            Box::new(Subscription::Id(event)),
        )
    }

    /// Determines whether this subscription is exclusively subscribed to a single profile.
    pub fn is_a_profile(&self) -> bool {
        matches!(self, Subscription::Profile(..))
    }

    /// Collects all events from this subscription.
    pub fn events(&self) -> HashSet<EventId> {
        let mut ids: HashSet<EventId> = Default::default();

        match self {
            Subscription::Sink => {}
            Subscription::Hashtag(..) => {}
            Subscription::Or(s1, s2) => s1.events().union(&s2.events()).for_each(|t| {
                ids.insert(*t);
            }),
            Subscription::Profile(..) => {}
            Subscription::Id(..) => {}
            Subscription::Event(id) => {
                ids.insert(*id);
            }
        }

        ids
    }

    /// Collects all events from this subscription.
    pub fn ids(&self) -> HashSet<EventId> {
        let mut ids: HashSet<EventId> = Default::default();

        match self {
            Subscription::Sink => {}
            Subscription::Hashtag(..) => {}
            Subscription::Or(s1, s2) => s1.events().union(&s2.events()).for_each(|t| {
                ids.insert(*t);
            }),
            Subscription::Profile(..) => {}
            Subscription::Event(..) => {}
            Subscription::Id(id) => {
                ids.insert(*id);
            }
        }

        ids
    }

    /// Collects all hashtags from this subscription.
    pub fn hashtags(&self) -> HashSet<&str> {
        let mut tags: HashSet<&str> = Default::default();

        match self {
            Subscription::Sink => {}
            Subscription::Hashtag(t) => {
                tags.insert(t);
            }
            Subscription::Or(s1, s2) => s1.hashtags().union(&s2.hashtags()).for_each(|t| {
                tags.insert(t);
            }),
            Subscription::Profile(..) => {}
            Subscription::Event(..) => {}
            Subscription::Id(..) => {}
        }

        tags
    }

    /// Collects all pubkeys from this subscription.
    pub fn pubkeys(&self) -> HashSet<XOnlyPublicKey> {
        let mut pubkeys: HashSet<XOnlyPublicKey> = Default::default();

        match self {
            Subscription::Sink => {}
            Subscription::Profile(p, _) => {
                pubkeys.insert(*p);
            }
            Subscription::Or(s1, s2) => s1.pubkeys().union(&s2.pubkeys()).for_each(|p| {
                pubkeys.insert(*p);
            }),

            Subscription::Hashtag(_) => {}
            Subscription::Event(..) => {}
            Subscription::Id(..) => {}
        }

        pubkeys
    }

    pub fn from_sdk(subscription: &ActiveSubscription) -> Option<Subscription> {
        Subscription::from_filters(&subscription.filters())
    }

    pub fn from_filters(filters: &[Filter]) -> Option<Subscription> {
        todo!()
    }

    pub fn add(self, other: Subscription) -> Subscription {
        Subscription::Or(Box::new(self), Box::new(other))
    }

    // TODO: After introducing Subscription::Add, this has to be modified.
    // TODO: At some point, since Timestamp Now has to be removed.
    pub fn to_filters(&self) -> Vec<Filter> {
        let mut filters = Vec::new();

        let hashtags = self.hashtags().into_iter().collect::<Vec<_>>();
        if !hashtags.is_empty() {
            filters.push(Filter::new().hashtags(hashtags).since(Timestamp::now()));
        }

        let events = self.events().into_iter().collect::<Vec<_>>();
        if !events.is_empty() {
            filters.push(Filter::new().events(events));
        }

        let ids = self.ids().into_iter().collect::<Vec<_>>();
        if !ids.is_empty() {
            filters.push(Filter::new().ids(ids));
        }

        let pubkeys = self.pubkeys().into_iter().collect::<Vec<_>>();
        if !pubkeys.is_empty() {
            filters.push(Filter::new().pubkeys(pubkeys).since(Timestamp::now()));
        }

        // TODO: When Sink lane is removed, this can be removed, too.
        filters.push(Filter::new().since(Timestamp::now()));

        filters
    }

    pub fn to_string(&self) -> String {
        match self {
            Subscription::Sink => "Sink".to_string(),
            Subscription::Hashtag(t) => format!("#{t}"),
            // Subscriptions::And(s1, s2) => format!("{} & {}", s1.to_string(), s2.to_string()),
            Subscription::Or(s1, s2) => format!("{} + {}", s1.to_string(), s2.to_string()),
            Subscription::Profile(p, _) => format!("@{p}"),
            Subscription::Event(event) => event.to_string(),
            Subscription::Id(event) => event.to_string(),
        }
    }

    /// Determines whether the incoming `event` is going to be placed in this lane.
    /// Gradually, it will cover all cases and at the end will replace lane kind.
    pub fn accepts(&self, event: &Event) -> bool {
        let tags = self
            .hashtags()
            .iter()
            .map(|t| t.to_lowercase())
            .collect::<HashSet<_>>();

        // TODO: could also consider content of the text note, not only event.tags.
        let accepts_tags = event
            .tags
            .iter()
            .any(|t| matches!(t, Tag::Hashtag(h) if tags.contains(h.to_lowercase().as_str())));

        let accept_pubkeys = self.pubkeys().contains(&event.pubkey);

        let accept_event_ids = self.events().iter().any(|id| {
            event.id == *id
                || event.replies_to() == Some(*id)
                || matches!(event.thread_root(), Some((i, _)) if i == *id)
        });

        matches!(self, Subscription::Sink) || accepts_tags || accept_pubkeys || accept_event_ids
    }
}

#[cfg(test)]
mod tests {
    use nostr_sdk::Filter;

    use super::Subscription;

    #[test]
    fn hashtags_to_filter() {
        let s = Subscription::Or(
            Box::new(Subscription::Or(
                Box::new(Subscription::Hashtag("one".to_string())),
                Box::new(Subscription::Hashtag("two".to_string())),
            )),
            Box::new(Subscription::Hashtag("one".to_string())),
        );

        let f = s.to_filters();

        assert_eq!(
            f,
            Filter::new().hashtags(vec!["one".to_string(), "two".to_string()])
        );
    }
}
