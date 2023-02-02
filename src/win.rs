use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gtk::gdk;
use gtk::prelude::*;
use nostr_sdk::nostr::nips::nip05;
use nostr_sdk::nostr::prelude::*;
use nostr_sdk::nostr::Event;
use relm4::component::*;
use relm4::factory::AsyncFactoryVecDeque;
use relm4::{Sender, ShutdownReceiver};
use sqlx::{query, SqlitePool};
use tracing::{info, warn};

use crate::download::{Download, DownloadResult};
use crate::nostr::{EventExt, Persona};
use crate::ui::details::*;
use crate::ui::editprofile::model::*;
use crate::ui::lane::*;
use crate::ui::statusbar::*;
use crate::ui::writenote::model::*;
use crate::Gnostique;

pub struct Win {
    gnostique: Gnostique,
    lanes: AsyncFactoryVecDeque<Lane>,
    details: Controller<DetailsWindow>,
    status_bar: Controller<StatusBar>,
    write_note: Controller<WriteNote>,
    edit_profile: Controller<EditProfile>,
}

#[derive(Debug)]
pub enum Msg {
    Event(Url, Event),
    ShowDetail(Details),
    WriteNote,
    EditProfile,
    UpdateProfile(Metadata),
    Send(String),
    Noop,
    MetadataBitmap {
        pubkey: XOnlyPublicKey,
        url: Url,
        file: PathBuf,
    },
    Nip05Verified(XOnlyPublicKey),
}

#[derive(Debug)]
pub enum WinCmd {
    MetadataBitmap {
        pubkey: XOnlyPublicKey,
        url: Url,
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
        let gnostique = relm4::spawn(crate::app::init::make_gnostique())
            .await
            .unwrap();

        // relm4::spawn(crate::app::task::refresh_relay_information(
        //     gnostique.clone(),
        // ));

        relm4::spawn(crate::app::task::receive_events(
            gnostique.client().clone(),
            sender.clone(),
        ));

        let mut model = Win {
            gnostique: gnostique.clone(),
            lanes: AsyncFactoryVecDeque::new(gtk::Box::default(), sender.input_sender()),
            details: DetailsWindow::builder().launch(()).detach(),
            status_bar: StatusBar::builder().launch(gnostique).detach(),
            edit_profile: EditProfile::builder()
                .launch(())
                .forward(sender.input_sender(), forward_edit_profile),
            write_note: WriteNote::builder()
                .launch(())
                .forward(sender.input_sender(), |result| match result {
                    WriteNoteResult::Send(c) => Msg::Send(c),
                    _ => Msg::Noop,
                }),
        };

        let lanes_box = model.lanes.widget();
        let status_bar = model.status_bar.widget();
        let widgets = view_output!();

        {
            let mut guard = model.lanes.guard();

            guard.push_back(LaneKind::Sink);

            // guard.push_back(LaneKind::Profile(
            //     "febbaba219357c6c64adfa2e01789f274aa60e90c289938bfc80dd91facb2899"
            //         .parse()
            //         .unwrap(),
            // ));
            // guard.push_back(LaneKind::Thread(
            //     "b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b"
            //         .parse()
            //         .unwrap(),
            // ));
        }

        widgets
            .window
            .insert_action_group("author", Some(&crate::app::action::make_author_actions()));

        widgets.window.insert_action_group(
            "main",
            Some(&crate::app::action::make_main_menu_actions(sender)),
        );

        AsyncComponentParts { model, widgets }
    }

    async fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: AsyncComponentSender<Win>,
        _root: &Self::Root,
    ) {
        match msg {
            WinCmd::MetadataBitmap { pubkey, url, file } => {
                sender.input(Msg::MetadataBitmap { pubkey, url, file })
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

            Msg::WriteNote => self.write_note.emit(WriteNoteInput::Show),

            Msg::Noop => {}

            Msg::EditProfile => self.edit_profile.emit(EditProfileInput::Show),

            Msg::UpdateProfile(metadata) => {
                let client = self.gnostique.client().clone();
                relm4::spawn(async move { client.update_profile(metadata).await })
                    .await
                    .unwrap()
                    .unwrap();
            }

            Msg::Send(c) => {
                let client = self.gnostique.client().clone();
                relm4::spawn(async move {
                    client
                        .publish_text_note(
                            c,
                            &[Tag::Generic(
                                TagKind::Custom("client".to_string()),
                                vec!["Gnostique".to_string()],
                            )],
                        )
                        .await
                })
                .await
                .unwrap()
                .unwrap();
            }

            Msg::ShowDetail(details) => self.details.emit(DetailsWindowInput::Show(details)),

            Msg::Nip05Verified(nip05) => self.lanes.broadcast(LaneMsg::Nip05Verified(nip05)),

            Msg::MetadataBitmap { pubkey, url, file } => match gdk::Texture::from_filename(&file) {
                Ok(bitmap) => {
                    self.lanes.broadcast(LaneMsg::MetadataBitmap {
                        pubkey,
                        url,
                        bitmap: Arc::new(bitmap),
                    });
                }
                Err(e) => {
                    warn!("Could not load '{:?}': {}", file, e);
                }
            },
        }
    }
}

impl Win {
    /// Processes an incoming event, mostly delegates work to other methods.
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
            Kind::TextNote => self.received_text_note(relay, event).await,
            Kind::Metadata => self.received_metadata(relay, event, sender).await,
            Kind::Reaction => self.received_reaction(relay, event),
            _ => {}
        }
    }

    /// Processes an incoming text note.
    async fn received_text_note(&self, relay: Url, event: Event) {
        self.gnostique.store_event(&relay, &event).await;
        let relays = self.gnostique.textnote_relays(event.id).await;
        let author = self.gnostique.get_persona(event.pubkey).await;

        // Send the event to all lanes, they will decide themselves what to do with it.
        self.lanes.broadcast(LaneMsg::NewTextNote {
            event: Rc::new(event),
            relays,
            author,
        });

        // TODO: If author is none, we have to retrieve his metadata
        // TODO: Obtain bitmaps for author
    }

    async fn download_command(
        out: Sender<WinCmd>,
        shutdown: ShutdownReceiver,
        pubkey: XOnlyPublicKey,
        download: Download,
        url: Url,
    ) {
        shutdown
            .register(async move {
                match download.cached_file(&url).await {
                    DownloadResult::File(file) => {
                        out.send(WinCmd::MetadataBitmap { pubkey, url, file })
                            .unwrap();
                    }
                    DownloadResult::Dowloading => {
                        // do nothing when being dowloaded, it will arrive
                    }
                }
            })
            .drop_on_shutdown()
            .await;
    }

    async fn received_metadata(
        &self,
        _relay: Url,
        event: Event,
        sender: AsyncComponentSender<Self>,
    ) {
        async fn insert(pool: SqlitePool, pubkey_vec: Vec<u8>, json: String) {
            let _ = query!(
                r#"
INSERT INTO metadata (author, event) VALUES (?, ?)
ON CONFLICT (author) DO UPDATE SET event = EXCLUDED.event
"#,
                pubkey_vec,
                json
            )
            .execute(&pool)
            .await;
        }

        let json = event.as_pretty_json();
        let metadata = event.as_metadata().unwrap();

        relm4::spawn(insert(
            self.gnostique.pool().clone(),
            event.pubkey.serialize().to_vec(),
            json.clone(),
        ))
        .await
        .unwrap();

        let avatar_url = metadata.picture.as_ref().and_then(|p| Url::parse(p).ok());
        let banner_url = metadata.banner.as_ref().and_then(|p| Url::parse(p).ok());

        // If the metadata's picture contains valid URL, download it.
        if let Some(url) = avatar_url.clone() {
            let download = self.gnostique.download().clone();
            sender.command(move |out, shutdown| {
                Self::download_command(out, shutdown, event.pubkey, download, url)
            })
        }

        // // If the metadata's banner contains valid URL, download it.
        // if let Some(url) = banner_url.clone() {
        //     let download = self.gnostique.download().clone();
        //     sender.command(move |out, shutdown| {
        //         Self::download_command(out, shutdown, event.pubkey, download, url)
        //     })
        // }

        if let Some(nip05) = metadata.nip05.clone() {
            sender.oneshot_command(verify_nip05(
                self.gnostique.pool().clone(),
                event.pubkey,
                nip05,
            ));
        }

        self.lanes.broadcast(LaneMsg::UpdatedProfile {
            author: Persona {
                pubkey: event.pubkey,
                name: metadata.name,
                avatar: avatar_url,
                banner: banner_url,
                about: metadata.about,
                nip05: metadata.nip05,
                nip05_verified: false,
                metadata_json: json,
            },
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

    async fn offer_relay_url(&self, relay: &Url) {
        async fn go(pool: SqlitePool, relay_str: String) {
            let _ = query!(
                "INSERT INTO relays(url) VALUES (?) ON CONFLICT(url) DO NOTHING",
                relay_str
            )
            .execute(&pool)
            .await;
        }

        relm4::spawn(go(self.gnostique.pool().clone(), relay.to_string()))
            .await
            .unwrap();
    }
}

async fn verify_nip05(pool: SqlitePool, pubkey: XOnlyPublicKey, nip05: String) -> WinCmd {
    let pubkey_bytes = pubkey.serialize().to_vec();
    // If the nip05 is already verified and not for too long, just confirm.
    let x = query!(
        r#"
SELECT (unixepoch('now') - unixepoch(nip05_verified)) / 60 / 60 AS "hours?: u32"
FROM metadata WHERE author = ?"#,
        pubkey_bytes
    )
    .fetch_optional(&pool)
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
                    .execute(&pool)
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

/// Translates result of [`edit profile`](editprofile::component) dialog to [`Msg`].
fn forward_edit_profile(result: EditProfileResult) -> Msg {
    match result {
        EditProfileResult::Apply(metadata) => Msg::UpdateProfile(metadata),
    }
}
