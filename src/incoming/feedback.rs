use futures_util::*;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{EventId, Url};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::gnostique::Gnostique;

/// Requests requested by processing functions during processing incoming events.
#[derive(Debug)]
pub enum Feedback {
    /// Metadata for `pubkey` are requested from `relay`.
    NeedMetadata { relay: Url, pubkey: XOnlyPublicKey },
    NeedNote {
        event_id: EventId,
        relay: Option<Url>,
    },
}

/// Listens to incoming messages asking for some additional actions or data
/// and processes them.
pub async fn deal_with_feedback(gnostique: Gnostique, rx: mpsc::Receiver<Feedback>) {
    ReceiverStream::new(rx)
        .for_each(|f| async {
            match f {
                Feedback::NeedMetadata { relay, pubkey } => {
                    gnostique.demand().metadata(pubkey, relay).await;
                }
                Feedback::NeedNote { event_id, relay } => {
                    gnostique.demand().text_note(event_id, relay).await;
                }
            }
        })
        .await
}