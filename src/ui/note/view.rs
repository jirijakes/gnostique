use std::time::Duration;

use chrono::{TimeZone, Utc};
use gtk::pango::WrapMode;
use gtk::prelude::*;
use nostr_sdk::prelude::ToBech32;
use relm4::component::AsyncComponentController;
use relm4::prelude::*;

use super::model::*;
use super::msg::*;
use super::quote::Quote;
use crate::app::action::*;
use crate::nostr::*;
use crate::ui::details::Details;
use crate::ui::lane::LaneMsg;
use crate::ui::replies::RepliesInput;
use crate::ui::widgets::author::Author;

/*
    +-------------------------------------+
    | [REPOST]       0 0 2 1              |
    +-------------------------------------+
    |   AVATAR    |        AUTHOR         |   1 1 1 1
    |             +-----------------------+
    |  0 1 1 6    |        CONTENT        |   1 2 1 1
    |             +-----------------------+
    |             |        [QUOTE]        |   1 3 1 1
    |             +-----------------------+
    |             |       [REPLIES]       |   1 4 1 1
    |             +-----------------------+
    |             |       REACTIONS       |   1 5 1 1 
    |             +-----------------------+
    |             |        STATUS         |   1 6 1 1
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
        #[name = "root"]
        gtk::Grid {
            add_css_class: "text-note",
            add_css_class: if self.is_central { "central" } else { "text-note" },
            set_column_spacing: 6,
            set_row_spacing: 6,

            // here be REPOSTER
            
            // AVATAR
            attach[0, 1, 1, 6] = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_visible: !self.is_profile,
                add_css_class: "avatar",
                
                gtk::Image {
                    #[watch] set_from_paintable: Some(self.avatar.as_ref()),
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
            },

            // AUTHOR
            attach[1, 1, 1, 1] = &gtk::Overlay {
                Author::with_pubkey(self.author.pubkey) {
                    set_visible: !self.is_profile,
                    set_context_menu: Some(&author_menu),
                    #[watch] set_persona: &self.author,
                    #[watch] set_nip05_verified: self.nip05_verified,
                    connect_clicked[sender, author = self.author.clone(), relays = self.relays.clone()] => move |_| {
                        let r = relays.first().unwrap().clone();
                        sender.output(NoteOutput::OpenProfile(author.clone(), r));
                    }
                },
                add_overlay = &gtk::Label {
                    set_valign: gtk::Align::Start,
                    set_halign: gtk::Align::End,
                    set_tooltip_markup: Some(&self.format_age_tooltip()),
                    add_css_class: "note-age",
                    #[watch] set_label: &self.age,
                }
            },

            // CONTENT
            attach[1, 2, 1, 1]: content = &gtk::Label {
                #[watch] set_markup: self.content.augment(&html_escape::encode_text(&self.event.content)).trim(),
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

            // here be QUOTES
                    
            // here be REPLIES

            // REACTIONS
            attach[1, 5, 1, 1]: reactions = &gtk::Grid {
                set_widget_name: "reactions",
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
            attach[1, 6, 1, 1] = &gtk::Box {
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
                }
            },
            add_controller = gtk::EventControllerMotion::new() {
                connect_enter[sender] => move |_, _, _| { sender.input(NoteInput::FocusIn) },
                connect_leave[sender] => move |_| { sender.input(NoteInput::FocusOut) }
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
                "Copy event ID as bech32" => Copy(self.event.id.to_bech32().unwrap())
            }
        }
    }

    fn forward_to_parent(output: Self::Output) -> Option<Self::ParentInput> {
        match output {
            NoteOutput::ShowDetails(details) => Some(LaneMsg::ShowDetails(details)),
            NoteOutput::LinkClicked(uri) => uri.parse().map(|u| LaneMsg::LinkClicked(u)).ok(),
            NoteOutput::OpenProfile(persona, relay) => Some(LaneMsg::OpenProfile(persona, relay))
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

        // TODO: Now we make only one of the referenced notes a quote.
        // Perhaps we could show all of them somehow?
        let quote = init.referenced_notes
            .into_iter()
            .next()
            .map(|q| Quote::builder().launch(q).detach());
        
        let (event, author) = init.note.underlying();

        Note {
            nip05_verified: author.nip05_preverified,
            author,
            is_central: init.is_central,
            is_profile: init.is_profile,
            content: (*init.content).clone(),
            show_hidden_buttons: false,
            avatar: ANONYMOUS_USER.clone(),
            likes: 0,
            dislikes: 0,
            time: Utc.timestamp_opt(event.created_at.as_i64(), 0).unwrap(),
            event,
            relays: init.relays,
            replies: None,
            repost: init.repost,
            quote,
            age: String::new(),
            tick_handle,
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        _root: &Self::Root,
        _returned_widget: &gtk::ListBoxRow,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        if let Some(repost) = &self.repost {
            relm4::view! {
                #[name = "reposter_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 10,
                    add_css_class: "repost",
                    // set_visible: self.repost_author.is_some(),

                    Author::with_pubkey(repost.author().pubkey) {
                        //TODO: Does the watch work here?
                        #[watch] set_persona: repost.author(),
                        // set_context_menu: Some(&reposter_menu),
                        set_icon = &gtk::Image {
                            set_icon_name: Some("gnostique-repost-symbolic"),
                            set_pixel_size: 18,
                        }
                    }
                }
            }

            widgets.root.attach(&reposter_box, 0, 0, 2, 1);
        }
        
        // If controller for Quote has been created in `init_model`,
        // add its widget to the note. If note and a quote arrives
        // later, the controller will be created in note::model::receive.
        if let Some(quote) = &self.quote {
            widgets.root.attach(quote.widget(), 1, 3, 1, 1);
        }

        widgets
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: FactorySender<Self>)
    {
        match message {
            NoteInput::UpdatedProfile { author } => {
                if self.author.pubkey == author.pubkey {
                    self.author = author.clone();
                    self.nip05_verified = author.nip05_preverified;
                };
                self.content.provide(&author);
                if let Some(replies)  = &self.replies {                
                    replies.emit(RepliesInput::UpdatedProfile { author });
                };
            }
            NoteInput::FocusIn => self.show_hidden_buttons = true,
            NoteInput::FocusOut => self.show_hidden_buttons = false,
            NoteInput::MetadataBitmap {
                pubkey,
                url,
                bitmap,
            } => {
                if self.author.pubkey == pubkey && self.author.avatar == Some(url) {
                    self.avatar = bitmap;
                }
            }
            NoteInput::TextNote {
                note,
                content: _, // NOTE: I guess someday we will need it to augment replies, quotes etc.
                relays,
                repost,
                ..                
            } => self.receive(widgets, note, relays, repost),
            NoteInput::Nip05Verified(pubkey) => {
                if pubkey == self.author.pubkey {
                    self.nip05_verified = true;
                }
                if let Some(replies) = &self.replies {
                    replies.emit(RepliesInput::Nip05Verified(pubkey));
                };
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
                    Some(repost) => serde_json::to_string_pretty(repost.event()).unwrap(),
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

        self.update_view(widgets, sender);
    }
}
