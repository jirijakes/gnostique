use std::str::FromStr;

use directories::ProjectDirs;
use nostr_sdk::prelude::*;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tracing_subscriber::EnvFilter;

use crate::follow::Follow;
use crate::Gnostique;

/// Initializes the application, reads all the configurations and databases
/// and all that and returns it all inside [`Gnostique`].
///
/// Requires Tokio.
pub async fn make_gnostique() -> Gnostique {
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

    let secret_key = SecretKey::from_str(include_str!("../../.seckey")).unwrap();
    let keys = Keys::new(secret_key);

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
    let client = Client::new(&keys);
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

    // gnostique
    //     .client()
    //     .subscribe(vec![Follow::new().subscriptions()])
    //     .await;

    gnostique
}
