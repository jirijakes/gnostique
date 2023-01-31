mod app;
mod nostr;
mod ui;
mod win;

use std::sync::Arc;

use directories::ProjectDirs;
use nostr::Persona;
use nostr_sdk::prelude::{Event, Metadata, XOnlyPublicKey};
use nostr_sdk::Client;
use relm4::*;
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
