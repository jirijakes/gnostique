use gtk::prelude::*;
use relm4::prelude::*;

use super::identities::model::IdentitiesInput;
use super::identities::Identities;
use super::model::{Settings, SettingsInput};

#[relm4::component(pub)]
impl SimpleComponent for Settings {
    type Init = ();
    type Input = SettingsInput;
    type Output = ();

    view! {
        gtk::Window {
            #[watch]
            set_visible: model.visible,
            set_title: Some("Settings"),
            set_default_size: (500, 500),
            set_modal: true,
            set_widget_name: "settings",

            connect_close_request[sender] => move |_| {
                sender.input(SettingsInput::Hide);
                gtk::Inhibit(false)
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,

                    gtk::StackSidebar {
                        set_stack: &stack
                    },

                    #[name(stack)]
                    gtk::Stack {
                        set_hexpand: true,
                        set_vexpand: true,

                        add_child = &gtk::Box { } -> { set_title: "Appearance" },
                        #[local_ref] add_child = identities -> gtk::Stack { } -> { set_title: "Identities" },
                        add_child = &gtk::Box { } -> { set_title: "Relays" },
                        add_child = &gtk::Box { } -> { set_title: "Nostr" },
                        add_child = &gtk::Box { } -> { set_title: "Privacy" },
                        add_child = &gtk::Box { } -> { set_title: "Addressbook" },
                        add_child = &gtk::Box { } -> { set_title: "Scripts" },
                        add_child = &gtk::Box { } -> { set_title: "External" },
                        add_child = &gtk::Box { } -> { set_title: "Connections" },
                    },
                },
                gtk::Box {
                    set_halign: gtk::Align::End,

                    gtk::Button {
                        add_css_class: "suggested-action",
                        set_label: "OK"
                    },

                    gtk::Button {
                        set_label: "Cancel"
                    }

                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Settings {
            visible: false,
            identities: Identities::builder().launch(()).detach(),
        };

        let identities = model.identities.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            SettingsInput::Hide => self.visible = false,
            SettingsInput::Show(identities) => {
                self.identities.emit(IdentitiesInput::Load(identities));
                self.visible = true;
            }
        }
    }
}
