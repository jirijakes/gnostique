use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use nostr_sdk::prelude::*;
use reqwest::Url;
use tokio::sync::Mutex;
use tracing::{debug, info};

#[derive(Clone)]
pub struct Demand(Arc<DemandInner>);

#[derive(Clone)]
struct DemandInner {
    client: Client,
    notes: Arc<Mutex<HashMap<EventId, Instant>>>,
    metadata: Arc<Mutex<HashMap<XOnlyPublicKey, Instant>>>,
}

impl Demand {
    pub fn new(client: Client) -> Demand {
        Demand(Arc::new(DemandInner {
            client,
            notes: Default::default(),
            metadata: Default::default(),
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
                        None,
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
                        r.req_events_of(sub, None, FilterOptions::ExitOnEOSE);
                    }
                } else {
                    self.0.client.req_events_of(sub, None).await;
                }
            }
        };
    }
}
