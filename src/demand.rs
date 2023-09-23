use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nostr_sdk::prelude::*;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, info};

use crate::incoming::Incoming;
use crate::nostr::preview::Preview;

#[derive(Clone)]
pub struct Demand(Arc<DemandInner>);

#[derive(Clone)]
struct DemandInner {
    client: Client,
    notes: Arc<Mutex<HashMap<EventId, Instant>>>,
    metadata: Arc<Mutex<HashMap<XOnlyPublicKey, Instant>>>,
    external: broadcast::Sender<Incoming>,
}

impl Demand {
    pub fn new(client: Client, external: broadcast::Sender<Incoming>) -> Demand {
        Demand(Arc::new(DemandInner {
            client,
            notes: Default::default(),
            metadata: Default::default(),
            external,
        }))
    }

    // TODO: Clean up `notes` and `metadata`

    pub async fn metadata(&self, pubkey: XOnlyPublicKey, relay: Vec<Url>) {
        let elapsed = self
            .0
            .metadata
            .lock()
            .await
            .get(&pubkey)
            .map(|i| i.elapsed().as_millis());
        match elapsed {
            Some(el) if el < 5000 => {
                debug!(
                    "Ignoring request for metadata {}, last {el} ms ago.",
                    pubkey
                );
            }
            _ => {
                self.0.metadata.lock().await.insert(pubkey, Instant::now());

                info!("Requesting metadata {}.", pubkey.to_bech32().unwrap());

                let relays = self.0.client.relays().await;
                // TODO: Try more relays.
                let relay = relay.first().expect("Relays should be multiple");
                if let Some(r) = relays.get(relay) {
                    r.req_events_of(
                        vec![Filter::new()
                            .kind(Kind::Metadata)
                            .author(pubkey.to_string())
                            .limit(1)],
                        Duration::from_secs(3),
                        FilterOptions::ExitOnEOSE,
                    );
                }
            }
        };
    }

    pub async fn text_note(&self, event_id: EventId, relay: Option<Url>) {
        let elapsed = self
            .0
            .notes
            .lock()
            .await
            .get(&event_id)
            .map(|i| i.elapsed().as_millis());
        match elapsed {
            Some(el) if el < 5000 => {
                debug!(
                    "Ignoring request for note {}, last {el} ms ago.",
                    event_id.to_hex()
                );
            }
            _ => {
                self.0.notes.lock().await.insert(event_id, Instant::now());

                info!("Requesting note {}.", event_id.to_bech32().unwrap());

                let sub = vec![
                    Filter::new().kind(Kind::TextNote).id(event_id.to_hex()),
                    Filter::new().kind(Kind::TextNote).event(event_id),
                ];

                if let Some(r) = relay {
                    let relays = self.0.client.relays().await;
                    if let Some(r) = relays.get(&r) {
                        r.req_events_of(sub, Duration::from_secs(3), FilterOptions::ExitOnEOSE);
                    }
                } else {
                    self.0.client.req_events_of(sub, None).await;
                }
            }
        };
    }

    pub async fn link_preview(&self, url: &reqwest::Url) {
        info!("Requesting preview for {}", url);
        let preview = Preview::create(url.clone()).await;
        self.0
            .external
            .send(Incoming::Preview(preview))
            .unwrap_or_default();
    }
}
