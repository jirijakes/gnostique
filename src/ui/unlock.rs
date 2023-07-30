use std::path::PathBuf;

use directories::ProjectDirs;
use gtk::prelude::*;
use relm4::*;
use secrecy::{Secret, SecretString};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

use crate::config::Config;
use crate::gnostique::{make_gnostique, Gnostique};

#[derive(Clone, Debug)]
pub struct Conf {
    dirs: ProjectDirs,
    pool: SqlitePool,
    id_file: PathBuf,
}

#[derive(Debug)]
pub enum Unlock {
    Loading,
    Loaded(Conf),
}

#[derive(Debug)]
pub enum UnlockResult {
    Unlocked(Gnostique),
    Quit,
}

#[derive(Debug)]
pub enum UnlockInput {
    Unlock(SecretString),
    RequestPassword,
}

#[derive(Debug)]
pub enum UnlockCmd {
    Unlocked(Gnostique),
    Error(String),
    UnlockIdentity(Conf),
    CreateIdentity(Conf),
}

#[relm4::component(pub)]
impl Component for Unlock {
    type Init = ();
    type Input = UnlockInput;
    type Output = UnlockResult;
    type CommandOutput = UnlockCmd;

    view! {
        gtk::Stack {
            #[name = "spinner_page"]
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
            },

            #[name = "password_page"]
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
                    add_css_class: "formbox",

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
                            let pwd = password.text().to_string();
                            password.set_text("");
                            sender.input(UnlockInput::Unlock(Secret::new(pwd)));
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
                                let pwd = password.text().to_string();
                                password.set_text("");
                                sender.input(UnlockInput::Unlock(Secret::new(pwd)));
                            }
                        },

                        gtk::Button {
                            set_label: "Quit",
                            connect_clicked[sender] => move |_| sender.output(UnlockResult::Quit).unwrap_or_default(),
                        }
                    }
                }

            },
        }
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Unlock::Loading;

        let widgets = view_output!();

        widgets.spinner.start();

        sender.oneshot_command(async {
            let dirs = ProjectDirs::from("com.jirijakes", "", "Gnostique").unwrap();
            tokio::fs::create_dir_all(dirs.data_dir()).await.unwrap();
            tokio::fs::create_dir_all(dirs.config_dir()).await.unwrap();

            let config = Config {
                db_file: dirs.data_dir().join("databaze.db"),
            };

            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(config.db_file)
                        .create_if_missing(true),
                )
                .await
                .unwrap();

            sqlx::migrate!().run(&pool).await.unwrap();

            let id_file = dirs.config_dir().join("identity");

            let id_exists = tokio::fs::try_exists(&id_file).await.unwrap();

            let conf = Conf {
                dirs,
                pool,
                id_file,
            };

            if id_exists {
                UnlockCmd::UnlockIdentity(conf)
            } else {
                UnlockCmd::CreateIdentity(conf)
            }
        });

        ComponentParts { model, widgets }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _stack: &Self::Root,
    ) {
        match message {
            UnlockCmd::Unlocked(gn) => sender
                .output(UnlockResult::Unlocked(gn))
                .unwrap_or_default(),

            UnlockCmd::Error(e) => {
                println!(">>>>>>> {e}");
            }

            UnlockCmd::UnlockIdentity(conf) => {
                *self = Unlock::Loaded(conf);
                sender.input(UnlockInput::RequestPassword);
            }

            UnlockCmd::CreateIdentity(conf) => {
                *self = Unlock::Loaded(conf);
                sender.input(UnlockInput::RequestPassword);
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
            UnlockInput::RequestPassword => {
                stack.set_visible_child(&widgets.password_page);
            }
            UnlockInput::Unlock(password) => {
                stack.set_visible_child(&widgets.spinner_page);

                if let Unlock::Loaded(conf) = self {
                    let conf = conf.clone();
                    sender.oneshot_command(async {
                        match make_gnostique(conf.dirs, conf.pool, conf.id_file, password).await {
                            Ok(gn) => UnlockCmd::Unlocked(gn),
                            Err(e) => UnlockCmd::Error(e),
                        }
                    });
                };
            }
        }

        self.update_view(widgets, sender);
    }
}
