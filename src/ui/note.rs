use std::rc::Rc;

use chrono::{DateTime, TimeZone, Utc};
use gtk::gdk_pixbuf::Pixbuf;
use gtk::pango::WrapMode;
use gtk::prelude::*;
use nostr_sdk::nostr::prelude::TagKind;
use nostr_sdk::nostr::secp256k1::XOnlyPublicKey;
use nostr_sdk::nostr::*;
use nostr_sdk::sqlite::model::Profile;
use relm4::gtk;
use relm4::prelude::*;

use crate::lane::LaneMsg;

use super::details::Details;

/// Initial
pub struct NoteInit {
    pub event: Event,
    pub profile: Option<Profile>,
    pub is_central: bool,
}

#[derive(Debug)]
pub struct Note {
    content: String,
    is_central: bool,
    author_name: Option<String>,
    author_pubkey: XOnlyPublicKey,
    client: Option<String>,
    show_hidden_buttons: bool,
    event_json: String,
    metadata_json: Option<String>,
    avatar: Rc<Pixbuf>,
    pub time: DateTime<Utc>,
    pub event_id: Sha256Hash,
}

impl Note {
    /// Format author's pubkey according to context (has or has not author name).
    fn format_pubkey(&self) -> String {
        let chars = if self.author_name.is_some() { 8 } else { 16 };

        let s = self.author_pubkey.to_string();
        let (pre, tail) = s.split_at(chars);
        let (_, post) = tail.split_at(tail.len() - chars);
        format!("{pre}…{post}")
    }
}

#[derive(Debug)]
pub enum NoteInput {
    /// Author profile has some new data.
    UpdatedProfile {
        author_pubkey: XOnlyPublicKey,
        author_name: Option<String>,
        metadata_json: String,
    },
    /// The text note comes into focus.
    FocusIn,
    /// The text note loses focus.
    FocusOut,
    /// Show this note's details.
    ShowDetails,
    /// (New) avatar bitmap is available.
    AvatarBitmap(Rc<Pixbuf>),
}

#[derive(Debug)]
pub enum NoteOutput {
    ShowDetails(Details),
}

/*
    +-------------------------------------+
    |   AVATAR    |        AUTHOR         |
    |             +-----------------------+
    |             |        CONTENT        |
    +-------------+-----------------------+
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
                        set_from_pixbuf: Some(&self.avatar),
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
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,
                        add_css_class: "author",

                        gtk::Button {
                            #[watch]
                            set_label?: self.author_name.as_ref(),
                            #[watch]
                            set_visible: self.author_name.is_some(),
                            add_css_class: "author-name"
                        },

                        gtk::Label {
                            #[watch]
                            set_label: &self.format_pubkey(),
                            add_css_class: "author-pubkey"
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
                    add_css_class: "content"
                },

                // status
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    add_css_class: "status",

                    gtk::Label {
                        set_label?: &self.client.as_ref().map(|c| format!("Sent by {c}")),
                        set_xalign: 0.0,
                        set_visible: self.client.is_some(),
                        add_css_class: "client",
                    },

                    gtk::Label {
                        set_label: &self.time.to_string(),
                        set_hexpand: true,
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

    fn output_to_parent_input(output: Self::Output) -> Option<Self::ParentInput> {
        match output {
            NoteOutput::ShowDetails(details) => Some(LaneMsg::ShowDetails(details)),
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

        let client = init.event.tags.iter().find_map(|t| match t {
            Tag::Generic(TagKind::Custom(tag), s) if tag.as_str() == "client" => s.first().cloned(),
            _ => None,
        });

        Self {
            client,
            author_name: init.profile.and_then(|p| p.name),
            author_pubkey: init.event.pubkey,
            is_central: init.is_central,
            content: add_links(&init.event.content),
            show_hidden_buttons: false,
            event_json: serde_json::to_string_pretty(&init.event).unwrap(),
            metadata_json: None,
            avatar: Rc::new(Pixbuf::from_file("default-user-icon-8.jpg").unwrap()), // TODO: Share
            time: Utc.timestamp_opt(init.event.created_at as i64, 0).unwrap(),
            event_id: init.event.id,
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            NoteInput::UpdatedProfile {
                author_pubkey,
                author_name,
                metadata_json,
            } => {
                if self.author_pubkey == author_pubkey {
                    self.author_name = author_name;
                    self.metadata_json = Some(metadata_json);
                }
            }
            NoteInput::FocusIn => self.show_hidden_buttons = true,
            NoteInput::FocusOut => self.show_hidden_buttons = false,
            NoteInput::AvatarBitmap(pixbuf) => self.avatar = pixbuf,
            NoteInput::ShowDetails => {
                let details = Details {
                    event_json: self.event_json.clone(),
                    metadata_json: self.metadata_json.clone(),
                };
                sender.output(NoteOutput::ShowDetails(details));
            }
        }
    }
}

/// Detect URLs in given text and wrap them by `<a href="…">…</a>`.
fn add_links(content: &str) -> String {
    use linkify::*;

    LinkFinder::new()
        .spans(content)
        .map(|span| {
            let s = span.as_str();
            match span.kind() {
                Some(LinkKind::Url) => {
                    format!(r#"<a href="{s}">{s}</a>"#)
                }
                _ => s.to_string(),
            }
        })
        .collect()
}
