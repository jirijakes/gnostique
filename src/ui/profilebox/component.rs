use gtk::prelude::*;
use nostr_sdk::prelude::ToBech32;
use relm4::*;

use super::model::{Input, Profilebox};

#[relm4::component(pub)]
impl Component for Profilebox {
    type Input = Input;
    type Output = ();
    type Init = ();
    type CommandOutput = ();

    view! {
        gtk::Box {
            add_css_class: "profilebox",
            set_orientation: gtk::Orientation::Horizontal,

            // avatar
            gtk::Box {
                set_size_request: (100, 100),

                gtk::Picture {
                    #[watch]
                    set_paintable: Some(model.avatar.as_ref()),
                    set_content_fit: gtk::ContentFit::Contain,
                    set_can_shrink: true,
                    add_css_class: "avatar"
                }
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                // name + nip05
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,

                    gtk::Label {
                        set_selectable: true,
                        set_xalign: 0.0,
                        add_css_class: "name",
                        #[watch] set_label?: model.author.as_ref().and_then(|a| a.name.as_ref()),
                    },

                    gtk::Label {
                        set_selectable: true,
                        set_xalign: 0.0,
                        add_css_class: "nip05",
                        #[watch] set_label?: &model.author.as_ref().and_then(|a| a.format_nip05()),
                    },
                },

                gtk::Label {
                    set_selectable: true,
                    set_xalign: 0.0,
                    set_ellipsize: gtk::pango::EllipsizeMode::Middle,
                    add_css_class: "pubkey",
                    #[watch] set_label?: &model.author.as_ref().map(|a| a.pubkey.to_bech32().unwrap()),
                },

                gtk::Label {
                    set_selectable: true,
                    set_xalign: 0.0,
                    add_css_class: "about",
                    #[watch] set_label?: &model.author.as_ref().and_then(|a| a.about.as_ref()),
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Profilebox::new();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            Input::UpdatedProfile { author } => self.author = Some(author),
            Input::MetadataBitmap { bitmap, url } => {
                if let Some(author) = &self.author {
                    if author.avatar == Some(url.clone()) {
                        self.avatar = bitmap
                    } else if author.banner == Some(url) {
                        self.banner = Some(bitmap)
                    }
                }
            }
        }
    }
}
