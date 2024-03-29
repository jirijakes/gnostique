use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;

use age::Decryptor;
use directories::ProjectDirs;
use gtk::{gdk, glib};
use nostr_sdk::prelude::{Event, EventId, Metadata, XOnlyPublicKey};
use nostr_sdk::{Client, Filter, Options, Relay, RelayPoolOptions, Timestamp, Url};
use secrecy::SecretString;
use sqlx::{query, SqlitePool};
use tokio::io::AsyncReadExt;
use tokio::sync::broadcast;

use crate::demand::Demand;
use crate::download::Download;
use crate::identity::Identity;
use crate::incoming::Incoming;
use crate::nostr::preview::Preview;
use crate::nostr::{Persona, ReceivedEvent};

/// Gnostique session. In order to use Gnostique, an instance of this
/// has to exist.
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
    // TODO: Should this be Incoming or a new type?
    external: broadcast::Sender<Incoming>,
}

impl Gnostique {
    fn new(pool: SqlitePool, dirs: ProjectDirs, client: Client) -> Gnostique {
        let (external_tx, _) = broadcast::channel(10);
        Gnostique(Arc::new(GnostiqueInner {
            demand: Demand::new(client.clone(), external_tx.clone()),
            download: Download::new(dirs.clone()),
            dirs,
            client,
            pool,
            external: external_tx,
        }))
    }

    pub fn demand(&self) -> &Demand {
        &self.0.demand
    }

    pub fn download(&self) -> &Download {
        &self.0.download
    }

    /// A channel with messages coming from other sources
    /// than Nostr. For Nostr messages, see `client()`.
    pub fn external(&self) -> broadcast::Receiver<Incoming> {
        self.0.external.subscribe()
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.0.pool
    }

    pub fn client(&self) -> &Client {
        &self.0.client
    }

    // pub fn dirs(&self) -> &ProjectDirs {
    //     &self.0.dirs
    // }

    /// Stores event and relay from which it arrives into database,
    /// does nothing when already exist.
    pub async fn store_event(&self, event: &ReceivedEvent) {
        let ReceivedEvent { event, relay } = event;
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

    /// Attempts to obtain text note from database with a given `id`,
    /// runs in relm4 executor.
    pub async fn get_note(&self, id: EventId) -> Option<Event> {
        let id_bytes = id.as_bytes().to_vec();

        query!("SELECT event FROM textnotes WHERE id = ?", id_bytes)
            .fetch_optional(self.pool())
            .await
            .ok()
            .flatten()
            .and_then(|record| serde_json::from_str::<Event>(&record.event).ok())
    }

    // TODO: Consider whether caching previews makes sense.
    pub async fn get_link_preview(&self, url: &reqwest::Url) -> Option<Preview> {
        None
        //         use crate::nostr::preview::PreviewKind;

        //         let url = url.to_string();
        //         query!(
        //             r#"
        // SELECT url, kind AS "kind: PreviewKind", title, description, thumbnail, error, time
        // FROM previews
        // WHERE url = ?
        // "#,
        //             url
        //         )
        //         .fetch_optional(self.pool())
        //         .await
        //         .ok()
        //         .flatten()
        //         .and_then(|record| {
        //             Some(Preview::new(
        //                 Url::parse(&record.url).ok()?,
        //                 record.kind,
        //                 record.title,
        //                 record.description,
        //                 record
        //                     .thumbnail
        //                     .and_then(|bs| gdk::Texture::from_bytes(&glib::Bytes::from(&bs)).ok()),
        //                 record.error,
        //             ))
        //         })
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
                    // p.nip05_verified = true
                }
            };

            persona
        })
    }
}

pub enum LoadError {
    PaswordRequired,
    Config(config::ConfigError),
    ConfigFile(tokio::io::Error),
    IdentityFile(std::io::Error),
    Age(age::DecryptError),
}

/// Creates a gnostique session. If `identity_file` does not exist, a new random
/// identity will be created and saved to file encrypted using `password`.
// TODO: this function should not be creating new identities, a ready identity should be passed.
pub async fn make_gnostique(
    dirs: ProjectDirs,
    pool: SqlitePool,
    identity_file: PathBuf,
    password: SecretString,
) -> Result<Gnostique, String> {
    // Logging, tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // .pretty()
        .compact()
        .with_max_level(tracing::Level::TRACE)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(true)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let identity = if tokio::fs::try_exists(&identity_file).await.unwrap() {
        let ciph = tokio::fs::File::open(identity_file).await;
        let mut buf = Vec::new();
        ciph.unwrap()
            .read_to_end(&mut buf)
            .await
            .map_err(|e| e.to_string())?;

        if let Ok(Decryptor::Passphrase(d)) = Decryptor::new(buf.as_slice()) {
            let rea = d.decrypt(&password, Some(18)).map_err(|e| e.to_string())?;
            serde_json::from_reader(rea).map_err(|e| e.to_string())?
        } else {
            Err("Can't".to_string())?
        }
    } else {
        let new_identity = Identity::new_random("Default identity");

        // TODO: Save

        new_identity
    };

    // Create Nostr client
    let client = Client::new(&identity.nostr_key());

    let gnostique = Gnostique::new(pool, dirs, client);

    gnostique
        .client()
        .add_relays(vec![
            // ("ws://localhost:8080", None),
            // ("wss://eden.nostr.land", None),
            // ("wss://nostr.fmt.wiz.biz", None),
            // ("wss://relay.damus.io", None),
            // ("wss://nostr-pub.wellorder.net", None),
            ("wss://nos.lol", None),
            // ("wss://relay.snort.social", None),
            // ("wss://relay.current.fyi", None),
        ])
        .await
        .unwrap();

    gnostique.client().connect().await;

    for (_, r) in gnostique.client().relays().await {
        r.subscribe(vec![Filter::new().since(Timestamp::now())], None)
            .await
            .expect("Did not subscribe successfully.");
    }

    Ok(gnostique)
}

// async fn load_config(dirs: &ProjectDirs, filename: &str) -> Result<Config, LoadError> {
//     let file = dirs.config_dir().join(filename);

//     let exists = tokio::fs::try_exists(&file)
//         .await
//         .map_err(LoadError::ConfigFile)?;

//     let x = toml::to_string_pretty(&Config {
//         default_identity: None,
//         db_file: dirs.data_dir().join("gnostique.db"),
//     });

//     let xxx = config::Config::builder()
//         .add_source(config::File::from(file.as_ref()))
//         .build()
//         .map_err(LoadError::Config)?;

//     xxx.try_deserialize().map_err(LoadError::Config)
// }
