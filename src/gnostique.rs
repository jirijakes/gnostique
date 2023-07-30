use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;

use age::Decryptor;
use directories::ProjectDirs;
use nostr_sdk::prelude::{Event, EventId, Metadata, XOnlyPublicKey};
use nostr_sdk::{Client, Filter, Timestamp};
use reqwest::Url;
use secrecy::SecretString;
use sqlx::{query, SqlitePool};
use tokio::io::AsyncReadExt;

use crate::demand::Demand;
use crate::download::Download;
use crate::identity::Identity;
use crate::nostr::Persona;

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
}

impl Gnostique {
    fn new(pool: SqlitePool, dirs: ProjectDirs, client: Client) -> Gnostique {
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

    // pub fn dirs(&self) -> &ProjectDirs {
    //     &self.0.dirs
    // }

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
            ("wss://eden.nostr.land", None),
            ("wss://nostr.fmt.wiz.biz", None),
            ("wss://relay.damus.io", None),
            ("wss://nostr-pub.wellorder.net", None),
            ("wss://offchain.pub", None),
            ("wss://nos.lol", None),
            ("wss://relay.snort.social", None),
            ("wss://relay.current.fyi", None),
        ])
        .await
        .unwrap();

    gnostique.client().connect().await;

    gnostique
        .client()
        // .subscribe(vec![crate::follow::Follow::new().subscriptions()])
        .subscribe(vec![Filter::new().since(Timestamp::now())])
        .await;

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
