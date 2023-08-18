use std::collections::HashSet;

use nostr_sdk::prelude::{Filter, Url, XOnlyPublicKey};
use nostr_sdk::relay::ActiveSubscription;

#[derive(Debug, Clone)]
pub enum Subscription {
    Hashtag(String),
    Profile(XOnlyPublicKey, Vec<Url>),
    // And(Box<Subscriptions>, Box<Subscriptions>),
    Or(Box<Subscription>, Box<Subscription>),
}

impl Subscription {
    /// Creates new hashtag subscription from given `tag`.
    pub fn hashtag<S: Into<String>>(tag: S) -> Subscription {
        Subscription::Hashtag(tag.into())
    }

    pub fn profile(pubkey: XOnlyPublicKey, relays: Vec<Url>) -> Subscription {
        Subscription::Profile(pubkey, relays)
    }

    /// Collects all hashtags from this subscription.
    pub fn hashtags(&self) -> HashSet<&str> {
        let mut tags: HashSet<&str> = Default::default();

        match self {
            Subscription::Hashtag(t) => {
                tags.insert(t);
            }
            Subscription::Or(s1, s2) => s1.hashtags().union(&s2.hashtags()).for_each(|t| {
                tags.insert(t);
            }),
            Subscription::Profile(..) => {}
        }

        tags
    }

    /// Collects all pubkeys from this subscription.
    pub fn pubkeys(&self) -> HashSet<XOnlyPublicKey> {
        let mut pubkeys: HashSet<XOnlyPublicKey> = Default::default();

        match self {
            Subscription::Profile(p, _) => {
                pubkeys.insert(*p);
            }
            Subscription::Or(s1, s2) => s1.pubkeys().union(&s2.pubkeys()).for_each(|p| {
                pubkeys.insert(*p);
            }),

            Subscription::Hashtag(_) => {}
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

    pub fn to_filter(&self) -> Filter {
        let mut filter = Filter::new();
        to_filter(self, &mut filter);
        filter
    }

    pub fn to_string(&self) -> String {
        match self {
            Subscription::Hashtag(t) => format!("#{t}"),
            // Subscriptions::And(s1, s2) => format!("{} & {}", s1.to_string(), s2.to_string()),
            Subscription::Or(s1, s2) => format!("{} + {}", s1.to_string(), s2.to_string()),
            Subscription::Profile(p, _) => format!("@{p}"),
        }
    }
}

fn to_filter(subscriptions: &Subscription, filter: &mut Filter) {
    match subscriptions {
        Subscription::Hashtag(t) => {
            let ts = filter.hashtags.get_or_insert(vec![]);
            ts.push(t.to_string());
            ts.sort(); // make hashtags distinct
            ts.dedup(); // make hashtags distinct
        }
        Subscription::Profile(p, _) => {
            let ps = filter.pubkeys.get_or_insert(vec![]);
            ps.push(*p);
            ps.sort();
            ps.dedup();
        }
        Subscription::Or(s1, s2) => {
            to_filter(s1, filter);
            to_filter(s2, filter);
        }
    };
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

        let f = s.to_filter();

        assert_eq!(
            f,
            Filter::new().hashtags(vec!["one".to_string(), "two".to_string()])
        );
    }
}
