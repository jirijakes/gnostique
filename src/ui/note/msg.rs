use std::collections::HashSet;
use std::sync::Arc;

use gtk::gdk;
use nostr_sdk::prelude::*;

use crate::nostr::content::DynamicContent;
use crate::nostr::*;
use crate::ui::details::Details;

/// Initial
pub struct NoteInit {
    pub note: TextNote,
    pub content: Arc<DynamicContent>,
    pub relays: Vec<Url>,
    pub is_central: bool,
    pub is_profile: bool,
    pub repost: Option<Repost>,
    pub referenced_notes: HashSet<TextNote>,
    pub referenced_profiles: HashSet<Persona>,
}

#[derive(Clone, Debug)]
pub enum NoteInput {
    /// Author profile has some new data.
    UpdatedProfile {
        author: Arc<Persona>,
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
        note: TextNote,
        content: Arc<DynamicContent>,
        relays: Vec<Url>,
        repost: Option<Repost>,
        referenced_notes: HashSet<TextNote>,
        referenced_profiles: HashSet<Persona>,
    },
    Tick,
}

#[derive(Debug)]
pub enum NoteOutput {
    ShowDetails(Details),
    LinkClicked(String),
    OpenProfile(Arc<Persona>, Url)
}
