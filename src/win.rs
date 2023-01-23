use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gtk::gdk;
use gtk::prelude::*;
use nostr_sdk::nostr::prelude::*;
use nostr_sdk::nostr::Event;
use nostr_sdk::Client;
use relm4::component::*;
use relm4::factory::FactoryVecDeque;
use tracing::info;

use crate::lane::{Lane, LaneMsg};
use crate::nostr::{EventExt, Persona};
use crate::ui::details::*;

pub struct Win {
    // client: Client,
    lanes: FactoryVecDeque<Lane>,
    details: Controller<DetailsWindow>,
}

#[derive(Debug)]
pub enum Msg {
    Event(Url, Event),
    ShowDetail(Details),
    AvatarBitmap {
        pubkey: XOnlyPublicKey,
        file: PathBuf,
    },
}

#[derive(Debug)]
pub enum GnostiqueCmd {
    AvatarBitmap {
        pubkey: XOnlyPublicKey,
        file: PathBuf,
    },
}

#[relm4::component(pub async)]
impl AsyncComponent for Win {
    type Init = Client;
    type Input = Msg;
    type Output = ();
    type CommandOutput = GnostiqueCmd;

    #[rustfmt::skip]
    view! {
	gtk::ApplicationWindow {
	    #[local_ref]
	    lanes_box -> gtk::Box {
		set_orientation: gtk::Orientation::Horizontal,
		set_vexpand: true,
	    }
	}
    }

    async fn init(
        client: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let lanes = FactoryVecDeque::new(gtk::Box::default(), sender.input_sender());

        // TODO: join handle?
        let mut notif = client.notifications();
        tokio::spawn(async move {
            include_str!(
                "../resources/b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b.json"
            )
            .lines()
            .for_each(|l| {
                let ev = nostr_sdk::nostr::event::Event::from_json(l).unwrap();
                let url: Url = "http://example.com".parse().unwrap();
                sender.input(Msg::Event(url, ev));
            });

            // while let Ok(not) = notif.recv().await {
            // sender.input(Msg::Notification(not));
            // }
        });

        let mut model = Win {
            // client,
            lanes,
            details: DetailsWindow::builder().launch(()).detach(),
        };

        let lanes_box = model.lanes.widget();
        let widgets = view_output!();

        {
            let mut guard = model.lanes.guard();
            // Create one lane.
            guard.push_back(Some(
                // "3b39477d16f6433ad7a6a1e68c0ee88ecd5acd087139583e6246adfdb3ce4b3b"
                "b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b"
                    .parse()
                    .unwrap(),
            ));
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
            GnostiqueCmd::AvatarBitmap { pubkey, file } => {
                sender.input(Msg::AvatarBitmap { pubkey, file })
            }
        }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            Msg::Event(relay, event) => self.received_event(relay, event, sender),

            Msg::ShowDetail(details) => self.details.emit(DetailsWindowInput::Show(details)),

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
    fn received_event(&mut self, relay: Url, event: Event, sender: AsyncComponentSender<Self>) {
        match event.kind {
            Kind::TextNote => self.received_text_note(relay, event),
            Kind::Metadata => self.received_metadata(relay, event, sender),
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

    fn received_metadata(&self, _relay: Url, event: Event, sender: AsyncComponentSender<Self>) {
        let json = event.as_pretty_json();
        let m = event.as_metadata().unwrap();

        // If the metadata contains valid URL, download it as an avatar.
        if let Some(url) = m.picture.and_then(|p| Url::parse(&p).ok()) {
            sender.oneshot_command(obtain_avatar(event.pubkey, url));
        }

        self.lanes.broadcast(LaneMsg::UpdatedProfile {
            author: Persona {
                pubkey: event.pubkey,
                name: m.name,
            },
            metadata_json: json,
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

/// Find `pubkey`'s avatar image either in cache or, if not available,
/// download it from `url` and then cache.
async fn obtain_avatar(pubkey: XOnlyPublicKey, url: Url) -> GnostiqueCmd {
    let filename: PathBuf = pubkey.to_string().into();

    let cache = directories::ProjectDirs::from("com.jirijakes", "", "Gnostique")
        .unwrap()
        .cache_dir()
        .join("avatars");
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

    GnostiqueCmd::AvatarBitmap { pubkey, file }
}
