use std::rc::Rc;
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use gtk::gdk;
use gtk::pango::WrapMode;
use gtk::prelude::*;
use nostr_sdk::nostr::secp256k1::XOnlyPublicKey;
use nostr_sdk::nostr::*;
use nostr_sdk::prelude::ToBech32;
use relm4::component::{AsyncComponent, AsyncComponentController, AsyncController};
use relm4::prelude::*;

use super::author::Author;
use super::details::Details;
use super::replies::{Replies, RepliesInput};
use crate::app::action::*;
use crate::nostr::*;
use crate::ui::lane::LaneMsg;

/// Initial
pub struct NoteInit {
    pub event: Rc<Event>,
    pub relays: Vec<Url>,
    pub author: Option<Persona>,
    pub is_central: bool,
}

#[derive(Debug)]
pub struct Note {
    content: String,
    is_central: bool,
    author: Persona,
    show_hidden_buttons: bool,
    metadata_json: Option<Arc<String>>,
    avatar: Arc<gdk::Texture>,
    likes: u32,
    dislikes: u32,
    pub time: DateTime<Utc>,
    event: Rc<Event>,
    relays: Vec<Url>,
    replies: AsyncController<Replies>,
}

#[derive(Clone, Debug)]
pub enum NoteInput {
    /// Author profile has some new data.
    UpdatedProfile {
        author: Persona,
    },
    /// The text note comes into focus.
    FocusIn,
    /// The text note loses focus.
    FocusOut,
    /// Show this note's details.
    ShowDetails,
    /// (New) avatar bitmap is available.
    MetadataBitmap {
        pubkey: XOnlyPublicKey,
        url: Url,
        bitmap: Arc<gdk::Texture>,
    },
    Reaction {
        event: Sha256Hash,
        reaction: String,
    },
    Reply(Rc<Event>),
    Nip05Verified(XOnlyPublicKey),
}

#[derive(Debug)]
pub enum NoteOutput {
    ShowDetails(Details),
    LinkClicked(String),
}

/*
    +-------------------------------------+
    |   AVATAR    |        AUTHOR         |
    |             +-----------------------+
    |             |        CONTENT        |
    +             +-----------------------+
    |             |       REACTIONS       |
    +             +-----------------------+
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
            set_orientation: gtk::Orientation::Horizontal,
            set_hexpand: true,
            add_css_class: "text-note",
            add_css_class: if self.is_central { "central" } else { "text-note" },

            // left column
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                // avatar
                gtk::Box {
                    add_css_class: "avatar",

                    gtk::Image {
                        #[watch]
                        set_from_paintable: Some(self.avatar.as_ref()),
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Start,
                    }
                }
            },

            // right column
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_hexpand: true,
                set_spacing: 10,
                add_css_class: "right-column",

                // author (template widget?)
                gtk::Overlay {
                    #[template]
                    #[name(author)]
                    Author {
                        #[watch]
                        set_tooltip_markup: Some(&self.author.tooltip()),

                        #[template_child]
                        author_name {
                            #[watch] set_label?: self.author.name.as_ref(),
                            #[watch] set_visible: self.author.name.is_some(),
                        },
                        #[template_child]
                        author_pubkey {
                            #[watch] set_label: &self.author.format_pubkey(8, 16),
                            #[watch] set_visible: !self.author.show_nip05(),
                        },
                        #[template_child]
                        author_nip05 {
                            #[watch] set_label?: &self.author.format_nip05(),
                            #[watch] set_visible: self.author.show_nip05(),
                        },

                        add_controller = &gtk::GestureClick::new() {
                            set_button: 3,
                            connect_pressed[author] => move |_, _, x, y| {
                                let popover = gtk::PopoverMenu::builder()
                                    .menu_model(&author_menu)
                                    .has_arrow(false)
                                    .pointing_to(&gdk::Rectangle::new(x as i32, y as i32, 1, 1))
                                    .build();

                                popover.set_parent(author.widget_ref());
                                popover.popup();
                            }
                        }
                    },
                    add_overlay = &gtk::Box {
                        set_valign: gtk::Align::Start,
                        set_halign: gtk::Align::End,
                        #[watch]
                        set_visible: self.show_hidden_buttons,
                        add_css_class: "hidden-buttons",

                        gtk::Button {
                            set_label: "src",
                            connect_clicked => NoteInput::ShowDetails
                        }
                    }
                },

                #[name(content)]
                gtk::Label {
                    #[watch]
                    set_markup: &self.content,
                    set_wrap: true,
                    set_wrap_mode: WrapMode::WordChar,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Start,
                    set_vexpand: true,
                    set_xalign: 0.0,
                    set_selectable: true,
                    add_css_class: "content",

                    connect_activate_link[sender] => move |_, uri| {
                        if uri.starts_with("nostr") {
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
                        &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 4,
                            set_halign: gtk::Align::Start,

                            gtk::Label {
                                set_label: "♥"
                            },

                            gtk::Label {
                                #[watch]
                                set_label: &self.likes.to_string(),
                                #[watch]
                                set_visible: self.likes > 0
                            }
                        },
                    attach[2, 1, 1, 1] =
                        &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 4,
                            set_halign: gtk::Align::Start,

                            gtk::Label {
                                set_label: "👎"
                            },

                            gtk::Label {
                                #[watch]
                                set_label: &self.dislikes.to_string(),
                                #[watch]
                                set_visible: self.dislikes > 0,

                            }
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
                    },

                    gtk::Label {
                        set_label: &self.time.to_string(),
                        set_xalign: 1.0,
                        add_css_class: "time",
                    }
                }
            },
            add_controller = &gtk::EventControllerMotion::new() {
                connect_enter[sender] => move |_, _, _| { sender.input(NoteInput::FocusIn) },
                connect_leave[sender] => move |_| { sender.input(NoteInput::FocusOut) }
            }
        }
    }

    menu! {
        author_menu: {
            "Copy pubkey as hex" => Copy(self.author.pubkey.to_string()),
            "Copy pubkey as bech32" => Copy(self.author.pubkey.to_bech32().unwrap()),
        }
    }

    fn output_to_parent_input(output: Self::Output) -> Option<Self::ParentInput> {
        match output {
            NoteOutput::ShowDetails(details) => Some(LaneMsg::ShowDetails(details)),
            NoteOutput::LinkClicked(uri) => uri.parse().map(LaneMsg::LinkClicked).ok(),
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(include_bytes!("text_note.css"));
        gtk::StyleContext::add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let replies = Replies::builder().launch(()).detach();
        let author = init.author.unwrap_or(Persona::new(init.event.pubkey));

        Self {
            author,
            is_central: init.is_central,
            content: init.event.augment_content(),
            show_hidden_buttons: false,
            metadata_json: None,
            avatar: ANONYMOUS_USER.clone(),
            likes: 0,
            dislikes: 0,
            time: Utc.timestamp_opt(init.event.created_at as i64, 0).unwrap(),
            event: init.event,
            relays: init.relays,
            replies,
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            NoteInput::UpdatedProfile { author } => {
                if self.author.pubkey == author.pubkey {
                    self.author = author.clone();
                };

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
            NoteInput::Reply(event) => {
                self.replies.emit(RepliesInput::NewReply(event));
            }
            NoteInput::Nip05Verified(pubkey) => {
                if pubkey == self.author.pubkey {
                    self.author.nip05_verified = true;
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
                let details = Details {
                    event_json: serde_json::to_string_pretty(self.event.as_ref()).unwrap(),
                    metadata_json: self.metadata_json.clone(),
                };
                sender.output(NoteOutput::ShowDetails(details));
            }
        }
    }
}
