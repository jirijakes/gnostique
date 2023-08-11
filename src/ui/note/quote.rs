use std::sync::Arc;

use gtk::prelude::*;
use nostr_sdk::Event;
use relm4::{SimpleComponent, ComponentSender, ComponentParts};

use crate::nostr::{TextNote, Persona};
use crate::ui::widgets::author::Author;

/// Quoted text note whose widget is inserted into a text note
/// that references it.
#[derive(Debug)]
pub struct Quote {
    /// Event of the quoted text note.
    event: Arc<Event>,
    /// Author of the quoted texxt note.
    author: Arc<Persona>
}

#[relm4::component(pub)]
impl SimpleComponent for Quote {

    type Init = TextNote;
    type Input = ();
    type Output = ();
    
    #[rustfmt::skip]
    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,
            add_css_class: "quote",

            Author::with_pubkey(model.author.pubkey) {
                #[watch] set_persona: &model.author,
                #[watch] set_nip05_verified: model.author.nip05_preverified,
            },

            gtk::Label {
                #[watch] set_label: &model.event.content,
                set_wrap: true,
                set_wrap_mode: gtk::pango::WrapMode::Word,
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Start,
                set_xalign: 0.0,
            }
        }
    }

    fn init(
        note: TextNote,
        _root: &Self::Root,
        _sender: ComponentSender<Self>) -> ComponentParts<Self>
    {
        let (event, author) = note.underlying();
        let model = Quote { event, author };
        let widgets = view_output!();
        
        ComponentParts { model, widgets }
    }
    
}
