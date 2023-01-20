use std::collections::HashSet;
use std::rc::Rc;

use gtk::prelude::*;
use nostr_sdk::nostr::{Event, Sha256Hash};
use relm4::factory::FactoryVecDeque;
use relm4::gtk;
use relm4::prelude::*;

#[derive(Debug)]
pub struct Replies {
    reply_hashes: HashSet<Sha256Hash>,
    replies: FactoryVecDeque<Reply>,
}

#[derive(Debug)]
pub enum RepliesInput {
    NewReply(Rc<Event>),
}

#[relm4::component(pub)]
impl SimpleComponent for Replies {
    type Input = RepliesInput;
    type Output = ();
    type Init = ();

    #[rustfmt::skip]
    view! {
        gtk::Box {
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
        }
    }
}

#[derive(Debug)]
pub struct Reply {
    content: String,
}

#[derive(Debug)]
pub enum ReplyInput {}

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
        }
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {
            ReplyInput => {}
        }
    }
}
