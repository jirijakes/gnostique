use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use nostr_sdk::nostr::nips::nip11;
use nostr_sdk::Client;
use relm4::AsyncComponentSender;
use reqwest::Url;
use sqlx::query;
use tracing::info;

use crate::win::{Msg, Win};
use crate::Gnostique;

/// Obtains Nostr events and forwards them to the provided `sender`.
pub async fn receive_events(_nostr: Client, sender: AsyncComponentSender<Win>) {
    include_str!(
        // "../../resources/b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b.json"
        "../../resources/febbaba219357c6c64adfa2e01789f274aa60e90c289938bfc80dd91facb2899.json"
    )
    .lines()
    .for_each(|l| {
        let ev = nostr_sdk::nostr::event::Event::from_json(l).unwrap();
        let url = "http://example.com".parse().unwrap();
        sender.input(Msg::Event(url, ev));
    });

    // let mut notif = nostr.notifications();
    // while let Ok(nostr_sdk::RelayPoolNotification::Event(relay, event)) = notif.recv().await {
    // sender.input(Msg::Event(relay, event));
    // }
}

/// Regularly, and in the background, obtain information about relays.
pub async fn refresh_relay_information(gnostique: Arc<Gnostique>) {
    let mut int = tokio::time::interval(Duration::from_secs(60));
    loop {
        int.tick().await;

        let client_relays = gnostique.client.relays().await;
        let mut client_relays: HashSet<Url> = client_relays.keys().cloned().collect();

        let old_info = query!(
            r#"
SELECT
  url,
  information IS NULL OR unixepoch('now') - unixepoch(updated) > 60 * 60 AS "old: bool"
FROM relays
"#
        )
        .fetch_all(gnostique.pool.as_ref())
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
            if let Ok(info) = nip11::get_relay_information_document(url.clone(), None).await {
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
                .execute(gnostique.pool.as_ref())
                .await;

                info!("Stored fresh relay information of {}", url);
            }
        }
    }
}
