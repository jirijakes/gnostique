use futures_util::FutureExt;
use gtk::prelude::*;
use relm4::component::*;
use relm4::*;
use secrecy::Secret;

use crate::ui::main::Main;

use super::model::*;
use super::msg::*;

#[relm4::component(pub)]
impl Component for App {
    type Init = ();
    type Input = AppInput;
    type Output = ();
    type CommandOutput = AppCmd;

    view! {
        #[name(window)]
        gtk::ApplicationWindow {
            set_default_widget: Some(&unlock),

            #[name(stack)]
            gtk::Stack {
                set_hexpand: true,
                set_vexpand: true,

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
                                    sender.input(AppInput::Unlock(Secret::new(password.text().to_string())));
                                }
                            },

                            gtk::Button {
                                set_label: "Quit",
                                connect_clicked => AppInput::Quit,
                            }
                        }
                    }
                },

                #[name(spinner)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,
                    set_valign: gtk::Align::Center,
                    set_halign: gtk::Align::Center,

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
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Default::default();
        let widgets = view_output!();

        widgets
            .window
            .insert_action_group("author", Some(&crate::app::action::make_author_actions()));

        // widgets.window.insert_action_group(
        //     "main",
        //     Some(&crate::app::action::make_main_menu_actions(sender)),
        // );

        ComponentParts { model, widgets }
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AppCmd::Unlocked(gn) => {
                let main = Main::builder().launch(gn).detach();
                widgets.stack.add_named(main.widget(), Some("main"));
                self.main = Some(main);
                widgets.stack.set_visible_child_name("main");
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AppInput::Quit => relm4::main_application().quit(),
            AppInput::Unlock(password) => {
                widgets.password.set_text("");
                widgets.stack.set_visible_child(&widgets.spinner);
                sender.oneshot_command(crate::app::init::make_gnostique().map(AppCmd::Unlocked));
            }
        }

        self.update_view(widgets, sender);
    }
}
