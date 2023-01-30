use gtk::prelude::*;
use relm4::*;

use crate::app::action::EditProfile;

#[derive(Debug)]
pub struct LaneHeader {}

#[relm4::component(pub)]
impl SimpleComponent for LaneHeader {
    type Input = ();
    type Init = ();
    type Output = ();

    view! {
        gtk::CenterBox {
            set_hexpand: true,
            add_css_class: "laneheader",

            #[wrap(Some)]
            set_start_widget = &gtk::Box {
                gtk::Button::from_icon_name("mail-message-new-symbolic") {
                    set_has_frame: false,
                    set_tooltip_text: Some("Write new text note with the current identity"),
                    connect_clicked[sender] => move |_| { sender.output(()).unwrap() }
                }
            },

            #[wrap(Some)]
            set_center_widget = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 10,
                gtk::Label {
                    set_text: "Feed",
                    add_css_class: "name"
                },
                gtk::Label {
                    set_text: "Main identity",
                    add_css_class: "identity"
                }
            },

            #[wrap(Some)]
            set_end_widget = &gtk::Box {
                gtk::Button::from_icon_name("open-menu-symbolic") {
                    set_has_frame: false,
                    set_tooltip_text: Some("Open menu to see list of actions"),
                    connect_clicked => move |b| {
                        let popover = gtk::PopoverMenu::builder()
                            .menu_model(&main_menu)
                            .has_arrow(false)
                            .build();
                        popover.set_parent(b);
                        popover.popup();
                    }
                }
            },
        }
    }

    menu! {
        main_menu: {
            "Edit profile" => EditProfile
        }
    }

    fn init(
        _init: Self::Init,
        _root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = LaneHeader {};
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _message: Self::Input, _sender: ComponentSender<Self>) {}
}
