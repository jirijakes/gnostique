use gtk;
use gtk::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

use crate::nostr::preview;

#[derive(Debug)]
pub struct Preview {
    preview: preview::Preview,
    image: Option<gtk::Picture>,
}

#[relm4::component(pub)]
impl SimpleComponent for Preview {
    type Init = preview::Preview;
    type Input = ();
    type Output = ();

    #[rustfmt::skip]
    view! {
        gtk::Overlay {
            add_css_class: "preview",
            set_child = model.image.as_ref(),
            add_overlay = &gtk::Box {
                add_css_class: "infobox",
                set_orientation: gtk::Orientation::Vertical,
                set_halign: gtk::Align::Fill,
                set_valign: gtk::Align::End,
                gtk::Label {
                    set_label?: model.preview.title(),
                    add_css_class: "title",
                    set_xalign: 0.0,
                    set_wrap_mode: gtk::pango::WrapMode::Word,
                    set_wrap: true,
                    set_visible: model.preview.title().is_some()
                },
                gtk::Label {
                    set_label?: model.preview.description(),
                    add_css_class: "description",
                    set_xalign: 0.0,
                    set_wrap_mode: gtk::pango::WrapMode::Word,
                    set_wrap: true,
                    set_visible: model.preview.description().is_some()
                },
                gtk::Label {
                    set_label: model.preview.url().as_ref(),
                    add_css_class: "url",
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_xalign: 0.0
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: &Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let image = init.thumbnail().map(|t| {
            gtk::Picture::builder()
                .paintable(t)
                .content_fit(gtk::ContentFit::Cover)
                .width_request(400)
                .height_request(
                    (f64::from(t.height()) / f64::from(t.width()) * 400.0).ceil() as i32,
                )
                .build()
        });
        let model = Preview {
            preview: init,
            image,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}
