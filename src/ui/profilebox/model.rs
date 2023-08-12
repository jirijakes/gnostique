use std::sync::Arc;

use gtk::gdk::Texture;
use reqwest::Url;

use crate::nostr::{Persona, ANONYMOUS_USER};

#[derive(Debug)]
pub struct Profilebox {
    pub author: Arc<Persona>,
    pub avatar: Arc<Texture>,
    pub banner: Option<Arc<Texture>>,
}

impl Profilebox {
    pub fn new(persona: Arc<Persona>) -> Self {
        Self {
            author: persona,
            avatar: ANONYMOUS_USER.clone(),
            banner: None,
        }
    }
}

#[derive(Debug)]
pub enum Input {
    UpdatedProfile { author: Arc<Persona> },
    MetadataBitmap { url: Url, bitmap: Arc<Texture> },
}
