use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use gtk::pango::WrapMode;
use gtk::prelude::*;
use nostr_sdk::prelude::ToBech32;
use relm4::component::{AsyncComponent, AsyncComponentController};
use relm4::prelude::*;

use super::model::*;
use super::msg::*;
use crate::app::action::*;
use crate::nostr::*;
use crate::ui::details::Details;
use crate::ui::lane::LaneMsg;
use crate::ui::replies::{Replies, RepliesInput};
use crate::ui::widgets::author::Author;

/*
    +-------------------------------------+
    | REPOST                              |
    +-------------------------------------+
    |   AVATAR    |        AUTHOR         |
    |             +-----------------------+
    |             |        CONTENT        |
    |             +-----------------------+
    |             |       REACTIONS       |
    |             +-----------------------+
    |             |        STATUS         |
    +-------------+-----------------------+
*/

#[relm4::factory(pub)]
impl FactoryComponent for Note {
    type Init = NoteInit;
    type Input = NoteInput;
    type Output = NoteOutput;
    type CommandOutput = ();
    type ParentInput = LaneMsg;
    type ParentWidget = gtk::ListBox;

    #[rustfmt::skip]
    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            // REPOSTER may be prepended here
            
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                add_css_class: "text-note",
                add_css_class: if self.is_central { "central" } else { "text-note" },

                // left column
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    // avatar
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        add_css_class: "avatar",

                        gtk::Image {
                            #[watch]
                            set_from_paintable: Some(self.avatar.as_ref()),
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Start,
                        },

                        gtk::Box {
                            #[watch] set_visible: self.show_hidden_buttons,
                            add_css_class: "hidden-buttons",

                            gtk::Button {
                                set_label: "src",
                                connect_clicked => NoteInput::ShowDetails
                            }
                        }
                    }
                },

                // right column
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_spacing: 10,
                    add_css_class: "right-column",

                    // author
                    gtk::Overlay {
                        Author::with_pubkey(self.author.pubkey) {
                            set_context_menu: Some(&author_menu),
                            #[watch] set_persona: &self.author,
                            #[watch] set_nip05_verified: self.nip05_verified,
                        },
                        add_overlay = &gtk::Label {
                            set_valign: gtk::Align::Start,
                            set_halign: gtk::Align::End,
                            set_tooltip_markup: Some(&self.format_age_tooltip()),
                            add_css_class: "note-age",
                            #[watch] set_label: &self.age,
                        }
                    },

                    // content
                    #[name(content)]
                    gtk::Label {
                        #[watch]
                        set_markup: self.content.augment(&self.event.content).trim(),
                        set_wrap: true,
                        set_wrap_mode: WrapMode::WordChar,
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Start,
                        set_vexpand: true,
                        set_xalign: 0.0,
                        set_selectable: true,
                        add_css_class: "content",

                        connect_activate_link[sender] => move |_, uri| {
                            if uri.starts_with("nostr") || uri.starts_with("gnostique") {
                                sender.output(NoteOutput::LinkClicked(uri.to_string()));
                                gtk::Inhibit(true)
                            } else { gtk::Inhibit(false) }
                        }
                    },

                    self.replies.widget(),

                    // reactions
                    gtk::Grid {
                        // set_column_spacing: 20,
                        set_column_homogeneous: true,
                        set_hexpand: true,
                        add_css_class: "reactions",

                        attach[1, 1, 1, 1] =
                            &gtk::Button {
                                set_halign: gtk::Align::Center,
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 4,
                                    gtk::Image {
                                        set_icon_name: Some("emblem-favorite-symbolic"),
                                        set_pixel_size: 12,
                                    },
                                    gtk::Label {
                                        #[watch] set_label: &self.likes.to_string(),
                                        #[watch] set_visible: self.likes > 0
                                    }
                                }
                            },
                        attach[2, 1, 1, 1] =
                            &gtk::Button {
                                set_halign: gtk::Align::Center,
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 4,
                                    gtk::Image {
                                        set_icon_name: Some("gnostique-down-symbolic"),
                                        set_pixel_size: 12,
                                    },
                                    gtk::Label {
                                        #[watch] set_label: &self.dislikes.to_string(),
                                        #[watch] set_visible: self.dislikes > 0
                                    }
                                }
                            },
                        attach[3, 1, 1, 1] =
                            &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 4,
                                set_halign: gtk::Align::Center,

                                gtk::Button::from_icon_name("gnostique-repost-symbolic") { }
                            },
                        attach[4, 1, 1, 1] =
                            &gtk::MenuButton {
                                set_halign: gtk::Align::Center,
                                set_icon_name: "content-loading-symbolic",
                                set_menu_model: Some(&note_menu)
                            }
                    },

                    // status
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::End,
                        set_hexpand: true,
                        set_spacing: 12,
                        add_css_class: "status",

                        gtk::Label {
                            set_label: &self.relays.iter().map(|u| u.domain().unwrap()).collect::<Vec<_>>().join("   "),
                            set_visible: !self.relays.is_empty(),
                            set_xalign: 1.0,
                            add_css_class: "relays",
                        },

                        gtk::Label {
                            set_label?: &self.event.client().as_ref().map(|c| format!("Sent by {c}")),
                            set_xalign: 1.0,
                            set_visible: self.event.client().is_some(),
                            add_css_class: "client",
                        }                    }
                },
                add_controller = gtk::EventControllerMotion::new() {
                    connect_enter[sender] => move |_, _, _| { sender.input(NoteInput::FocusIn) },
                    connect_leave[sender] => move |_| { sender.input(NoteInput::FocusOut) }
                }
            }
        }
    }

    menu! {
        author_menu: {
            "Copy pubkey as hex" => Copy(self.author.pubkey.to_string()),
            "Copy pubkey as bech32" => Copy(self.author.pubkey.to_bech32().unwrap()),
        },

        note_menu: {
            section! {
                "Copy event ID as hex" => Copy(self.event.id.to_hex()),
                "Copy event ID as bech32" => Copy(self.event.id.to_string())
            }
        }
    }

    fn forward_to_parent(output: Self::Output) -> Option<Self::ParentInput> {
        match output {
            NoteOutput::ShowDetails(details) => Some(LaneMsg::ShowDetails(details)),
            NoteOutput::LinkClicked(uri) => uri.parse().map(LaneMsg::LinkClicked).ok(),
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let tick_handle = relm4::spawn(async move {
            let mut int = tokio::time::interval(Duration::from_secs(30));
            loop {
                int.tick().await;
                sender.input(NoteInput::Tick);
            }
        });

        let replies = Replies::builder().launch(()).detach();
        let author = init
            .author
            .unwrap_or(Arc::new(Persona::new(init.event.pubkey)));
        let repost_author = init
            .repost
            .as_ref()
            .map(|r| r.author.clone().unwrap_or(Persona::new(r.event.pubkey)));
        let repost = init.repost.map(|r| r.event);

        Self {
            nip05_verified: author.nip05_preverified,
            author,
            is_central: init.is_central,
            content: init.event.prepare_content(),
            show_hidden_buttons: false,
            avatar: ANONYMOUS_USER.clone(),
            likes: 0,
            dislikes: 0,
            time: Utc
                .timestamp_opt(init.event.created_at.as_i64(), 0)
                .unwrap(),
            event: init.event,
            relays: init.relays,
            replies,
            repost_author,
            repost,
            age: String::new(),
            tick_handle
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: &Self::Root,
        _returned_widget: &gtk::ListBoxRow,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        if let Some(reposter) = &self.repost_author {
            relm4::view! {
                #[name = "reposter_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 10,
                    add_css_class: "repost",
                    set_visible: self.repost_author.is_some(),

                    Author::with_pubkey(reposter.pubkey) {
                        //TODO: Does the watch work here?
                        #[watch] set_persona?: &self.repost_author,
                        // set_context_menu: Some(&reposter_menu),
                        set_icon = &gtk::Image {
                            set_icon_name: Some("gnostique-repost-symbolic"),
                            set_pixel_size: 18,
                        }
                    }
                }
            }

            root.prepend(&reposter_box);
        }

        widgets
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            NoteInput::UpdatedProfile { author } => {
                if self.author.pubkey == author.pubkey {
                    self.author = author.clone();
                    self.nip05_verified = author.nip05_preverified;
                };
                self.content.provide(&author);
                self.replies.emit(RepliesInput::UpdatedProfile { author });
            }
            NoteInput::FocusIn => self.show_hidden_buttons = true,
            NoteInput::FocusOut => self.show_hidden_buttons = false,
            NoteInput::MetadataBitmap {
                pubkey,
                url,
                bitmap,
            } => {
                if self.author.pubkey == pubkey && self.author.avatar == Some(url) {
                    self.avatar = bitmap
                }
            }
            // NoteInput::Reply(event) => {
            // self.replies.emit(RepliesInput::NewReply(event));
            // }
            NoteInput::TextNote {
                event,
                relays,
                author,
                repost,
            } => self.receive(event, relays, author, repost),
            NoteInput::Nip05Verified(pubkey) => {
                if pubkey == self.author.pubkey {
                    self.nip05_verified = true;
                }
                self.replies.emit(RepliesInput::Nip05Verified(pubkey));
            }
            NoteInput::Reaction { event, reaction } => {
                if self.event.id == event {
                    if reaction == "+" || reaction == "🤙" {
                        self.likes += 1;
                    } else if reaction == "-" {
                        self.dislikes += 1;
                    }
                }
            }
            NoteInput::ShowDetails => {
                let event_json = match &self.repost {
                    Some(e) => serde_json::to_string_pretty(e).unwrap(),
                    None => serde_json::to_string_pretty(self.event.as_ref()).unwrap(),
                };
                let details = Details {
                    event_json,
                    metadata_json: Some(self.author.metadata_json.clone()),
                };
                sender.output(NoteOutput::ShowDetails(details));
            }
            NoteInput::Tick => self.age = self.format_age(),
        }
    }
}
