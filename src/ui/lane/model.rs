use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use gtk::gdk;
use nostr_sdk::nostr::secp256k1::XOnlyPublicKey;
use nostr_sdk::nostr::{Event, EventId};
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use reqwest::Url;
use tracing::trace;

use crate::follow::Follow;
use crate::nostr::content::DynamicContent;
use crate::nostr::{EventExt, Persona, Repost, TextNote};
use crate::ui::details::Details;
use crate::ui::lane_header::LaneHeader;
use crate::ui::note::{Note, NoteInit};
use crate::ui::profilebox::model::Profilebox;

#[derive(Debug)]
pub struct Lane {
    pub(super) kind: LaneKind,
    pub(super) text_notes: FactoryVecDeque<Note>,
    pub(super) hash_index: HashMap<EventId, DynamicIndex>,
    pub(super) profile_box: Controller<Profilebox>,
    pub(super) header: Controller<LaneHeader>,
}

#[derive(Clone, Debug)]
pub enum LaneKind {
    Profile(XOnlyPublicKey),
    Thread(EventId),
    Feed(Follow),
    Sink,
}

impl LaneKind {
    pub fn is_thread(&self, event_id: &EventId) -> bool {
        matches!(self, LaneKind::Thread(e) if e == event_id)
    }

    pub fn is_profile(&self, pubkey: &XOnlyPublicKey) -> bool {
        matches!(self, LaneKind::Profile(p) if p == pubkey)
    }

    pub fn is_a_profile(&self) -> bool {
        matches!(self, LaneKind::Profile(_))
    }

    pub fn accepts(&self, event: &Event) -> bool {
        match self {
            LaneKind::Sink => true,
            LaneKind::Feed(f) => f.follows(&event.pubkey) && event.replies_to().is_none(),
            LaneKind::Profile(pubkey) => &event.pubkey == pubkey,
            LaneKind::Thread(id) => {
                event.id == *id
                    || event.replies_to() == Some(*id)
                    || matches!(event.thread_root(), Some((i, _)) if i == *id)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum LaneMsg {
    NewTextNote {
        note: TextNote,
        content: Arc<DynamicContent>,
        relays: Vec<Url>,
        repost: Option<Repost>,
    },
    UpdatedProfile {
        author: Arc<Persona>,
    },
    ShowDetails(Details),
    MetadataBitmap {
        pubkey: XOnlyPublicKey,
        url: Url,
        bitmap: Arc<gdk::Texture>,
    },
    Reaction {
        event: EventId,
        reaction: String,
    },
    Nip05Verified(XOnlyPublicKey),
    LinkClicked(Url),
}

#[derive(Debug)]
pub enum LaneOutput {
    ShowDetails(Details),
    WriteNote,
}

impl Lane {
    /// New text note was received, let's handle it.
    pub(super) fn text_note_received(
        &mut self,
        note: TextNote,
        content: Arc<DynamicContent>,
        relays: Vec<Url>,
        repost: Option<Repost>,
    ) {
        let event_id = note.event().id;

        // Add note iff it has not been added yet (they may arrive multiple times).
        if !self.hash_index.contains_key(&event_id) {
            let is_central = self.kind.is_thread(&event_id);
            let event_time = note.event().created_at;

            let init = NoteInit {
                note,
                content,
                relays,
                is_central,
                repost,
            };

            let di = if is_central {
                // Central text note always goes first.
                self.text_notes.guard().push_front(init)
            } else {
                // Find index of first text note that was created later
                // than the text note being inserted.
                let idx = self.text_notes.iter().position(|tn| {
                    let ord = tn.time.timestamp().cmp(&event_time.as_i64());
                    match self.kind {
                        LaneKind::Profile(_) => ord == Ordering::Greater,
                        LaneKind::Thread(_) => ord == Ordering::Less,
                        LaneKind::Feed(_) => ord == Ordering::Less,
                        LaneKind::Sink => ord == Ordering::Less,
                    }
                });

                if let Some(idx) = idx {
                    // Inserting somewhere in the middle.
                    self.text_notes.guard().insert(idx, init)
                } else {
                    // Appending to the end.
                    self.text_notes.guard().push_back(init)
                }
            };

            // At the end, let's remember (event_id -> dynamic index) pair.
            self.hash_index.insert(event_id, di);
        }

        // Remove oldest notes if there are too many already.
        //
        // TODO: The maximum number of notes to show should be configurable.
        {
            let mut g = self.text_notes.guard();

            if g.len() > 10 {
                trace!("Lane {:?} has {} notes, removing some.", self.kind, g.len());
            };

            while g.len() > 10 {
                let _ = g.pop_back();
            }
        }
    }
}
