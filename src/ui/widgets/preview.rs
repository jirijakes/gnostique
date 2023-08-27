use gtk;
use gtk::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

use crate::nostr::preview;

#[derive(Debug)]
pub struct Preview {
    preview: preview::Preview,
}

#[relm4::component(pub)]
impl SimpleComponent for Preview {
    type Init = preview::Preview;
    type Input = ();
    type Output = ();

    #[rustfmt::skip]
    view! {
        #[name = "grid"]
        gtk::Grid {
            add_css_class: "preview",
            attach[1, 1, 1, 1] = &gtk::Box {
                add_css_class: "infobox",
                set_orientation: gtk::Orientation::Vertical,
                set_hexpand: true,
                gtk::Label {
                    set_label?: preview.title(),
                    add_css_class: "title",
                    set_xalign: 0.0,
                    set_wrap_mode: gtk::pango::WrapMode::Word,
                    set_wrap: true,
                    set_visible: preview.title().is_some()
                },
                gtk::Label {
                    set_label?: preview.description(),
                    add_css_class: "description",
                    set_xalign: 0.0,
                    set_wrap_mode: gtk::pango::WrapMode::Word,
                    set_wrap: true,
                    set_visible: preview.description().is_some()
                },
                gtk::Label {
                    set_label: preview.url().as_ref(),
                    add_css_class: "url",
                    set_ellipsize: gtk::pango::EllipsizeMode::Middle,
                    set_xalign: 0.0
                }
            }
        }
    }

    fn init(
        preview: Self::Init,
        root: &Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();

        if let Some(t) = preview.thumbnail() {
            let picture = gtk::Picture::builder()
                .paintable(t)
                .content_fit(gtk::ContentFit::Cover)
                .width_request(400)
                .height_request(
                    (f64::from(t.height()) / f64::from(t.width()) * 400.0).ceil() as i32,
                )
                .build();

            widgets.grid.attach(&picture, 1, 0, 1, 1);
        }

        let model = Preview { preview };

        ComponentParts { model, widgets }
    }
}
