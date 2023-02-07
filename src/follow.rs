use std::collections::HashSet;

use nostr_sdk::prelude::*;

fn following() -> Vec<XOnlyPublicKey> {
    [
        "npub1xtscya34g58tk0z605fvr788k263gsu6cy9x0mhnm87echrgufzsevkk5s", // jb55
        "npub17u5dneh8qjp43ecfxr6u5e9sjamsmxyuekrg2nlxrrk6nj9rsyrqywt4tp", // lopp
        "npub1ktt8phjnkfmfrsxrgqpztdjuxk3x6psf80xyray0l3c7pyrln49qhkyhz0", // 0xtr1
        "npub1pyp9fqq60689ppds9ec3vghsm7s6s4grfya0y342g2hs3a0y6t0segc0qq", // DylanLeClair_
        "npub1rxysxnjkhrmqd3ey73dp9n5y5yvyzcs64acc9g0k2epcpwwyya4spvhnp8", // BTCsessions
    ]
    .into_iter()
    .map(|s| XOnlyPublicKey::from_bech32(s).unwrap())
    .collect()
}

#[derive(Clone, Debug)]
pub struct Follow {
    following: HashSet<XOnlyPublicKey>,
}

impl Follow {
    pub fn new() -> Follow {
        Follow {
            following: following().into_iter().collect(),
        }
    }

    // TODO: Batch
    pub fn subscriptions(&self) -> SubscriptionFilter {
        SubscriptionFilter::new()
            .kinds(vec![Kind::TextNote, Kind::Repost])
            .authors(self.following.iter().copied().collect())
            .limit(30)
    }

    pub fn follows(&self, pubkey: &XOnlyPublicKey) -> bool {
        self.following.contains(pubkey)
    }
}
