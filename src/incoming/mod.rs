mod feedback;

use std::collections::HashSet;
use std::path::PathBuf;

use futures_util::*;
use nostr_sdk::nostr::nips::nip05;
use nostr_sdk::prelude::*;
use nostr_sdk::RelayPoolNotification;
use sqlx::query;
use tokio::sync::mpsc;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

use self::feedback::{deal_with_feedback, Feedback};
use crate::gnostique::Gnostique;
use crate::nostr::content::{DynamicContent, Reference};
use crate::nostr::gnevent::GnEvent;
use crate::nostr::preview::Preview;
use crate::nostr::{EventExt, Persona, Repost, TextNote};

// Note: Clone is required by broadcast::channel.
#[derive(Clone, Debug)]
pub enum Incoming {
    TextNote {
        note: TextNote,
        relays: Vec<Url>,
        avatar: Option<PathBuf>,
        repost: Option<Repost>,
        content: DynamicContent,
        referenced_notes: HashSet<TextNote>,
        referenced_profiles: HashSet<Persona>,
    },
    Reaction {
        event_id: EventId,
        content: String,
    },
    Metadata {
        persona: Persona,
        avatar: Option<PathBuf>,
    },
    Preview(Preview),
}

/// Stream of incoming messages. These are not only Nostr messages but any that can
/// be of interest to consumers (see `Incoming` enum to see what it offers).
pub fn incoming_stream(gnostique: &Gnostique) -> impl Stream<Item = Incoming> + '_ {
    // A feedback from processing functions. If they need something,
    // they can ask by sending a message to `feedback`.
    let (feedback, rx) = mpsc::channel(10);
    tokio::spawn(deal_with_feedback(gnostique.clone(), rx));

    let nostream = BroadcastStream::new(gnostique.client().notifications())
        .filter_map(|r| async {
            if let Ok(RelayPoolNotification::Event(relay, event)) = r {
                Some((relay, event))
            } else {
                // println!("\n{r:?}\n");
                None
            }
        })
        .then(|(relay, event)| async {
            offer_relays(gnostique, &relay, &event).await;
            (relay, event)
        })
        .map(move |(relay, event)| received_event(gnostique, feedback.clone(), relay, event))
        .buffer_unordered(64)
        .filter_map(future::ready);

    let other = BroadcastStream::new(gnostique.external()).filter_map(|r| future::ready(r.ok()));

    {
        use tokio_stream::StreamExt;
        nostream.merge(other)
    }
}

async fn received_event(
    gnostique: &Gnostique,
    feedback: mpsc::Sender<Feedback>,
    relay: Url,
    event: Event,
) -> Option<Incoming> {
    match event.kind {
        Kind::TextNote => Some(received_text_note(gnostique, feedback, relay, event, None).await),
        Kind::Metadata => Some(received_metadata(gnostique, event).await),
        Kind::Reaction => event.reacts_to().map(|to| Incoming::Reaction {
            event_id: to,
            content: event.content,
        }),
        Kind::Repost => {
            if let Ok(inner) = Event::from_json(&event.content) {
                Some(received_text_note(gnostique, feedback, relay, inner, Some(event)).await)
            } else {
                None
            }
        }
        _ => None,
    }
}

async fn received_metadata(gnostique: &Gnostique, event: Event) -> Incoming {
    let pubkey_vec = event.pubkey.serialize().to_vec();
    let json = event.as_json();

    let _ = query!(
        r#"
INSERT INTO metadata (author, event) VALUES (?, ?)
ON CONFLICT (author) DO UPDATE SET event = EXCLUDED.event
"#,
        pubkey_vec,
        json
    )
    .execute(gnostique.pool())
    .await;

    let metadata = event.as_metadata().unwrap();

    let avatar_url = metadata
        .picture
        .as_ref()
        .and_then(|p| reqwest::Url::parse(p).ok());
    let banner_url = metadata
        .banner
        .as_ref()
        .and_then(|p| reqwest::Url::parse(p).ok());

    // If the metadata's picture contains valid URL, download it.
    let avatar = if let Some(ref url) = avatar_url {
        Some(gnostique.download().to_cached_file(url).await)
    } else {
        None
    };

    let verified: bool = if let Some(ref nip05) = metadata.nip05 {
        !nip05.trim().is_empty() && verify_nip05(gnostique, event.pubkey, nip05).await
    } else {
        false
    };

    let p = Persona {
        pubkey: event.pubkey,
        name: metadata.name,
        display_name: metadata.display_name,
        avatar: avatar_url,
        banner: banner_url,
        about: metadata.about,
        nip05: metadata.nip05,
        nip05_preverified: verified,
        metadata_json: json,
    };

    Incoming::Metadata {
        persona: p,
        avatar: avatar.and_then(|d| d.file()),
    }
}

/// Attempts to load persona with `pubkey` from storage and return it.
/// If the persona is unknown, demand is created in the background and
/// `None` is returned immediately.
async fn get_persona_or_demand(
    gnostique: &Gnostique,
    feedback: mpsc::Sender<Feedback>,
    relay: Url,
    pubkey: XOnlyPublicKey,
) -> Option<Persona> {
    let persona = gnostique.get_persona(pubkey).await;

    if persona.is_none() {
        feedback
            .send(Feedback::NeedMetadata { relay, pubkey })
            .await
            .unwrap_or_default();
    }

    persona
}

async fn get_link_preview_or_demand(
    gnostique: &Gnostique,
    feedback: mpsc::Sender<Feedback>,
    url: &reqwest::Url,
) -> Option<Preview> {
    let preview = gnostique.get_link_preview(url).await;

    if preview.is_none() {
        feedback
            .send(Feedback::MakePreview { url: url.clone() })
            .await
            .unwrap_or_default();
    }

    preview
}

/// Attempts to load text note with `event_id` from storage and return it.
/// If the text note is unknown, demand is created in the background and
/// `None` is returned immediately.
async fn get_note_or_demand(
    gnostique: &Gnostique,
    feedback: mpsc::Sender<Feedback>,
    relay: Option<Url>,
    event_id: EventId,
) -> Option<Event> {
    let note = gnostique.get_note(event_id).await;

    if note.is_none() {
        feedback
            .send(Feedback::NeedNote { event_id, relay })
            .await
            .unwrap_or_default();
    }

    note
}

async fn received_text_note(
    gnostique: &Gnostique,
    feedback: mpsc::Sender<Feedback>,
    relay: Url,
    event: Event,
    repost: Option<Event>,
) -> Incoming {
    gnostique.store_event(&relay, &event).await;

    // if let Some((root, root_relay)) = event.thread_root() {
    //     feedback
    //         .send(Feedback::NeedNote {
    //             event_id: root,
    //             relay: root_relay,
    //         })
    //         .await
    //         .unwrap_or_default();
    // };

    let content = event.prepare_content();

    let mut referenced_notes: HashSet<TextNote> = Default::default();
    let mut referenced_profiles: HashSet<Persona> = Default::default();
    let mut referenced_urls: HashSet<&reqwest::Url> = Default::default();

    for r in content.references() {
        match r {
            Reference::Event(id) => {
                // TODO: Beautify
                let note = get_note_or_demand(gnostique, feedback.clone(), None, *id).await;
                if let Some(n) = note {
                    let author =
                        get_persona_or_demand(gnostique, feedback.clone(), relay.clone(), n.pubkey)
                            .await;
                    referenced_notes.insert(TextNote::new(GnEvent::new(n, author)));
                }
            }
            Reference::Profile(pubkey, rs) => {
                let r = rs
                    .clone()
                    .and_then(|rs| rs.first().cloned())
                    .and_then(|r| Url::parse(r.as_str()).ok())
                    .unwrap_or(relay.clone());
                let persona = get_persona_or_demand(gnostique, feedback.clone(), r, *pubkey).await;
                if let Some(p) = persona {
                    referenced_profiles.insert(p);
                }
            }
            Reference::Url(url) => {
                let preview = get_link_preview_or_demand(gnostique, feedback.clone(), url).await;
                dbg!(preview);
                referenced_urls.insert(url);
            }
        }
    }

    let author =
        get_persona_or_demand(gnostique, feedback.clone(), relay.clone(), event.pubkey).await;

    let repost = match repost {
        Some(r) => {
            let author = gnostique.get_persona(r.pubkey).await;
            Some(Repost::new(GnEvent::new(r, author)))
        }
        None => None,
    };

    let avatar = author.as_ref().and_then(|Persona { avatar, .. }| {
        avatar
            .as_ref()
            .and_then(|url| gnostique.download().cached(url))
    });

    let relays = gnostique.textnote_relays(event.id).await;

    let note = TextNote::new(GnEvent::new(event, author));

    Incoming::TextNote {
        note,
        relays,
        avatar,
        repost,
        content,
        referenced_notes,
        referenced_profiles,
    }
}

async fn offer_relays(gnostique: &Gnostique, relay: &Url, event: &Event) {
    offer_relay_url(gnostique, &UncheckedUrl::new(relay.to_string())).await;

    for r in event.collect_relays() {
        offer_relay_url(gnostique, &r).await
    }
}

async fn offer_relay_url(gnostique: &Gnostique, relay: &UncheckedUrl) {
    let url_s = relay.to_string();
    let _ = query!(
        "INSERT INTO relays(url) VALUES (?) ON CONFLICT(url) DO NOTHING",
        url_s
    )
    .execute(gnostique.pool())
    .await;
}

async fn verify_nip05(gnostique: &Gnostique, pubkey: XOnlyPublicKey, nip05: &str) -> bool {
    let pubkey_bytes = pubkey.serialize().to_vec();
    // If the nip05 is already verified and not for too long, just confirm.
    let x = query!(
        r#"
SELECT (unixepoch('now') - unixepoch(nip05_verified)) / 60 / 60 AS "hours?: u32"
FROM metadata WHERE author = ?"#,
        pubkey_bytes
    )
    .fetch_optional(gnostique.pool())
    .await;

    if let Ok(result) = x {
        let x = result.and_then(|r| r.hours);

        match x {
            Some(hours) if hours < 12 => {
                info!("NIP05: {} verified {} hours ago", nip05, hours);
                true
            }
            _ => {
                info!("NIP05: Verifying {}.", nip05);
                // If it's not yet verified or been verified for very long, update.
                if nip05::verify(pubkey, nip05, None).await.is_ok() {
                    let _ = query!(
                        r#"
UPDATE metadata SET nip05_verified = datetime('now')
WHERE author = ?"#,
                        pubkey_bytes
                    )
                    .execute(gnostique.pool())
                    .await;

                    info!("NIP05: {} verified.", nip05);
                    true
                } else {
                    info!("NIP05: {} verification failed.", nip05);
                    false
                }
            }
        }
    } else {
        false
    }
}
