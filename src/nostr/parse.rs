use lazy_static::lazy_static;
use linkify::*;
use nostr_sdk::prelude::*;
use regex::Regex;

use super::content::DynamicContent;

lazy_static! {
    pub(super) static ref NIP21: Regex = Regex::new(
        "(nostr:)?(?P<nip19>(?P<type>nprofile|nevent|nrelay|naddr|npub|note)1[qpzry9x8gf2tvdw0s3jn54khce6mua7l]+)",
    ).unwrap();

    pub(super) static ref TAG: Regex = Regex::new("(^|\\s+)(?P<tag>#[a-zA-Z0-9]+)").unwrap();

    pub(super) static ref MENTION: Regex = Regex::new("#\\[(?P<idx>\\d+)\\]").unwrap();
}

pub fn parse_content(event: &Event) -> DynamicContent {
    let mut dcontent = DynamicContent::new();

    // About trimming of content.
    //
    // There used to be `.trim()` here but it was removed because trimming was not
    // used consistently across all the places. For example, while here during parsing
    // the content was trimmed and ranges and substitutions were created for it,
    // later during augmentation the substitution failed because there was no trimming
    // during the augmentation (and ranges pointed to different locations, which could
    // be in the middle of Unicode character).
    //
    // So the trimming is done only at the presentation time.
    let message = &html_escape::encode_text(&event.content);

    NIP21.captures_iter(message).for_each(|c| {
        let nip19 = c.name("nip19").unwrap().as_str();
        let range = c.get(0).unwrap().range();

        match c.name("type").map(|m| m.as_str()) {
            Some("nprofile") => {
                let what = Profile::from_bech32(nip19).unwrap();
                let key = what.public_key.to_bech32().unwrap();
                let with = format!(r#"<a href="nostr:{}">@{}…</a>"#, nip19, &key[..16]);
                dcontent.add(range, with, what);
            }
            Some("npub") => match XOnlyPublicKey::from_bech32(nip19) {
                Ok(key) => {
                    let with = format!(
                        r#"<a href="nostr:{}">@{}…</a>"#,
                        nip19,
                        &key.to_bech32().unwrap()[0..16]
                    );
                    dcontent.add(range, with, key);
                }
                Err(err) => {
                    tracing::error!("Failed parse {} because {:?}", nip19, err);
                }
            },
            Some("nevent") => {
                let what = Nip19Event::from_bech32(nip19).unwrap();
                let with = format!(r#"<a href="nostr:{}">{}…</a>"#, nip19, &nip19[..24]);
                dcontent.add(range, with, what);
            }
            Some("note") => {
                let what = EventId::from_bech32(nip19).unwrap();
                let with = format!(r#"<a href="nostr:{}">{}…</a>"#, nip19, &nip19[..24]);
                dcontent.add(range, with, (Kind::TextNote, what));
            }
            _ => (),
        }
    });

    TAG.captures_iter(message).for_each(|c| {
        if let Some(m) = c.name("tag") {
            let tag = m.as_str().trim_start_matches('#');
            dcontent.add_fixed(
                m.range(),
                format!(r#"<a href="gnostique:search?tag={tag}">#{tag}</a>"#),
            );
        }
    });

    MENTION.captures_iter(message).for_each(|c| {
        let range = c.get(0).unwrap().range();
        let idx: usize = c.name("idx").unwrap().as_str().parse().unwrap();
        match event.tags.get(idx) {
            Some(Tag::Event(id, _, _)) => {
                let nip19 = id.to_bech32().unwrap();
                let with = format!(r#"<a href="nostr:{}">{}…</a>"#, nip19, &nip19[..24]);
                match Nip19Event::from_bech32(&nip19) {
                    Err(e) => {
                        tracing::error!("Failed parse {}: {:?}", nip19, e);
                    }
                    Ok(what) => {
                        dcontent.add(range, with, what);
                    }
                }
            }
            Some(Tag::PubKey(key, _)) => {
                let nip19 = key.to_bech32().unwrap();
                let with = format!(
                    r#"<a href="nostr:{nip19}">@{}…</a>"#,
                    &key.to_bech32().unwrap()[0..16]
                );
                dcontent.add(range, with, *key);
            }
            _ => {}
        };
    });

    LinkFinder::new().spans(message).for_each(|span| {
        if let Some(LinkKind::Url) = span.kind() {
            let str = span.as_str();
            let safe = html_escape::encode_text(span.as_str());
            if let Ok(url) = reqwest::Url::parse(str) {
                dcontent.add(
                    span.start()..span.end(),
                    format!(r#"<a href="{safe}" title="{safe}">{safe}</a>"#),
                    url,
                );
            } else {
                tracing::error!("{:?}", Url::parse(str));
            }
        }
    });

    dcontent
}

#[cfg(test)]
mod tests {
    use super::TAG;

    #[test]
    fn parse_tags() {
        let c = TAG.captures_iter("#nostr.").collect::<Vec<_>>();
        assert!(!c.is_empty());
        c.iter().for_each(|c| {
            assert_eq!(c.name("tag").map(|m| m.as_str()), Some("nostr"));
        });

        let c = TAG.captures_iter("this is #nostr").collect::<Vec<_>>();
        assert!(!c.is_empty());
        c.iter().for_each(|c| {
            assert_eq!(c.name("tag").map(|m| m.as_str()), Some("nostr"));
        });

        let c = TAG.captures_iter("link#nostr").collect::<Vec<_>>();
        assert!(c.is_empty());
    }
}
