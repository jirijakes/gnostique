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
    pub(super) event: Arc<Event>,
    pub(super) relays: Vec<Url>,
    pub(super) replies: AsyncController<Replies>,
    pub(super) repost_author: Option<Persona>,
    pub(super) repost: Option<Event>,
    pub(super) age: String,
}

impl Note {
    pub(super) fn receive(
        &mut self,
        event: Arc<Event>,
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

    /// Generates textual representation of the age of this text note. It is
    /// relatively fuzzy and serves to inform reader about the rough duration
    /// since the note was broadcast.
    pub(super) fn format_age(&self) -> String {
        use chrono::*;

        let created_at =
            NaiveDateTime::from_timestamp_opt(self.event.created_at.as_i64(), 0).unwrap();
        let duration = Utc::now().naive_utc().signed_duration_since(created_at);

        if duration.num_weeks() > 0 {
            // If the duration is a week or more, let's show date instead of
            // duration since creation.
            let utc = DateTime::<Utc>::from_utc(created_at, Utc);
            let local = utc.with_timezone(&Local);

            if local.year() == Local::now().year() {
                // It's this year ⇒ just day and month
                local.format("%e %b").to_string()
            } else {
                // It's before this year ⇒ day and month and year
                local.format("&e %b %Y").to_string()
            }
        } else if duration.num_days() > 0 {
            format!("{}d", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{}h", duration.num_hours())
        } else if duration.num_minutes() > 0 {
            format!("{}m", duration.num_minutes())
        } else {
            "< 1m".to_string()
        }
    }

    /// Generates tooltip for note age indicator. It always shows precise time.
    pub(super) fn format_age_tooltip(&self) -> String {
        let format = "%A, %e %B %Y, %T";
        let local = self.time.with_timezone(&chrono::Local).format(format);
        let utc = self.time.format(format);

        format!("<b>Local:</b> {local}\n<b>UTC:</b> {utc}")
    }
}
