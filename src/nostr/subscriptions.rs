use nostr_sdk::{relay::ActiveSubscription, Filter};

#[derive(Debug, Clone)]
pub enum Subscription {
    Hashtag(String),
    // And(Box<Subscriptions>, Box<Subscriptions>),
    Or(Box<Subscription>, Box<Subscription>),
}

impl Subscription {
    pub fn hashtag<S: Into<String>>(tag: S) -> Subscription {
        Subscription::Hashtag(tag.into())
    }
    
    pub fn from_sdk(subscription: &ActiveSubscription) -> Option<Subscription> {
        Subscription::from_filters(&subscription.filters())
    }

    pub fn from_filters(filters: &[Filter]) -> Option<Subscription> {
        todo!()
    }

    pub fn add(&self, other: &Subscription) -> Subscription {
        todo!()
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
