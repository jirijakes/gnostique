use std::path::PathBuf;
use std::sync::Arc;

use gtk::gdk;
use gtk::prelude::*;
use nostr_sdk::nostr::prelude::*;
use relm4::component::*;
use relm4::factory::AsyncFactoryVecDeque;
use tracing::warn;

use crate::follow::Follow;
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
    Event(crate::stream::X),
    ShowDetail(Details),
    WriteNote,
    EditProfile,
    UpdateProfile(Metadata),
    Send(String),
    Noop,
    Quit,
    MetadataBitmap {
        pubkey: XOnlyPublicKey,
        url: Url,
        file: PathBuf,
    },
    Nip05Verified(XOnlyPublicKey),
}

#[relm4::component(pub async)]
impl AsyncComponent for Win {
    type Init = ();
    type Input = Msg;
    type Output = ();
    type CommandOutput = ();

    #[rustfmt::skip]
    view! {
        #[name(window)]
        gtk::ApplicationWindow {
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Stack {
                    gtk::Box {
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::Center,
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 18,
                        set_widget_name: "password",

                        gtk::Label {
                            set_label: "Unlock Gnostique identity",
                            add_css_class: "caption",
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                            add_css_class: "passwordbox",
                            
                            gtk::Label {
                                set_xalign: 0.0,
                                set_label: "Enter password:"
                            },

                            #[name(password)]
                            gtk::PasswordEntry {
                                set_hexpand: true,
                                set_show_peek_icon: true
                            },
                            gtk::Box {
                                set_halign: gtk::Align::End,
                                set_spacing: 8,
                                add_css_class: "buttons",

                                gtk::Button {
                                    add_css_class: "suggested-action",
                                    set_label: "Unlock",
                                    connect_clicked[password] => move |_| {
                                        println!(">>>> {:?}", password.text());
                                    }
                                },

                                gtk::Button {
                                    set_label: "Quit",
                                    connect_clicked => Msg::Quit,
                                }
                            }
                        }
                    },

                    #[local_ref]
                    lanes_box -> gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_vexpand: true,
                    }
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
            gnostique.clone(),
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

            guard.push_back(LaneKind::Feed(Follow::new()));

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

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            Msg::Event(crate::stream::X::TextNote {
                event,
                relays,
                author,
                avatar,
                repost,
            }) => {
                let pubkey = event.pubkey;
                let url = author.as_ref().and_then(|a| a.avatar.as_ref()).cloned();

                self.lanes.broadcast(LaneMsg::NewTextNote {
                    event: Arc::new(event),
                    relays,
                    author,
                    repost,
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

            Msg::Event(crate::stream::X::Reaction { event_id, content }) => {
                self.lanes.broadcast(LaneMsg::Reaction {
                    event: event_id,
                    reaction: content,
                })
            }

            Msg::Event(crate::stream::X::Metadata { persona, avatar }) => {
                let url = persona.avatar.clone();
                let pubkey = persona.pubkey;

                self.lanes
                    .broadcast(LaneMsg::UpdatedProfile { author: persona });

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

            Msg::WriteNote => self.write_note.emit(WriteNoteInput::Show),

            Msg::Noop => {}

            Msg::Quit => {
                relm4::main_application().quit();
            }
            
            Msg::EditProfile => self.edit_profile.emit(EditProfileInput::Show),

            Msg::UpdateProfile(metadata) => {
                let client = self.gnostique.client().clone();
                relm4::spawn(async move { client.set_metadata(metadata).await })
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

/// Translates result of [`edit profile`](editprofile::component) dialog to [`Msg`].
fn forward_edit_profile(result: EditProfileResult) -> Msg {
    match result {
        EditProfileResult::Apply(metadata) => Msg::UpdateProfile(metadata),
    }
}
