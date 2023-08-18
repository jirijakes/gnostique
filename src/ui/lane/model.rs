use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use gtk::gdk;
use nostr_sdk::nostr::secp256k1::XOnlyPublicKey;
use nostr_sdk::nostr::{Event, EventId};
use nostr_sdk::Tag;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use reqwest::Url;
use tracing::trace;

use crate::follow::Follow;
use crate::nostr::content::DynamicContent;
use crate::nostr::subscriptions::Subscription;
use crate::nostr::{EventExt, Persona, Repost, TextNote};
use crate::ui::details::Details;
use crate::ui::lane_header::LaneHeader;
use crate::ui::link::InternalLink;
use crate::ui::note::{Note, NoteInit};
use crate::ui::profilebox::model::Profilebox;

#[derive(Debug)]
pub struct Lane {
    pub(super) kind: LaneKind,
    pub(super) index: DynamicIndex,
    pub(super) text_notes: FactoryVecDeque<Note>,
    pub(super) hash_index: HashMap<EventId, DynamicIndex>,
    /// Component of profile box; exists only when the lane
    /// is of kind Profile.
    pub(super) profile_box: Option<Controller<Profilebox>>,
    pub(super) header: Controller<LaneHeader>,
}

#[derive(Clone, Debug)]
pub enum LaneKind {
    Profile(Arc<Persona>, Url),
    Thread(EventId),
    Feed(Follow),
    Subscription(Subscription), // TODO: perhaps more general Search?
    Sink,
}

impl LaneKind {
    pub fn is_thread(&self, event_id: &EventId) -> bool {
        matches!(self, LaneKind::Thread(e) if e == event_id)
    }

    pub fn is_profile(&self, pubkey: &XOnlyPublicKey) -> bool {
        matches!(self, LaneKind::Profile(p, _) if &p.pubkey == pubkey)
    }

    pub fn is_a_profile(&self) -> bool {
        matches!(self, LaneKind::Profile(_, _))
    }

    pub fn accepts(&self, event: &Event) -> bool {
        match self {
            LaneKind::Sink => true,
            LaneKind::Subscription(sub) => LaneKind::accepts_subscription(event, sub),
            LaneKind::Feed(f) => f.follows(&event.pubkey) && event.replies_to().is_none(),
            LaneKind::Profile(p, _) => event.pubkey == p.pubkey,
            LaneKind::Thread(id) => {
                event.id == *id
                    || event.replies_to() == Some(*id)
                    || matches!(event.thread_root(), Some((i, _)) if i == *id)
            }
        }
    }

    /// Determines whether the incoming `event` is going to be placed in this lane.
    /// Gradually, it will cover all cases and at the end will replace lane kind.
    fn accepts_subscription(event: &Event, subscription: &Subscription) -> bool {
        let tags = subscription
            .hashtags()
            .iter()
            .map(|t| t.to_lowercase())
            .collect::<HashSet<_>>();

        // TODO: could also consider content of the text note, not only event.tags.
        let accepts_tags = event
            .tags
            .iter()
            .any(|t| matches!(t, Tag::Hashtag(h) if tags.contains(h.to_lowercase().as_str())));

        let accept_pubkeys = subscription.pubkeys().contains(&event.pubkey);

        accepts_tags || accept_pubkeys
    }
}

#[derive(Clone, Debug)]
pub enum LaneMsg {
    NewTextNote {
        note: TextNote,
        content: Arc<DynamicContent>,
        relays: Vec<Url>,
        repost: Option<Repost>,
        referenced_notes: HashSet<TextNote>,
        referenced_profiles: HashSet<Persona>,
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
    LinkClicked(InternalLink),
    CloseLane,
}

#[derive(Debug)]
pub enum LaneOutput {
    ShowDetails(Details),
    WriteNote,
    DemandProfile(XOnlyPublicKey, Url),
    CloseLane(DynamicIndex),
    LinkClicked(InternalLink),
}

impl Lane {
    /// New text note was received, let's handle it.
    pub(super) fn text_note_received(
        &mut self,
        note: TextNote,
        content: Arc<DynamicContent>,
        relays: Vec<Url>,
        repost: Option<Repost>,
        referenced_notes: HashSet<TextNote>,
        referenced_profiles: HashSet<Persona>,
    ) {
        let event_id = note.event().id;

        // Add note iff it has not been added yet (they may arrive multiple times).
        if let Entry::Vacant(e) = self.hash_index.entry(event_id) {
            let is_central = self.kind.is_thread(&event_id);
            let is_profile = self.kind.is_a_profile();
            let event_time = note.event().created_at;

            let init = NoteInit {
                note,
                content,
                relays,
                is_central,
                is_profile,
                repost,
                referenced_notes,
                referenced_profiles,
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
                        LaneKind::Profile(_, _) => ord == Ordering::Less,
                        LaneKind::Thread(_) => ord == Ordering::Less,
                        LaneKind::Feed(_) => ord == Ordering::Less,
                        LaneKind::Sink => ord == Ordering::Less,
                        LaneKind::Subscription(_) => ord == Ordering::Less,
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
            e.insert(di);
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

    /// Returns a subscription of this lane, if it exists.
    // TODO: Eventually, every lane should have a subscription, so
    // there will be no need for Option anymore.
    pub fn subscription(&self) -> Option<&Subscription> {
        match &self.kind {
            LaneKind::Subscription(s) => Some(s),
            _ => None,
        }
    }
}
