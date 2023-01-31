mod app;
mod nostr;
mod ui;
mod win;

use std::sync::Arc;

use directories::ProjectDirs;
use nostr::Persona;
use nostr_sdk::prelude::{Event, EventId, Metadata, XOnlyPublicKey};
use nostr_sdk::Client;
use relm4::*;
use reqwest::Url;
use sqlx::{query, SqlitePool};

#[derive(Clone)]
pub struct Gnostique(Arc<GnostiqueInner>);

struct GnostiqueInner {
    pool: SqlitePool,
    dirs: ProjectDirs,
    client: Client,
}

impl Gnostique {
    pub fn new(pool: SqlitePool, dirs: ProjectDirs, client: Client) -> Gnostique {
        Gnostique(Arc::new(GnostiqueInner { pool, dirs, client }))
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.0.pool
    }

    pub fn client(&self) -> &Client {
        &self.0.client
    }

    pub fn dirs(&self) -> &ProjectDirs {
        &self.0.dirs
    }

    /// Stores event and relay from which it arrives into database,
    /// does nothing when already exist.
    pub async fn store_event(&self, relay: &Url, event: &Event) {
        let pool = self.0.pool.clone();
        let id = event.id.as_bytes().to_vec();
        let json = serde_json::to_string(event).unwrap();

        relm4::spawn(async move {
            query!("INSERT INTO textnotes (id, event) VALUES (?, ?)", id, json)
                .execute(&pool)
                .await
        })
        .await
        .expect("Join handler had a problem")
        .unwrap();

        let pool = self.0.pool.clone();
        let id = event.id.as_bytes().to_vec();
        let relay_str = relay.to_string();
        relm4::spawn(async move {
            query!(
                "INSERT INTO textnotes_relays (textnote, relay) VALUES (?, ?)",
                id,
                relay_str
            )
            .execute(&pool)
            .await
        })
        .await
        .expect("Join handler had a problem")
        .unwrap();
    }

    pub async fn textnote_relays(&self, event_id: EventId) -> Vec<Url> {
        let pool = self.0.pool.clone();

        relm4::spawn(async move {
            let id: &[u8] = event_id.as_bytes();

            query!(
                r#"
SELECT url FROM relays
WHERE url IN (SELECT relay FROM textnotes_relays WHERE textnote = ?)"#,
                id
            )
            .map(|r| Url::parse(&r.url).unwrap())
            .fetch_all(&pool)
            .await
        })
        .await
        .unwrap()
        .unwrap_or_default()
    }

    /// Attempts to obtain [`Person`] from database for a given `pubkey`, runs
    /// in relm4 executor.
    pub async fn get_persona(&self, pubkey: XOnlyPublicKey) -> Option<Persona> {
        let pool = self.0.pool.clone();

        relm4::spawn(async move {
            let pubkey: &[u8] = &pubkey.serialize();

            query!(
                r#"
SELECT event, (unixepoch('now') - unixepoch(nip05_verified)) / 3600 AS "nip05_hours: u16"
FROM metadata
WHERE author = ?
"#,
                pubkey
            )
            .fetch_optional(&pool)
            .await
        })
        .await
        .expect("Join handler had a problem")
        .ok()
        .flatten()
        .and_then(|record| {
            let mut persona = serde_json::from_str::<Event>(&record.event)
                .and_then(|e| {
                    serde_json::from_str::<Metadata>(&e.content)
                        .map(|m| Persona::from_metadata(pubkey, m))
                })
                .ok();

            if matches!(record.nip05_hours, Some(h) if h < 5) {
                if let Some(ref mut p) = persona {
                    p.nip05_verified = true
                }
            };

            persona
        })
    }
}

fn main() {
    let app = RelmApp::new("com.jirijakes.gnostique");

    let settings = gtk::Settings::default().unwrap();
    settings.set_gtk_application_prefer_dark_theme(true);

    app.run_async::<crate::win::Win>(());
}
