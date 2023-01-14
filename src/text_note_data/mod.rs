mod imp;

use glib::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use nostr_sdk::nostr::secp256k1::XOnlyPublicKey;
use relm4::gtk;

glib::wrapper! {
    pub struct TextNoteData(ObjectSubclass<imp::TextNoteData>);
}

impl TextNoteData {
    /// Create new text note with given data.
    pub fn new(content: &str, pubkey: &XOnlyPublicKey) -> TextNoteData {
        glib::Object::new(&[("content", &content), ("pubkey", &pubkey.to_string())])
    }

    /// Get text note's content.
    pub fn content(&self) -> String {
        self.imp().content.borrow().to_owned()
    }

    pub fn set_content(&self, content: &str) {
        self.imp().content.replace(content.to_owned());
        self.notify("content");
    }

    pub fn set_pubkey(&self, pubkey: XOnlyPublicKey) {
        self.imp().pubkey.replace(Some(pubkey));
        self.notify("pubkey");
    }

    pub fn set_author(&self, author: &str) {
        self.imp().author.replace(Some(author.to_owned()));
        self.notify("author");
    }
}
