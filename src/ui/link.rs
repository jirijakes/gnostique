use std::sync::Arc;

use nostr_sdk::prelude::{Url, XOnlyPublicKey};

use crate::nostr::Persona;

/// Reference to anything that can be requested by user. Typically
/// used for opening something (e. g. new lane) after clicking on
/// something.
///
/// - gnostique:search?tag=TAG
/// - gnostique:search?pubkey=PUBKEY&relay=RELAY1&relay=RELAY2
// TODO: Make this comment documenting again.
#[derive(Debug, Clone)]
pub enum InternalLink {
    Tag(String),
    Profile(Arc<Persona>, Vec<Url>),
}

impl InternalLink {
    /// Interprets given URI as internal link (of form `gnostique:search?…`),
    /// returns None if the URI's format is not valid.
    pub fn from_url(uri: &Url) -> Option<InternalLink> {
        if uri.scheme() != "gnostique" && uri.path() != "search" {
            None
        } else {
            let params = uri.query_pairs().collect::<Vec<_>>();

            params.iter().find_map(|(k, v)| match k.as_ref() {
                "pubkey" => v.parse().ok().map(|pubkey| {
                    let relays = params
                        .iter()
                        .filter_map(|(k, v)| if k == "relay" { v.parse().ok() } else { None })
                        .collect();
                    InternalLink::Profile(Arc::new(Persona::new(pubkey)), relays)
                }),
                "tag" => Some(InternalLink::Tag(v.clone().into_owned())),
                _ => None,
            })
        }
    }

    pub fn from_url_str(s: &str) -> Option<InternalLink> {
        s.parse().ok().and_then(|u| Self::from_url(&u))
    }

    pub fn pubkey(pubkey: XOnlyPublicKey, relays: Vec<Url>) -> InternalLink {
        Self::profile(Arc::new(Persona::new(pubkey)), relays)
    }

    pub fn profile(persona: Arc<Persona>, relays: Vec<Url>) -> InternalLink {
        InternalLink::Profile(persona, relays)
    }
}
