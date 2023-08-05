use lazy_static::lazy_static;
use linkify::*;
use nostr_sdk::prelude::*;
use regex::Regex;

use super::content::DynamicContent;

pub fn parse_content(event: &Event) -> DynamicContent {
    lazy_static! {
        static ref NIP21: Regex = Regex::new(
            "(?P<nip19>(?P<type>nprofile|nevent|nrelay|naddr|npub|note)1[qpzry9x8gf2tvdw0s3jn54khce6mua7l]+)",
        ).unwrap();

        static ref TAG: Regex = Regex::new("#(?P<tag>[a-zA-Z0-9]+)").unwrap();

        static ref MENTION: Regex = Regex::new("#\\[(?P<idx>\\d+)\\]").unwrap();
    }

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
    let message = &event.content;

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
            Some("npub") => {
                let key = XOnlyPublicKey::from_bech32(nip19).unwrap();
                let with = format!(
                    r#"<a href="nostr:{}">@{}…</a>"#,
                    nip19,
                    &key.to_bech32().unwrap()[0..16]
                );
                dcontent.add(range, with, key);
            }
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
        let tag = c.name("tag").unwrap().as_str();
        let range = c.get(0).unwrap().range();
        dcontent.add_fixed(
            range,
            format!(r#"<a href="gnostique:search?tag={}">#{}</a>"#, tag, tag),
        );
    });

    MENTION.captures_iter(message).for_each(|c| {
        let range = c.get(0).unwrap().range();
        let idx: usize = c.name("idx").unwrap().as_str().parse().unwrap();
        match event.tags.get(idx) {
            Some(Tag::Event(id, _, _)) => {
                let nip19 = id.to_bech32().unwrap();
                let with = format!(r#"<a href="nostr:{}">{}…</a>"#, nip19, &nip19[..24]);
                let what = Nip19Event::from_bech32(nip19).unwrap();
                dcontent.add(range, with, what);
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
            let s = html_escape::encode_text(span.as_str());
            dcontent.add_fixed(
                span.start()..span.end(),
                format!(r#"<a href="{s}" title="{s}">{s}</a>"#),
            );
        }
    });

    dcontent
}
