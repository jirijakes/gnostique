use nostr_sdk::{relay::ActiveSubscription, secp256k1::XOnlyPublicKey, Filter};

#[derive(Debug, Clone)]
pub enum Subscription {
    Hashtag(String),
    Profile(XOnlyPublicKey),
    // And(Box<Subscriptions>, Box<Subscriptions>),
    Or(Box<Subscription>, Box<Subscription>),
}

impl Subscription {
    /// Creates new hashtag subscription from given `tag`.
    pub fn hashtag<S: Into<String>>(tag: S) -> Subscription {
        Subscription::Hashtag(tag.into())
    }

    pub fn profile(pubkey: XOnlyPublicKey) -> Subscription {
        Subscription::Profile(pubkey)
    }

    /// Collects all hashtags from this subscription.
    // TODO: HashSet?
    pub fn hashtags(&self) -> Vec<&str> {
        let mut tags: Vec<&str> = vec![];

        match self {
            Subscription::Hashtag(t) => tags.push(t),
            Subscription::Or(s1, s2) => {
                tags.append(&mut s1.hashtags());
                tags.append(&mut s2.hashtags());
            }
            Subscription::Profile(_) => {}
        }

        tags
    }

    /// Collects all pubkeys from this subscription.
    pub fn pubkeys(&self) -> Vec<XOnlyPublicKey> {
        let mut pubkeys: Vec<XOnlyPublicKey> = vec![];

        match self {
            Subscription::Profile(p) => pubkeys.push(*p),
            Subscription::Or(s1, s2) => {
                pubkeys.append(&mut s1.pubkeys());
                pubkeys.append(&mut s2.pubkeys());
            }
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
            Subscription::Profile(p) => format!("@{p}"),
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
        Subscription::Profile(p) => {
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
