use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future;
use nostr_sdk::nostr::nips::nip11;
use relm4::AsyncComponentSender;
use reqwest::Url;
use sqlx::query;
use tracing::info;

use crate::gnostique::Gnostique;
use crate::ui::main::{Main, MainInput};

/// Obtains Nostr events and forwards them to the provided `sender`.
pub async fn receive_events(gnostique: Gnostique, sender: AsyncComponentSender<Main>) {
    use futures_util::StreamExt;

    crate::incoming::incoming_stream(&gnostique)
        .for_each(|received| {
            sender.input(MainInput::Incoming(received));
            future::ready(())
        })
        .await;
}

/// Regularly, and in the background, obtain information about relays.
pub async fn refresh_relay_information(gnostique: Arc<Gnostique>) {
    let mut int = tokio::time::interval(Duration::from_secs(60));
    loop {
        int.tick().await;

        let client_relays = gnostique.client().relays().await;
        let mut client_relays: HashSet<Url> = client_relays.keys().cloned().collect();

        let old_info = query!(
            r#"
SELECT
  url,
  information IS NULL OR unixepoch('now') - unixepoch(updated) > 60 * 60 AS "old: bool"
FROM relays
"#
        )
        .fetch_all(gnostique.pool())
        .await;

        let old_info: HashSet<_> = if let Ok(rec) = old_info {
            rec.iter()
                .filter_map(|r| {
                    let url: reqwest::Url = r.url.parse().unwrap();
                    client_relays.remove(&url);

                    if r.old {
                        Some(url)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            HashSet::new()
        };

        for url in old_info.union(&client_relays) {
            if let Ok(info) = nip11::RelayInformationDocument::get(url.clone(), None).await {
                let url_s = url.to_string();
                let info_json = serde_json::to_string(&info).unwrap();
                let _ = query!(
                    r#"
INSERT INTO relays(url, information, updated)
VALUES (?, ?, CURRENT_TIMESTAMP)
ON CONFLICT(url) DO UPDATE SET
  information = EXCLUDED.information,
  updated = EXCLUDED.updated
"#,
                    url_s,
                    info_json
                )
                .execute(gnostique.pool())
                .await;

                info!("Stored fresh relay information of {}", url);
            }
        }
    }
}
