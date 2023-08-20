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
        gtk::Box {
            add_css_class: "preview",
            gtk::Label {
                set_label?: model.preview.title()
            },
            gtk::Label {
                set_label?: model.preview.description()
            },
            gtk::Image::from_paintable(model.preview.thumbnail()) {
            }
        }
    }

    fn init(
        init: Self::Init,
        root: &Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Preview { preview: init };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}
