mod app;
mod demand;
mod download;
mod follow;
mod identity;
mod nostr;
mod stream;
mod ui;

use std::fmt::Debug;
use std::sync::Arc;

use demand::Demand;
use directories::ProjectDirs;
use download::Download;
use nostr::Persona;
use nostr_sdk::prelude::{Event, EventId, Metadata, XOnlyPublicKey};
use nostr_sdk::Client;
use relm4::*;
use reqwest::Url;
use sqlx::{query, SqlitePool};

#[derive(Clone)]
pub struct Gnostique(Arc<GnostiqueInner>);

impl Debug for Gnostique {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Gnostique").field(&self.0.dirs).finish()
    }
}

struct GnostiqueInner {
    pool: SqlitePool,
    dirs: ProjectDirs,
    client: Client,
    download: Download,
    demand: Demand,
}

impl Gnostique {
    pub fn new(pool: SqlitePool, dirs: ProjectDirs, client: Client) -> Gnostique {
        Gnostique(Arc::new(GnostiqueInner {
            demand: Demand::new(client.clone()),
            download: Download::new(dirs.clone()),
            dirs,
            client,
            pool,
        }))
    }

    pub fn demand(&self) -> &Demand {
        &self.0.demand
    }

    pub fn download(&self) -> &Download {
        &self.0.download
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
        let id = event.id.as_bytes().to_vec();
        let json = serde_json::to_string(event).unwrap();

        query!("INSERT INTO textnotes (id, event) VALUES (?, ?)", id, json)
            .execute(self.pool())
            .await
            .unwrap();

        let id = event.id.as_bytes().to_vec();
        let relay_str = relay.to_string();

        query!(
            "INSERT INTO textnotes_relays (textnote, relay) VALUES (?, ?)",
            id,
            relay_str
        )
        .execute(self.pool())
        .await
        .unwrap();
    }

    pub async fn textnote_relays(&self, event_id: EventId) -> Vec<Url> {
        let id: &[u8] = event_id.as_bytes();

        query!(
            r#"
SELECT url FROM relays
WHERE url IN (SELECT relay FROM textnotes_relays WHERE textnote = ?)"#,
            id
        )
        .map(|r| Url::parse(&r.url).unwrap())
        .fetch_all(self.pool())
        .await
        .unwrap_or_default()
    }

    /// Attempts to obtain [`Person`] from database for a given `pubkey`, runs
    /// in relm4 executor.
    pub async fn get_persona(&self, pubkey: XOnlyPublicKey) -> Option<Persona> {
        let pubkey_bytes: &[u8] = &pubkey.serialize();

        query!(
            r#"
SELECT event, (unixepoch('now') - unixepoch(nip05_verified)) / 3600 AS "nip05_hours: u16"
FROM metadata
WHERE author = ?
"#,
            pubkey_bytes
        )
        .fetch_optional(self.pool())
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

    // GTK and resources
    gtk::glib::set_application_name("Gnostique");
    gtk::gio::resources_register_include!("resources.gresource").unwrap();
    let provider = gtk::CssProvider::new();
    provider.load_from_resource("/com/jirijakes/gnostique/ui/style.css");
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::StyleContext::add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        )
    };

    let icon_theme = gtk::IconTheme::for_display(&gtk::gdk::Display::default().unwrap());
    icon_theme.add_resource_path("/com/jirijakes/gnostique/icons");

    let settings = gtk::Settings::default().unwrap();
    settings.set_gtk_application_prefer_dark_theme(true);

    app.run::<crate::ui::app::App>(());
}
