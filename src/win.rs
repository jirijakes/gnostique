use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use directories::ProjectDirs;
use gtk::gdk;
use gtk::prelude::*;
use nostr_sdk::nostr::prelude::*;
use nostr_sdk::nostr::util::nips::nip05;
use nostr_sdk::nostr::Event;
use relm4::component::*;
use relm4::factory::FactoryVecDeque;
use sqlx::{query, SqlitePool};
use tracing::info;

use crate::lane::{Lane, LaneMsg};
use crate::nostr::{EventExt, Persona};
use crate::ui::details::*;
use crate::ui::statusbar::StatusBar;
use crate::Gnostique;

pub struct Win {
    gnostique: Arc<Gnostique>,
    lanes: FactoryVecDeque<Lane>,
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
    type Init = Arc<Gnostique>;
    type Input = Msg;
    type Output = ();
    type CommandOutput = WinCmd;

    #[rustfmt::skip]
    view! {
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
        gnostique: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let lanes = FactoryVecDeque::new(gtk::Box::default(), sender.input_sender());

        // TODO: join handle?
        let mut notif = gnostique.client.notifications();
        tokio::spawn(async move {
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
    async fn received_event(
        &mut self,
        relay: Url,
        event: Event,
        sender: AsyncComponentSender<Self>,
    ) {
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
        let json = event.as_pretty_json();
        let metadata = event.as_metadata().unwrap();

        let pool = self.gnostique.pool.clone();
        let pubkey_vec = event.pubkey.serialize().to_vec();

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
