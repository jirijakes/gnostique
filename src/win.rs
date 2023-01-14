use gtk::prelude::*;
use nostr_sdk::nostr::prelude::*;
use nostr_sdk::Client;
use nostr_sdk::RelayPoolNotification;
use relm4::component::*;
use relm4::factory::FactoryVecDeque;
use relm4::gtk;
// use relm4::prelude::*;

use crate::lane::Lane;
use crate::lane::LaneMsg;
use crate::ui::details::*;

pub struct Gnostique {
    client: Client,
    lanes: FactoryVecDeque<Lane>,
    details: Controller<DetailsWindow>,
}

#[derive(Debug)]
pub enum Msg {
    Notification(RelayPoolNotification),
    ShowDetail(Details),
}

#[relm4::component(pub async)]
impl AsyncComponent for Gnostique {
    type Init = Client;
    type Input = Msg;
    type Output = ();
    type CommandOutput = ();

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
            let f = std::fs::File::open(
                "b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b.json",
            )
            .unwrap();
            let f = std::io::BufReader::new(f);

            use std::io::prelude::*;

            f.lines().for_each(|l| {
                let ev = nostr_sdk::nostr::event::Event::from_json(l.unwrap()).unwrap();
                let url: Url = "http://example.com".parse().unwrap();
                sender.input(Msg::Notification(RelayPoolNotification::Event(url, ev)));
            });

            // while let Ok(not) = notif.recv().await {
            // sender.input(Msg::Notification(not));
            // }
        });

        let details = DetailsWindow::builder().launch(()).detach();

        let mut model = Gnostique {
            client,
            lanes,
            details,
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

    async fn update(
        &mut self,
        msg: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            Msg::Notification(RelayPoolNotification::Event(_url, ev))
                if ev.kind == Kind::Base(KindBase::TextNote) =>
            {
                let profile = self
                    .client
                    .store()
                    .ok()
                    .and_then(|s| s.get_profile(ev.pubkey).ok());

                // let replies = ev
                //     .tags
                //     .iter()
                //     .filter_map(|t| match t {
                //         Tag::Event(e, _, Some(Marker::Root)) => Some(*e),
                //         _ => None,
                //     })
                //     .collect::<HashSet<_>>();

                // if !replies.is_empty() {
                //     let reply_filter =
                //         SubscriptionFilter::new().events(replies.into_iter().collect());
                //     let x = self.client.get_events_of(vec![reply_filter]).await.unwrap();
                //     x.iter().for_each(|e| println!(">>>> {e:?}"));
                // }

                // println!("{}", ev.as_json().unwrap());
                // println!("{:?}", ev.tags);
                // println!();

                // let event = EventContext { event: ev, profile };

                // Send the event to all lanes, they will decide themselves what to do with it.
                for i in 0..self.lanes.len() {
                    self.lanes.send(
                        i,
                        LaneMsg::NewTextNote {
                            event: ev.clone(),
                            profile: profile.clone(),
                        },
                    );
                }
            }

            Msg::Notification(RelayPoolNotification::Event(_url, ev))
                if ev.kind == Kind::Base(KindBase::Metadata) =>
            {
                let json = serde_json::to_string_pretty(&ev).unwrap();
                let m = Metadata::from_json(ev.content).unwrap();
                for i in 0..self.lanes.len() {
                    self.lanes.send(
                        i,
                        LaneMsg::UpdatedProfile {
                            author_pubkey: ev.pubkey,
                            author_name: m.name.clone(),
                            metadata_json: json.clone(),
                        },
                    );
                }
            }

            Msg::Notification(RelayPoolNotification::Event(_url, ev))
                if ev.kind == Kind::Base(KindBase::ContactList) =>
            {
                println!("{ev:?}")
            }

            Msg::ShowDetail(details) => self.details.emit(DetailsWindowInput::Show(details)),

            ev => {}
        }
    }
}
