use std::cell::RefCell;
use std::str::FromStr;

use glib::subclass::prelude::*;
use glib::{ParamSpec, ParamSpecString, Value};
use gtk::glib;
use gtk::prelude::*;
use nostr_sdk::nostr::secp256k1::XOnlyPublicKey;
use relm4::gtk;

#[derive(Default)]
pub struct TextNoteData {
    pub(super) content: RefCell<String>,
    pub(super) pubkey: RefCell<Option<XOnlyPublicKey>>,
    pub(super) author: RefCell<Option<String>>,
}

#[glib::object_subclass]
impl ObjectSubclass for TextNoteData {
    const NAME: &'static str = "TextNoteData";
    type Type = super::TextNoteData;
}

impl ObjectImpl for TextNoteData {
    fn properties() -> &'static [ParamSpec] {
        use relm4::once_cell::sync::Lazy;

        static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
            vec![
                ParamSpecString::builder("content")
                    .explicit_notify()
                    .build(),
                ParamSpecString::builder("pubkey").explicit_notify().build(),
                ParamSpecString::builder("author").explicit_notify().build(),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
        match pspec.name() {
            "content" => {
                let content = value.get().unwrap();
                self.content.replace(content);
            }
            "author" => {
                let author = value.get().unwrap();
                self.author.replace(author);
            }
            "pubkey" => {
                let pubkey = XOnlyPublicKey::from_str(value.get().unwrap()).unwrap();
                self.pubkey.replace(Some(pubkey));
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
        match pspec.name() {
            "content" => self.content.borrow().to_value(),
            "author" => self.author.borrow().to_value(),
            "pubkey" => self.pubkey.borrow().map(|p| p.to_string()).to_value(),
            _ => unimplemented!(),
        }
    }
}
