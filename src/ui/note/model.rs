use std::rc::Rc;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use gtk::gdk;
use nostr_sdk::nostr::*;
use relm4::component::{AsyncComponentController, AsyncController};
use relm4::prelude::*;

use crate::nostr::*;
use crate::ui::replies::{Replies, RepliesInput};

#[derive(Debug)]
pub struct Note {
    pub(super) content: String,
    pub(super) is_central: bool,
    pub(super) author: Persona,
    pub(super) show_hidden_buttons: bool,
    pub(super) avatar: Arc<gdk::Texture>,
    pub(super) likes: u32,
    pub(super) dislikes: u32,
    pub time: DateTime<Utc>,
    pub(super) event: Rc<Event>,
    pub(super) relays: Vec<Url>,
    pub(super) replies: AsyncController<Replies>,
    pub(super) repost_author: Option<Persona>,
    pub(super) repost: Option<Event>,
}

impl Note {
    pub(super) fn receive(
        &mut self,
        event: Rc<Event>,
        relays: Vec<Url>,
        author: Option<Persona>,
        repost: Option<Repost>,
    ) {
        // The newly arriving event is this text note. Assuming that
        // it's all more up-to-date, so we can update notes state right away.
        if event.id == self.event.id {
            // update relays
            self.relays = relays;

            // update author
            if let Some(a) = author {
                self.author = a;
            }
        }

        if let Some(r) = repost {};

        if event.replies_to() == Some(self.event.id) {
            // The newly arriving event is a reply to this text note.
            self.replies.emit(RepliesInput::NewReply(event.clone()));
        }

        if let Some((root, root_relay)) = self.event.thread_root() {
            if root == event.id {
                // The newly arriving event is thread root of this text note.
                // println!(">>>>>> {:#?}", event);
            }
        }
    }
}
