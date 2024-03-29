use std::path::PathBuf;
use std::sync::Arc;

use gtk::gdk;
use gtk::prelude::*;
use nostr_sdk::nostr::prelude::*;
use relm4::component::*;
use relm4::factory::AsyncFactoryVecDeque;
use relm4::prelude::DynamicIndex;
use tracing::warn;

use super::link::InternalLink;
use crate::gnostique::Gnostique;
use crate::incoming::Incoming;
use crate::nostr::subscriptions::Subscription;
use crate::ui::details::*;
use crate::ui::editprofile::model::*;
use crate::ui::lane::*;
use crate::ui::statusbar::*;
use crate::ui::writenote::model::*;

pub struct Main {
    gnostique: Gnostique,
    lanes: AsyncFactoryVecDeque<Lane>,
    details: Controller<DetailsWindow>,
    status_bar: Controller<StatusBar>,
    write_note: Controller<WriteNote>,
    edit_profile: Controller<EditProfile>,
}

#[derive(Debug)]
pub enum MainInput {
    Incoming(Incoming),
    ShowDetail(Details),
    WriteNote,
    EditProfile,
    UpdateProfile(Metadata),
    Send(String),
    Noop,
    MetadataBitmap {
        pubkey: XOnlyPublicKey,
        url: reqwest::Url,
        file: PathBuf,
    },
    Nip05Verified(XOnlyPublicKey),
    DemandProfile(XOnlyPublicKey, Vec<Url>),
    CloseLane(DynamicIndex),
    LinkClicked(InternalLink),
    RefreshSubscriptions,
}

#[relm4::component(pub async)]
impl AsyncComponent for Main {
    type Init = Gnostique;
    type Input = MainInput;
    type Output = ();
    type CommandOutput = ();

    #[rustfmt::skip]
    view! {
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

    async fn init(
        gnostique: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        // relm4::spawn(crate::app::task::refresh_relay_information(
        //     gnostique.clone(),
        // ));

        relm4::spawn(crate::app::task::receive_events(
            gnostique.clone(),
            sender.clone(),
        ));

        let mut model = Main {
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
                    WriteNoteResult::Send(c) => MainInput::Send(c),
                    _ => MainInput::Noop,
                }),
        };

        let lanes_box = model.lanes.widget();
        let status_bar = model.status_bar.widget();
        let widgets = view_output!();

        model
            .lanes
            .guard()
            .push_back(LaneInit::subscription(Subscription::Sink));

        // widgets
        //     .window
        //     .insert_action_group("author", Some(&crate::app::action::make_author_actions()));

        // widgets.window.insert_action_group(
        //     "main",
        //     Some(&crate::app::action::make_main_menu_actions(sender)),
        // );

        AsyncComponentParts { model, widgets }
    }

    async fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            MainInput::Incoming(Incoming::TextNote {
                note,
                content,
                relays,
                avatar,
                repost,
                referenced_notes,
                referenced_profiles,
            }) => {
                let pubkey = note.author().pubkey;
                let url = note.author().avatar.clone();

                self.lanes.broadcast(LaneMsg::NewTextNote {
                    note,
                    content: Arc::new(content),
                    relays,
                    repost,
                    referenced_notes,
                    referenced_profiles,
                });

                if let Some(ref file) = avatar {
                    match gdk::Texture::from_filename(file) {
                        Ok(bitmap) => {
                            self.lanes.broadcast(LaneMsg::MetadataBitmap {
                                pubkey,
                                url: url.unwrap(),
                                bitmap: Arc::new(bitmap),
                            });
                        }
                        Err(e) => {
                            warn!("Could not load '{:?}': {}", file, e);
                        }
                    }
                }
            }

            MainInput::Incoming(Incoming::Reaction { event_id, content }) => {
                self.lanes.broadcast(LaneMsg::Reaction {
                    event: event_id,
                    reaction: content,
                })
            }

            MainInput::Incoming(Incoming::Metadata { persona, avatar }) => {
                let url = persona.avatar.clone();
                let pubkey = persona.pubkey;

                self.lanes.broadcast(LaneMsg::UpdatedProfile {
                    author: Arc::new(persona),
                });

                if let Some(ref file) = avatar {
                    match gdk::Texture::from_filename(file) {
                        Ok(bitmap) => {
                            self.lanes.broadcast(LaneMsg::MetadataBitmap {
                                pubkey,
                                url: url.unwrap(),
                                bitmap: Arc::new(bitmap),
                            });
                        }
                        Err(e) => {
                            warn!("Could not load '{:?}': {}", file, e);
                        }
                    }
                }
            }

            MainInput::Incoming(Incoming::Preview(p)) => {
                self.lanes.broadcast(LaneMsg::Preview(p));
            }

            MainInput::WriteNote => self.write_note.emit(WriteNoteInput::Show),

            MainInput::CloseLane(id) => {
                self.lanes.guard().remove(id.current_index());
                sender.input(MainInput::RefreshSubscriptions);
            }

            MainInput::RefreshSubscriptions => {
                let client = self.gnostique.client();
                let relays = client.relays().await;
                let relays = relays.values();

                let lane_subs = self
                    .lanes
                    .iter()
                    .filter_map(|l| l.map(|l| l.subscription().clone()))
                    .reduce(|x, y| x.add(y));

                tracing::info!("Subscribing to {lane_subs:?}");

                if let Some(subs) = lane_subs {
                    let sub_filter = subs.to_filters();
                    for relay in relays {
                        let _ = relay.subscribe(sub_filter.clone(), None).await;

                        let active_subs = relay.subscriptions().await;
                        tracing::debug!("On {} subscribed to {:#?}", relay.url(), active_subs);
                    }
                }
            }

            MainInput::LinkClicked(InternalLink::Tag(tag)) => {
                self.lanes
                    .guard()
                    .push_back(LaneInit::subscription(Subscription::hashtag(tag)));
            }

            MainInput::LinkClicked(InternalLink::Event(event)) => {
                self.lanes.guard().push_back(LaneInit::with_focused(
                    Subscription::thread(event.id()),
                    event.id(),
                ));
            }

            MainInput::LinkClicked(InternalLink::Profile(persona, relays)) => {
                self.lanes
                    .guard()
                    .push_back(LaneInit::subscription(Subscription::profile(
                        persona.pubkey,
                        relays,
                    )));
            }

            MainInput::Noop => {}

            MainInput::EditProfile => self.edit_profile.emit(EditProfileInput::Show),

            MainInput::DemandProfile(pubkey, relays) => {
                let demand = self.gnostique.demand().clone();
                relm4::spawn(async move { demand.metadata(pubkey, relays).await })
                    .await
                    .unwrap();
            }

            MainInput::UpdateProfile(metadata) => {
                let client = self.gnostique.client().clone();
                relm4::spawn(async move { client.set_metadata(metadata).await })
                    .await
                    .unwrap()
                    .unwrap();
            }

            MainInput::Send(c) => {
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

            MainInput::ShowDetail(details) => self.details.emit(DetailsWindowInput::Show(details)),

            MainInput::Nip05Verified(nip05) => self.lanes.broadcast(LaneMsg::Nip05Verified(nip05)),

            MainInput::MetadataBitmap { pubkey, url, file } => {
                match gdk::Texture::from_filename(&file) {
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
                }
            }
        };

        self.update_view(widgets, sender);
    }
}

/// Translates result of [`edit profile`](editprofile::component) dialog to [`Msg`].
fn forward_edit_profile(result: EditProfileResult) -> MainInput {
    match result {
        EditProfileResult::Apply(metadata) => MainInput::UpdateProfile(metadata),
    }
}
