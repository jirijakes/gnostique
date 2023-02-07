use std::rc::Rc;
use std::sync::Arc;

use gtk::gdk;
use nostr_sdk::prelude::*;

use crate::nostr::*;
use crate::ui::details::Details;

/// Initial
pub struct NoteInit {
    pub event: Rc<Event>,
    pub relays: Vec<Url>,
    pub author: Option<Persona>,
    pub is_central: bool,
    pub repost: Option<Repost>,
}

#[derive(Clone, Debug)]
pub enum NoteInput {
    /// Author profile has some new data.
    UpdatedProfile {
        author: Persona,
    },
    /// The text note comes into focus.
    FocusIn,
    /// The text note loses focus.
    FocusOut,
    /// Show this note's details.
    ShowDetails,
    /// (New) avatar bitmap is available.
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
    TextNote {
        event: Rc<Event>,
        relays: Vec<Url>,
        author: Option<Persona>,
        repost: Option<Repost>,
    },
}

#[derive(Debug)]
pub enum NoteOutput {
    ShowDetails(Details),
    LinkClicked(String),
}
