use std::collections::HashSet;
use std::rc::Rc;

use gtk::prelude::*;
use nostr_sdk::nostr::{Event, Sha256Hash};
use nostr_sdk::prelude::XOnlyPublicKey;
use relm4::factory::FactoryVecDeque;
use relm4::gtk;
use relm4::prelude::*;

use super::author::Author;

/// Widget displaying list of replies to a text note.
#[derive(Debug)]
pub struct Replies {
    reply_hashes: HashSet<Sha256Hash>,
    replies: FactoryVecDeque<Reply>,
}

#[derive(Debug)]
pub enum RepliesInput {
    NewReply(Rc<Event>),
    UpdatedProfile {
        author_pubkey: XOnlyPublicKey,
        author_name: Option<String>
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Replies {
    type Input = RepliesInput;
    type Output = ();
    type Init = ();

    #[rustfmt::skip]
    view! {
        gtk::Box {
            add_css_class: "replies",
            #[watch]
            set_visible: !model.replies.is_empty(),

            #[local_ref]
            replies_list -> gtk::Box { }
        }
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let replies = FactoryVecDeque::new(
            gtk::Box::new(gtk::Orientation::Vertical, 10),
            sender.input_sender(),
        );
        let model = Replies {
            reply_hashes: Default::default(),
            replies,
        };
        let replies_list = model.replies.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            RepliesInput::NewReply(event) => {
                if self.reply_hashes.insert(event.id) {
                    self.replies.guard().push_back(event);
                }
            }
            RepliesInput::UpdatedProfile { author_pubkey, author_name } => {
                for i in 0..self.replies.len() {
                    self.replies.send(
                        i,
                        ReplyInput::UpdatedProfile {
                            author_pubkey,
                            author_name: author_name.clone()
                        }
                    );
                }
            }
        }
    }
}

/// Widget display one of replies to a text note.
#[derive(Debug)]
pub struct Reply {
    content: String,
    author_name: Option<String>,
    author_pubkey: XOnlyPublicKey,
}

impl Reply {
    /// Format author's pubkey according to context (has or has not author name).
    fn format_pubkey(&self) -> String {
        let chars = 8;

        let s = self.author_pubkey.to_string();
        let (pre, tail) = s.split_at(chars);
        let (_, post) = tail.split_at(tail.len() - chars);
        format!("{pre}…{post}")
    }
}

#[derive(Debug)]
pub enum ReplyInput {
    UpdatedProfile {
        author_pubkey: XOnlyPublicKey,
        author_name: Option<String>
    }    
}

#[relm4::factory(pub)]
impl FactoryComponent for Reply {
    type Init = Rc<Event>;
    type Input = ReplyInput;
    type Output = ();
    type CommandOutput = ();
    type ParentInput = RepliesInput;
    type ParentWidget = gtk::Box;

    #[rustfmt::skip]
    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,
            add_css_class: "reply",

            #[template]
            Author {
                #[template_child]
                author_name {
                    #[watch]
                    set_label?: self.author_name.as_ref(),
                    #[watch]
                    set_visible: self.author_name.is_some(),
                },

                #[template_child]
                author_pubkey {
                    #[watch]
                    set_label: &self.format_pubkey(),
                }

            },

            gtk::Label {
                #[watch]
                set_label: &self.content,
                set_wrap: true,
                set_wrap_mode: gtk::pango::WrapMode::Word,
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Start,
                set_xalign: 0.0,
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Reply {
            content: init.as_ref().content.to_string(),
            author_pubkey: init.pubkey,
            author_name: None
        }
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {
            ReplyInput::UpdatedProfile { author_pubkey, author_name } => {
                if self.author_pubkey == author_pubkey {
                    self.author_name = author_name;
                }
            }
        }
    }
}
