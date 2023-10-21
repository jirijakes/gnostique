use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use gtk::gdk;
use nostr_sdk::nostr::secp256k1::XOnlyPublicKey;
use nostr_sdk::nostr::EventId;
use nostr_sdk::Url;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use tracing::trace;

use crate::nostr::content::DynamicContent;
use crate::nostr::preview::Preview;
use crate::nostr::subscriptions::Subscription;
use crate::nostr::{EventRef, Persona, Repost, TextNote};
use crate::ui::details::Details;
use crate::ui::lane_header::LaneHeader;
use crate::ui::link::InternalLink;
use crate::ui::note::{Note, NoteInit};
use crate::ui::profilebox::model::Profilebox;

#[derive(Debug)]
pub struct Lane {
    /// Subscription of this lane. The lane will display
    /// all the notes and other stuff as specified by
    /// the subscription.
    pub(super) subscription: Subscription,

    /// Event in the lane that the lane should be focused on.
    /// Typically if the lane is subscribed to a single event,
    /// it will be focused.
    pub(super) focused: Option<EventId>,

    /// Dynamic index of this lane.
    pub(super) index: DynamicIndex,

    /// All the notes currently displayed in the lane.
    pub(super) text_notes: FactoryVecDeque<Note>,

    pub(super) hash_index: HashMap<EventId, DynamicIndex>,

    /// Component of profile box; exists only when the lane
    /// is of kind Profile.
    pub(super) profile_box: Option<Controller<Profilebox>>,

    pub(super) header: Controller<LaneHeader>,
}

/// Initial object of a new lane.
pub struct LaneInit {
    /// Initial subscription of the lane.
    pub(super) subscription: Subscription,

    /// Focused event, if any.
    pub(super) focused: Option<EventId>,
}

impl LaneInit {
    /// Creates new Lane initial for given subscription.
    /// No event is focused.
    pub fn subscription(subscription: Subscription) -> LaneInit {
        LaneInit {
            subscription,
            focused: None,
        }
    }

    pub fn with_focused(subscription: Subscription, focused: EventId) -> LaneInit {
        LaneInit {
            subscription,
            focused: Some(focused),
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
        referenced_notes: HashSet<TextNote>,
        referenced_profiles: HashSet<Persona>,
    },
    UpdatedProfile {
        author: Arc<Persona>,
    },
    ShowDetails(Details),
    Preview(Preview),
    MetadataBitmap {
        pubkey: XOnlyPublicKey,
        url: reqwest::Url,
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
    DemandProfile(XOnlyPublicKey, Vec<Url>),
    // DemandTextNote(EventRef),
    CloseLane(DynamicIndex),
    LinkClicked(InternalLink),
    SubscriptionsChanged,
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
            let is_central = self.focused == Some(event_id);
            let is_profile = self.subscription.is_a_profile();
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
                let subscribes_events = !self.subscription.events().is_empty();
                let idx = self.text_notes.iter().position(|tn| {
                    let ord = tn.time.timestamp().cmp(&event_time.as_i64());
                    ord == if subscribes_events {
                        Ordering::Greater
                    } else {
                        Ordering::Less
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

            // if g.len() > 10 {
            //     trace!(
            //         "Lane {:?} has {} notes, removing some.",
            //         self.subscription(),
            //         g.len()
            //     );
            // };

            while g.len() > 10 {
                let _ = g.pop_back();
            }
        }
    }

    /// Returns a subscription of this lane, if it exists.
    pub fn subscription(&self) -> &Subscription {
        &self.subscription
    }
}
