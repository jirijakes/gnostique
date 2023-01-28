use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use gtk::prelude::*;
use nostr_sdk::Client;
use relm4::gtk;
use relm4::prelude::*;
use reqwest::Url;
use tokio::time::interval;

use crate::Gnostique;

#[derive(Debug)]
pub struct RelayStatus {
    connected: HashSet<Url>,
    connecting: HashSet<Url>,
    disconnected: HashSet<Url>,
}

#[derive(Debug)]
pub struct StatusBar {
    relay_status: Option<RelayStatus>,
}

#[derive(Debug)]
pub enum StatusBarInput {
    UpdateRelayStatus(RelayStatus),
}

#[relm4::component(pub)]
impl SimpleComponent for StatusBar {
    type Input = StatusBarInput;
    type Output = ();
    type Init = Arc<Gnostique>;

    #[rusfmt::skip]
    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_hexpand: true,
            add_css_class: "statusbar",

            // filler
            gtk::Box {
                set_hexpand: true,
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                #[watch] set_visible: model.relay_status.is_some(),
                add_css_class: "relaystatus",

                gtk::Button {
                    #[watch] set_tooltip_markup: Some(&model.format_relay_status_tooltip()),
                    #[wrap(Some)]
                    set_child = &gtk::Label {
                        #[watch] set_markup?: &model.format_relay_status(),
                    }
                }
            }
        }
    }

    fn init(
        gnostique: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        relm4::spawn(update_relay_status(gnostique.client.clone(), sender));

        let model = StatusBar { relay_status: None };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            StatusBarInput::UpdateRelayStatus(status) => self.relay_status = Some(status),
        }
    }
}

impl StatusBar {
    fn format_relay_status(&self) -> Option<String> {
        if let Some(RelayStatus {
            ref connected,
            ref connecting,
            ref disconnected,
        }) = self.relay_status
        {
            Some(format!(
                r###"Relays: <span color="#00ff00">{}</span>  <span color="orange">{}</span>  <span color="red">{}</span>"###,
                connected.len(),
                connecting.len(),
                disconnected.len()
            ))
        } else {
            None
        }
    }

    fn format_relay_status_tooltip(&self) -> String {
        fn status<'a>(
            relays: &'a HashSet<Url>,
            color: &'a str,
            status: &'a str,
        ) -> impl Iterator<Item = String> + 'a {
            relays
                .iter()
                .map(move |r| format!(r#"[<span color="{color}">{status}</span>] {r}"#))
        }

        if let Some(RelayStatus {
            ref connected,
            ref connecting,
            ref disconnected,
        }) = self.relay_status
        {
            let status = [
                status(connected, "#00ff00", "Connected"),
                status(connecting, "orange", "Connecting"),
                status(disconnected, "red", "Disconnected"),
            ]
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

            let status = if status.is_empty() {
                "No relays."
            } else {
                &status
            };

            format!("<b>Status of relays:</b>\n\n{status}")
        } else {
            "Could not obtain status of relays.".to_string()
        }
    }
}

/// Periodically checks status of connected relays and upon very changed
/// sends a message to this widget with latest status.
async fn update_relay_status(client: Client, sender: ComponentSender<StatusBar>) {
    let mut int = interval(Duration::from_secs(5));

    loop {
        int.tick().await;
        let relays = client.relays().await;

        let mut connected = HashSet::new();
        let mut connecting = HashSet::new();
        let mut disconnected = HashSet::new();

        use nostr_sdk::relay::RelayStatus::*;

        for s in relays.values() {
            match s.status().await {
                Connected => {
                    connected.insert(s.url());
                }
                Disconnected | Terminated => {
                    disconnected.insert(s.url());
                }
                _ => {
                    connecting.insert(s.url());
                }
            }
        }

        sender.input(StatusBarInput::UpdateRelayStatus(RelayStatus {
            connected,
            connecting,
            disconnected,
        }));
    }
}
