use gtk::prelude::*;
use relm4::gtk;
use relm4::prelude::*;

/// Widget temlate for displaying author name and author pubkey.
#[relm4::widget_template(pub)]
impl WidgetTemplate for Author {
    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 12,
            add_css_class: "author",

            #[name = "author_name"]
            gtk::Button {
                add_css_class: "author-name"
            },

            #[name = "author_pubkey"]
            gtk::Label {
                add_css_class: "author-pubkey",
                set_use_markup: true,
            },

            #[name = "author_nip05"]
            gtk::Label {
                add_css_class: "author-nip05",
                set_yalign: 1.0,
                set_visible: false
            }
        }
    }
}
