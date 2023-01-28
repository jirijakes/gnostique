use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use directories::ProjectDirs;
use gtk::gdk;
use gtk::prelude::*;
use nostr_sdk::nostr::nips::{nip05, nip11};
use nostr_sdk::nostr::prelude::*;
use nostr_sdk::nostr::Event;
use nostr_sdk::Client;
use relm4::actions::{RelmAction, RelmActionGroup};
use relm4::component::*;
use relm4::factory::AsyncFactoryVecDeque;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{query, SqlitePool};
use tokio::time::interval;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::lane::{Lane, LaneMsg};
use crate::nostr::{EventExt, Persona};
use crate::ui::details::*;
use crate::ui::statusbar::StatusBar;
use crate::Gnostique;

pub struct Win {
    gnostique: Arc<Gnostique>,
    lanes: AsyncFactoryVecDeque<Lane>,
    details: Controller<DetailsWindow>,
    status_bar: Controller<StatusBar>,
}

#[derive(Debug)]
pub enum Msg {
    Event(Url, Event),
    ShowDetail(Details),
    AvatarBitmap {
        pubkey: XOnlyPublicKey,
        file: PathBuf,
    },
    Nip05Verified(XOnlyPublicKey),
}

#[derive(Debug)]
pub enum WinCmd {
    AvatarBitmap {
        pubkey: XOnlyPublicKey,
        file: PathBuf,
    },
    Nip05Verified(XOnlyPublicKey),
    Noop,
}

#[relm4::component(pub async)]
impl AsyncComponent for Win {
    type Init = ();
    type Input = Msg;
    type Output = ();
    type CommandOutput = WinCmd;

    #[rustfmt::skip]
    view! {
        #[name(window)]
        gtk::ApplicationWindow {
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

	        #[local_ref]
	        lanes_box -> gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_vexpand: true,
	        },

                #[local_ref]
                status_bar -> gtk::Box { }
            }
	}
    }

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let gnostique = relm4::spawn(init_app()).await.unwrap();

        relm4::spawn(refresh_relay_information(gnostique.clone()));

        let lanes = AsyncFactoryVecDeque::new(gtk::Box::default(), sender.input_sender());

        // TODO: join handle?
        let mut notif = gnostique.client.notifications();
        relm4::spawn(async move {
            include_str!("../resources/febbaba219357c6c64adfa2e01789f274aa60e90c289938bfc80dd91facb2899.json").lines().for_each(|l| {
                let ev = nostr_sdk::nostr::event::Event::from_json(l).unwrap();
                let url: Url = "http://example.com".parse().unwrap();
                sender.input(Msg::Event(url, ev));
            });

            // while let Ok(not) = notif.recv().await {
            // sender.input(Msg::Notification(not));
            // }
        });

        let mut model = Win {
            gnostique: gnostique.clone(),
            lanes,
            details: DetailsWindow::builder().launch(()).detach(),
            status_bar: StatusBar::builder().launch(gnostique).detach(),
        };

        let lanes_box = model.lanes.widget();
        let status_bar = model.status_bar.widget();
        let widgets = view_output!();

        {
            let mut guard = model.lanes.guard();
            // Create one lane.
            guard.push_back(None);
            // guard.push_back(Some(
            // "b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b"
            // .parse()
            // .unwrap(),
            // ));
        }

        let group = RelmActionGroup::<AuthorActionGroup>::new();
        let copy: RelmAction<Copy> = RelmAction::new_with_target_value(|_, string: String| {
            let display = gdk::Display::default().unwrap();
            let clipboard = display.clipboard();
            clipboard.set_text(&string);
        });
        group.add_action(&copy);
        let actions = group.into_action_group();
        widgets.window.insert_action_group("author", Some(&actions));

        AsyncComponentParts { model, widgets }
    }

    async fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: AsyncComponentSender<Win>,
        _root: &Self::Root,
    ) {
        match msg {
            WinCmd::AvatarBitmap { pubkey, file } => {
                sender.input(Msg::AvatarBitmap { pubkey, file })
            }
            WinCmd::Nip05Verified(nip05) => sender.input(Msg::Nip05Verified(nip05)),
            WinCmd::Noop => {}
        }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            Msg::Event(relay, event) => self.received_event(relay, event, sender).await,

            Msg::ShowDetail(details) => self.details.emit(DetailsWindowInput::Show(details)),

            Msg::Nip05Verified(nip05) => self.lanes.broadcast(LaneMsg::Nip05Verified(nip05)),

            Msg::AvatarBitmap { pubkey, file } => {
                self.lanes.broadcast(LaneMsg::AvatarBitmap {
                    pubkey,
                    bitmap: Arc::new(gdk::Texture::from_filename(file).unwrap()),
                });
            }
        }
    }
}

impl Win {
    async fn offer_relay_url(&self, relay: &Url) {
        async fn go(pool: Arc<SqlitePool>, relay_str: String) {
            let _ = query!(
                "INSERT INTO relays(url) VALUES (?) ON CONFLICT(url) DO NOTHING",
                relay_str
            )
            .execute(pool.as_ref())
            .await;
        }

        relm4::spawn(go(self.gnostique.pool.clone(), relay.to_string()))
            .await
            .unwrap();
    }

    async fn received_event(
        &mut self,
        relay: Url,
        event: Event,
        sender: AsyncComponentSender<Self>,
    ) {
        self.offer_relay_url(&relay).await;

        for r in event.collect_relays() {
            self.offer_relay_url(&r).await
        }

        match event.kind {
            Kind::TextNote => self.received_text_note(relay, event),
            Kind::Metadata => self.received_metadata(relay, event, sender).await,
            Kind::Reaction => self.received_reaction(relay, event),
            _ => {}
        }
    }

    fn received_text_note(&self, _relay: Url, event: Event) {
        // Send the event to all lanes, they will decide themselves what to do with it.
        self.lanes.broadcast(LaneMsg::NewTextNote {
            event: Rc::new(event),
        })
    }

    async fn received_metadata(
        &self,
        _relay: Url,
        event: Event,
        sender: AsyncComponentSender<Self>,
    ) {
        async fn insert(pool: Arc<SqlitePool>, pubkey_vec: Vec<u8>, json: String) {
            let _ = query!(
                r#"
INSERT INTO metadata (author, event) VALUES (?, ?)
ON CONFLICT (author) DO UPDATE SET event = EXCLUDED.event
"#,
                pubkey_vec,
                json
            )
            .execute(pool.as_ref())
            .await;
        }

        let json = event.as_pretty_json();
        let metadata = event.as_metadata().unwrap();

        relm4::spawn(insert(
            self.gnostique.pool.clone(),
            event.pubkey.serialize().to_vec(),
            json.clone(),
        ))
        .await
        .unwrap();

        // If the metadata contains valid URL, download it as an avatar.
        if let Some(url) = metadata.picture.and_then(|p| Url::parse(&p).ok()) {
            sender.oneshot_command(obtain_avatar(
                self.gnostique.dirs.clone(),
                event.pubkey,
                url,
            ));
        }

        if let Some(nip05) = metadata.nip05.clone() {
            sender.oneshot_command(verify_nip05(
                self.gnostique.pool.clone(),
                event.pubkey,
                nip05,
            ));
        }

        self.lanes.broadcast(LaneMsg::UpdatedProfile {
            author: Persona {
                pubkey: event.pubkey,
                name: metadata.name,
                nip05: metadata.nip05,
                nip05_verified: false,
            },
            metadata_json: Arc::new(json),
        });
    }

    fn received_reaction(&self, _relay: Url, event: Event) {
        if let Some(to) = event.reacts_to() {
            self.lanes.broadcast(LaneMsg::Reaction {
                event: to,
                reaction: event.content,
            });
        }
    }
}

async fn verify_nip05(pool: Arc<SqlitePool>, pubkey: XOnlyPublicKey, nip05: String) -> WinCmd {
    let pubkey_bytes = pubkey.serialize().to_vec();
    // If the nip05 is already verified and not for too long, just confirm.
    let x = query!(
        r#"
SELECT (unixepoch('now') - unixepoch(nip05_verified)) / 60 / 60 AS "hours?: u32"
FROM metadata WHERE author = ?"#,
        pubkey_bytes
    )
    .fetch_optional(pool.as_ref())
    .await;

    if let Ok(result) = x {
        let x = result.and_then(|r| r.hours);

        match x {
            Some(hours) if hours < 12 => {
                info!("NIP05: {} verified {} hours ago", nip05, hours);
                WinCmd::Nip05Verified(pubkey)
            }
            _ => {
                info!("NIP05: Verifying {}.", nip05);
                // If it's not yet verified or been verified for very long, update.
                if nip05::verify(pubkey, &nip05, None).await.is_ok() {
                    let _ = query!(
                        r#"
UPDATE metadata SET nip05_verified = datetime('now')
WHERE author = ?"#,
                        pubkey_bytes
                    )
                    .execute(pool.as_ref())
                    .await;

                    info!("NIP05: {} verified.", nip05);
                    WinCmd::Nip05Verified(pubkey)
                } else {
                    info!("NIP05: {} verification failed.", nip05);
                    WinCmd::Noop
                }
            }
        }
    } else {
        WinCmd::Noop
    }
}

/// Find `pubkey`'s avatar image either in cache or, if not available,
/// download it from `url` and then cache.
async fn obtain_avatar(dirs: ProjectDirs, pubkey: XOnlyPublicKey, url: Url) -> WinCmd {
    let filename: PathBuf = pubkey.to_string().into();

    let cache = dirs.cache_dir().join("avatars");
    tokio::fs::create_dir_all(&cache).await.unwrap();
    let file = cache.join(&filename);

    let url_s = url.to_string();

    if !file.is_file() {
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        info!("Downloading {}", url_s);

        let mut f = tokio::fs::File::create(&file).await.unwrap();
        let response = reqwest::get(url).await.unwrap();
        // let content_length = response.headers().get("content-length");
        let mut bytes = response.bytes_stream();

        while let Some(chunk) = bytes.next().await {
            let c = chunk.unwrap();
            // println!("{}", c.len());
            f.write_all(&c).await.unwrap();
        }
        info!("Finished downloading: {}", url_s);
    } else {
        info!("Avatar obtained from cache: {}", url_s);
    }

    WinCmd::AvatarBitmap { pubkey, file }
}

/// Regularly, and in the background, obtain information about relays.
async fn refresh_relay_information(gnostique: Arc<Gnostique>) {
    let mut int = interval(Duration::from_secs(60));
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

relm4::new_action_group!(pub AuthorActionGroup, "author");
relm4::new_stateful_action!(pub Copy, AuthorActionGroup, "copy-hex", String, ());

async fn init_app() -> Arc<Gnostique> {
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

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let secret_key =
        SecretKey::from_bech32("nsec1qh685ta6ht7emkn8nlggzjfl0h58zxntgsdjgxmvjz2kctv5puysjcmm03")
            .unwrap();

    // npub1mwe5spuec22ch97tun3znyn8vcwrt6zgpfvs7gmlysm0nqn3g5msr0653t
    let keys = Keys::new(secret_key);

    let dirs = ProjectDirs::from("com.jirijakes", "", "Gnostique").unwrap();
    tokio::fs::create_dir_all(dirs.data_dir()).await.unwrap();

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
            ("wss://brb.io", None),
            ("wss://relay.nostr.info", None),
            ("wss://nostr-relay.wlvs.space", None),
            ("wss://nostr.onsats.org", None),
            ("wss://nostr.openchain.fr", None),
        ])
        .await
        .unwrap();

    gnostique.client.connect().await;

    // gnostique
    //     .client
    //     .get_events_of(vec![
    //         SubscriptionFilter::new()
    //             .author(
    //                 "febbaba219357c6c64adfa2e01789f274aa60e90c289938bfc80dd91facb2899"
    //                     .parse()
    //                     .unwrap(),
    //             )
    //             .limit(100),
    //         SubscriptionFilter::new()
    //             .pubkey(
    //                 "febbaba219357c6c64adfa2e01789f274aa60e90c289938bfc80dd91facb2899"
    //                     .parse()
    //                     .unwrap(),
    //             )
    //             .limit(100),
    //     ])
    //     .await?
    //     .iter()
    //     .for_each(|a| println!("{}", a.as_json().unwrap()));

    gnostique
}
