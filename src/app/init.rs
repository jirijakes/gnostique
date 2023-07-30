use age::Decryptor;
use directories::ProjectDirs;
use nostr_sdk::prelude::*;
use secrecy::SecretString;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tracing_subscriber::EnvFilter;

use crate::gnostique::Gnostique;
use crate::identity::Identity;

/// Initializes the application, reads all the configurations and databases
/// and all that and returns it all inside [`Gnostique`].
///
/// Requires Tokio.
pub async fn make_gnostique(password: SecretString) -> Result<Gnostique, String> {
    // Logging, tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // .pretty()
        .compact()
        .with_max_level(tracing::Level::TRACE)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(true)
        .with_env_filter(EnvFilter::new("debug,hyper=info,relm4=warn"))
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    use std::io::prelude::*;

    let ciph = std::fs::File::open("key");
    let mut buf = Vec::new();
    ciph.unwrap()
        .read_to_end(&mut buf)
        .map_err(|e| e.to_string())?;

    let id: Identity = if let Ok(Decryptor::Passphrase(d)) = Decryptor::new(buf.as_slice()) {
        let rea = d.decrypt(&password, Some(18)).map_err(|e| e.to_string())?;
        serde_json::from_reader(rea).map_err(|e| e.to_string())?
    } else {
        Err("Can't".to_string())?
    };

    let dirs = ProjectDirs::from("com.jirijakes", "", "Gnostique").unwrap();
    tokio::fs::create_dir_all(dirs.data_dir()).await.unwrap();

    // Database
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(dirs.data_dir().join("gnostique.db"))
                .create_if_missing(true),
        )
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();

    // Nostr
    let client = Client::new(&id.nostr_key());
    let gnostique = Gnostique::new(pool, dirs, client);

    // gnostique
    //     .client()
    //     .add_relays(vec![
    //         // ("ws://localhost:8080", None),
    //         ("wss://brb.io", None),
    //         ("wss://relay.nostr.info", None),
    //         ("wss://nostr.orangepill.dev", None),
    //         ("wss://nostr-pub.wellorder.net", None),
    //         ("wss://nostr.openchain.fr", None),
    //         ("wss://relay.damus.io", None),
    //     ])
    //     .await
    //     .unwrap();

    // gnostique.client().connect().await;

    // gnostique
    //     .client()
    //     .subscribe(vec![crate::follow::Follow::new().subscriptions()])
    //     .await;

    Ok(gnostique)
}
