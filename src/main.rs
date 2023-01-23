#![feature(option_result_contains)]

mod lane;
mod nostr;
mod ui;
mod win;

use std::sync::Arc;

use directories::ProjectDirs;
use nostr_sdk::nostr::prelude::*;
// use nostr_sdk::nostr::util::time::timestamp;
use nostr_sdk::*;
use relm4::*;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tracing_subscriber::EnvFilter;

#[derive(Debug)]
pub struct Gnostique {
    pool: Arc<SqlitePool>,
    dirs: ProjectDirs,
    client: Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // .pretty()
        .compact()
        .with_max_level(tracing::Level::TRACE)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(true)
        .with_env_filter(EnvFilter::new("info,relm4=warn"))
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    let secret_key =
        SecretKey::from_bech32("nsec1qh685ta6ht7emkn8nlggzjfl0h58zxntgsdjgxmvjz2kctv5puysjcmm03")
            .unwrap();

    // npub1mwe5spuec22ch97tun3znyn8vcwrt6zgpfvs7gmlysm0nqn3g5msr0653t
    let keys = Keys::new(secret_key);

    let dirs = ProjectDirs::from("com.jirijakes", "", "Gnostique").unwrap();
    tokio::fs::create_dir_all(dirs.data_dir()).await?;

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

    let pool = Arc::new(pool);
    let client = Client::new(&keys);
    let gnostique = Arc::new(Gnostique { dirs, pool, client });

    gnostique
        .client
        .add_relays(vec![
            ("wss://nostr.onsats.org", None),
            ("wss://nostr.openchain.fr", None),
        ])
        .await?;

    // npub1gl23nnfmlewvvuz7xgrrauuexx2xj70whdf5yhd47tj0r8p68t6sww70gt

    gnostique.client.connect().await;

    // client.sync().await?;

    // let sub = SubscriptionFilter::new()
    //     // .pubkey(XOnlyPublicKey::from_bech32(
    //     // "npub1gl23nnfmlewvvuz7xgrrauuexx2xj70whdf5yhd47tj0r8p68t6sww70gt",
    //     // )?)
    //     .events(vec![
    //         "b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b"
    //             .parse()
    //             .unwrap(),
    //     ])
    //     // .limit(20)
    //     ;

    // client
    //     .subscribe(vec![
    //         sub,
    //         SubscriptionFilter::new()
    //             .id("b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b"),
    //     ])
    //     .await?;

    // let x = client
    //     .get_events_of(vec![
    //         sub,
    //         SubscriptionFilter::new()
    //             .id("b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b"),
    //     ])
    //     .await?;

    // x.iter().for_each(|a| println!("{}", a.as_json().unwrap()));

    let app = RelmApp::new("com.jirijakes.gnostique");

    let settings = gtk::Settings::default().unwrap();
    settings.set_gtk_application_prefer_dark_theme(true);

    app.run_async::<crate::win::Win>(gnostique);

    Ok(())
}
