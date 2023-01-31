mod app;
mod lane;
mod nostr;
mod ui;
mod win;

use std::sync::Arc;

use directories::ProjectDirs;
use nostr::Persona;
use nostr_sdk::{
    prelude::{Event, Metadata, XOnlyPublicKey},
    Client,
};
use relm4::*;
use sqlx::{query, SqlitePool};

#[derive(Debug)]
pub struct Gnostique {
    pool: Arc<SqlitePool>,
    dirs: ProjectDirs,
    client: Client,
}

impl Gnostique {

    /// Attempts to obtain [`Person`] from database for a given `pubkey`.
    pub async fn get_persona(&self, pubkey: XOnlyPublicKey) -> Option<Persona> {
        let pubkey_vec = pubkey.serialize().to_vec();
        query!(
            r#"
SELECT event, (unixepoch('now') - unixepoch(nip05_verified)) / 3600 AS "nip05_hours: u16"
FROM metadata
WHERE author = ?
"#,
            pubkey_vec
        )
        .fetch_optional(self.pool.as_ref())
        .await
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
