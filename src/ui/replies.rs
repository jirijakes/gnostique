use std::collections::HashSet;
use std::sync::Arc;

use gtk::prelude::*;
use nostr_sdk::nostr::{Event, EventId};
use nostr_sdk::prelude::XOnlyPublicKey;
use relm4::component::{AsyncComponentParts, SimpleAsyncComponent};
use relm4::factory::{AsyncFactoryComponent, AsyncFactoryVecDeque};
use relm4::prelude::*;
use relm4::{gtk, AsyncComponentSender, AsyncFactorySender};

use crate::nostr::Persona;
use crate::ui::widgets::author::Author;

/// Widget displaying list of replies to a text note.
#[derive(Debug)]
pub struct Replies {
    reply_hashes: HashSet<EventId>,
    replies: AsyncFactoryVecDeque<Reply>,
}

#[derive(Debug)]
pub enum RepliesInput {
    NewReply(Arc<Event>),
    UpdatedProfile { author: Arc<Persona> },
    Nip05Verified(XOnlyPublicKey),
}

#[relm4::component(async pub)]
impl SimpleAsyncComponent for Replies {
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

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let replies = AsyncFactoryVecDeque::new(
            gtk::Box::new(gtk::Orientation::Vertical, 10),
            sender.input_sender(),
        );
        let model = Replies {
            reply_hashes: Default::default(),
            replies,
        };
        let replies_list = model.replies.widget();
        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, message: Self::Input, _sender: AsyncComponentSender<Self>) {
        match message {
            RepliesInput::NewReply(event) => {
                if self.reply_hashes.insert(event.id) {
                    self.replies.guard().push_back(event);
                }
            }
            RepliesInput::UpdatedProfile { author } => self
                .replies
                .broadcast(ReplyInput::UpdatedProfile { author }),
            RepliesInput::Nip05Verified(pubkey) => {
                self.replies.broadcast(ReplyInput::Nip05Verified(pubkey))
            }
        }
    }
}

/// Widget display one of replies to a text note.
#[derive(Debug)]
pub struct Reply {
    content: String,
    author: Arc<Persona>,
    nip05_verified: bool,
}

#[derive(Clone, Debug)]
pub enum ReplyInput {
    UpdatedProfile { author: Arc<Persona> },
    Nip05Verified(XOnlyPublicKey),
}

#[relm4::factory(async pub)]
impl AsyncFactoryComponent for Reply {
    type Init = Arc<Event>;
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

            Author::with_pubkey(self.author.pubkey) {
                #[watch] set_persona: &self.author,
                #[watch] set_nip05_verified: self.nip05_verified,
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

    async fn init_model(
        init: Self::Init,
        _index: &DynamicIndex,
        _sender: AsyncFactorySender<Self>,
    ) -> Self {
        Reply {
            content: init.as_ref().content.to_string(),
            author: Arc::new(Persona::new(init.pubkey)),
            nip05_verified: false,
        }
    }

    async fn update(&mut self, message: Self::Input, _sender: AsyncFactorySender<Self>) {
        match message {
            ReplyInput::UpdatedProfile { author } => {
                if self.author.pubkey == author.pubkey {
                    self.nip05_verified = author.nip05_preverified;
                    self.author = author;
                }
            }
            ReplyInput::Nip05Verified(pubkey) => {
                if pubkey == self.author.pubkey {
                    self.nip05_verified = true;
                }
            }
        }
    }
}
