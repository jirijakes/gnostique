use futures_util::FutureExt;
use gtk::prelude::*;
use relm4::*;
use secrecy::{Secret, SecretString};

use crate::app::init::make_gnostique;
use crate::Gnostique;

#[derive(Debug)]
pub struct Unlock;

#[derive(Debug)]
pub enum UnlockResult {
    Unlocked(Gnostique),
    Quit,
}

#[derive(Debug)]
pub enum UnlockInput {
    Unlock(SecretString),
}

#[derive(Debug)]
pub enum UnlockCmd {
    Unlocked(Gnostique),
    Error(String),
}

#[relm4::component(pub)]
impl Component for Unlock {
    type Init = ();
    type Input = UnlockInput;
    type Output = UnlockResult;
    type CommandOutput = UnlockCmd;

    view! {
        gtk::Stack {

            gtk::Box {
                set_valign: gtk::Align::Center,
                set_halign: gtk::Align::Center,
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 18,
                set_widget_name: "password",

                gtk::Label {
                    set_label: "Unlock Gnostique identity",
                    add_css_class: "caption",
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    add_css_class: "passwordbox",

                    gtk::Label {
                        set_xalign: 0.0,
                        set_label: "Enter password:"
                    },

                    #[name(password)]
                    gtk::PasswordEntry {
                        set_hexpand: true,
                        set_show_peek_icon: true,
                        set_activates_default: true,
                        connect_activate[sender] => move |password| {
                            sender.input(UnlockInput::Unlock(Secret::new(password.text().to_string())));
                        }
                    },

                    gtk::Box {
                        set_halign: gtk::Align::End,
                        set_spacing: 8,
                        add_css_class: "buttons",

                        #[name(unlock)]
                        gtk::Button {
                            add_css_class: "suggested-action",
                            set_label: "Unlock",
                            connect_clicked[sender, password] => move |_| {
                                sender.input(UnlockInput::Unlock(Secret::new(password.text().to_string())));
                            }
                        },

                        gtk::Button {
                            set_label: "Quit",
                            connect_clicked[sender] => move |_| sender.output(UnlockResult::Quit).unwrap_or_default(),
                        }
                    }
                }

            },

            #[name(spinner_page)]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_valign: gtk::Align::Center,
                set_halign: gtk::Align::Center,

                #[name(spinner)]
                gtk::Spinner {
                    set_spinning: true,
                    set_halign: gtk::Align::Center,
                    set_width_request: 32,
                    set_height_request: 32,
                },

                gtk::Label {
                    set_halign: gtk::Align::Center,
                    set_label: "Connecting to Nostr…",
                }
            }

        }
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Unlock;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            UnlockCmd::Unlocked(gn) => sender
                .output(UnlockResult::Unlocked(gn))
                .unwrap_or_default(),

            UnlockCmd::Error(e) => {
                println!(">>>>>>> {e}");
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        stack: &Self::Root,
    ) {
        match message {
            UnlockInput::Unlock(password) => {
                widgets.password.set_text("");
                widgets.spinner.start();
                stack.set_visible_child(&widgets.spinner_page);
                sender.oneshot_command(make_gnostique(password).map(|result| match result {
                    Ok(gn) => UnlockCmd::Unlocked(gn),
                    Err(e) => UnlockCmd::Error(e),
                }));
            }
        }

        self.update_view(widgets, sender);
    }
}
