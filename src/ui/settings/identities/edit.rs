use bip39::Mnemonic;
use gtk::glib::clone;
use gtk::prelude::*;
use nostr_sdk::secp256k1::rand::rngs::OsRng;
use nostr_sdk::secp256k1::rand::Rng;
use relm4::prelude::*;
use secrecy::ExposeSecret;

use crate::identity::Identity;

#[derive(Debug, PartialEq, Eq)]
enum RadioActive {
    Mnemonic,
    Existing,
    Readonly,
}

#[derive(Debug)]
pub struct Edit {
    /// Determines which radio button is currently active.
    radio_active: RadioActive,

    /// GTK Entry Buffer for identity name.
    name_buffer: gtk::EntryBuffer,

    /// GTK Text Buffer for mnemonic.
    mnemonic_buffer: gtk::TextBuffer,

    /// Index of identity that is currently being edited.
    editing_index: Option<DynamicIndex>,
}

impl Edit {
    /// Make an identity of this editing dialog.
    fn to_identity(&self) -> Option<Identity> {
        let name = self.name_buffer.text().to_string();
        let buffer = &self.mnemonic_buffer;
        let text = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .to_string();
        let mnemonic = Mnemonic::parse(text).ok()?;
        Some(Identity::from_bip39_mnemonic(&name, &mnemonic))
    }
}

#[derive(Debug)]
pub enum EditInput {
    /// Radio buttons toggled.
    Toggled,

    /// Command to refresh mnemonic.
    RefreshMnemonic,

    /// Mnemonic was just refreshed to given content.
    MnemonicRefreshed,

    /// Prepare dialog for editing.
    Edit {
        identity: Identity,
        index: DynamicIndex,
    },

    /// Edit finished.
    Finished,
}

#[derive(Debug)]
pub enum EditOutput {
    Canceled,
    Finished {
        index: DynamicIndex,
        new_identity: Identity,
    },
}

/// Edit identity.
#[relm4::component(pub)]
impl Component for Edit {
    type Init = ();
    type Input = EditInput;
    type Output = EditOutput;
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_valign: gtk::Align::Center,
            set_halign: gtk::Align::Center,
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 18,
            set_widget_name: "generate",

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                add_css_class: "formbox",
                add_css_class: "form",

                gtk::Grid {
                    set_column_spacing: 16,
                    set_row_spacing: 16,

                    attach[0, 0, 1, 1] = &gtk::Label {
                        set_text: "Name:",
                        set_xalign: 1.0,
                        set_valign: gtk::Align::Center,
                        add_css_class: "label"
                    },

                    attach[1, 0, 1, 1] = &gtk::Entry {
                        set_buffer: &model.name_buffer,
                        set_hexpand: true,
                    },

                    attach[0, 1, 2, 1] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::Overlay {
                            #[name(mnemonic_radio)]
                            gtk::CheckButton::with_label("From mnemonic") {
                                set_active: true,
                                connect_toggled => EditInput::Toggled
                            },

                            add_overlay = &gtk::Button::from_icon_name("view-refresh-symbolic") {
                                #[watch]
                                set_sensitive: model.radio_active == RadioActive::Mnemonic,
                                set_halign: gtk::Align::End,
                                set_valign: gtk::Align::Start,
                                add_css_class: "flat",
                                set_has_frame: false,
                                connect_clicked => EditInput::RefreshMnemonic,
                            }
                        },

                        #[name(mnemonic_box)]
                        gtk::ScrolledWindow {
                            #[watch]
                            set_sensitive: model.radio_active == RadioActive::Mnemonic,
                            set_hexpand: true,
                            set_min_content_height: 140,
                            gtk::Overlay {
                                #[name(mnemonic)]
                                gtk::TextView::with_buffer(&model.mnemonic_buffer) {
                                    set_monospace: true,
                                    set_top_margin: 4,
                                    set_bottom_margin: 4,
                                    set_left_margin: 4,
                                    set_right_margin: 4,
                                    set_wrap_mode: gtk::WrapMode::Word,
                                },

                                #[name(mnemonic_result)] add_overlay = &gtk::Label {
                                    #[watch]
                                    set_visible: model.radio_active == RadioActive::Mnemonic,
                                    set_halign: gtk::Align::End,
                                    set_valign: gtk::Align::End,
                                    set_margin_bottom: 6,
                                    set_margin_end: 8,
                                }
                            }
                        },

                    },
                    attach[0, 2, 2, 1] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        #[name(existing_radio)]
                        gtk::CheckButton::with_label("Open existing identity") {
                            set_group: Some(&mnemonic_radio),
                            connect_toggled => EditInput::Toggled
                        },

                        #[name(existing_box)]
                        gtk::Entry {
                            #[watch]
                            set_sensitive: model.radio_active == RadioActive::Existing,
                            set_hexpand: true,
                            set_icon_from_icon_name: (gtk::EntryIconPosition::Secondary, Some("folder-open-symbolic"))
                        }
                    },
                    attach[0, 3, 2, 1] = &gtk::Box {
                        #[name(readonly_radio)]
                        gtk::CheckButton::with_label("Read-only identity") {
                            set_group: Some(&mnemonic_radio),
                            connect_toggled => EditInput::Toggled
                        },
                    },
                    attach[0, 4, 2, 1] = &gtk::Box {
                        set_halign: gtk::Align::End,
                        set_spacing: 8,
                        add_css_class: "buttons",

                        #[name(ok)]
                        gtk::Button {
                            add_css_class: "suggested-action",
                            set_label: "OK",
                            connect_clicked[sender] => move |_| {
                                sender.input(EditInput::Finished);
                            }
                        },

                        gtk::Button {
                            set_label: "Cancel",
                            connect_clicked[sender] => move |_| sender.output(EditOutput::Canceled).unwrap_or_default(),
                        }
                    }

                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        _root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mnemonic_buffer = {
            let buffer = gtk::TextBuffer::default();
            buffer.connect_changed(clone!(@strong sender => move |_| {
                sender.input(EditInput::MnemonicRefreshed);
            }));
            buffer
        };

        let model = Edit {
            radio_active: RadioActive::Mnemonic,
            name_buffer: gtk::EntryBuffer::default(),
            mnemonic_buffer,
            editing_index: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            EditInput::MnemonicRefreshed => {
                let buffer = &self.mnemonic_buffer;
                let text = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), true)
                    .to_string();
                let mnemonic = Mnemonic::parse(&text);
                let tooltip = match &mnemonic {
                    Ok(_) => "Mnemonic is corect".to_string(),
                    Err(bip39::Error::BadWordCount(n)) => format!("Incorrent number of words: {n}"),
                    Err(bip39::Error::UnknownWord(idx)) => {
                        let unknown = text.split_whitespace().nth(*idx).unwrap();
                        format!("Unknown word: {unknown}")
                    }
                    Err(bip39::Error::InvalidChecksum) => "Checksum is invalid".to_string(),
                    Err(bip39::Error::AmbiguousLanguages(_)) => "Ambiguous languages".to_string(),
                    Err(_) => "Error during parsing".to_string(),
                };
                if mnemonic.is_ok() {
                    widgets
                        .mnemonic_result
                        .set_markup("<span color='lightgreen'>✓</span>");
                    widgets.ok.set_sensitive(true);
                } else {
                    widgets
                        .mnemonic_result
                        .set_markup("<span color='red'>✘</span>");
                    widgets.ok.set_sensitive(false);
                }
                widgets.mnemonic_result.set_tooltip_text(Some(&tooltip));
            }

            EditInput::Toggled => {
                if widgets.mnemonic_radio.is_active() {
                    self.radio_active = RadioActive::Mnemonic
                } else if widgets.existing_radio.is_active() {
                    self.radio_active = RadioActive::Existing
                } else {
                    self.radio_active = RadioActive::Readonly
                }
            }
            EditInput::RefreshMnemonic => {
                let m = Mnemonic::from_entropy(&OsRng.gen::<[_; 32]>()).unwrap();
                let x = m.word_iter().collect::<Vec<_>>().join(" ");
                widgets.mnemonic.buffer().set_text(&x);
            }
            EditInput::Edit { identity, index } => {
                self.name_buffer.set_text(identity.name());
                self.mnemonic_buffer
                    .set_text(identity.mnemonic().expose_secret().reveal());
                self.editing_index = Some(index);
                sender.input(EditInput::MnemonicRefreshed);
            }
            EditInput::Finished => {
                if let Some(index) = self.editing_index.take() {
                    if let Some(new_identity) = self.to_identity() {
                        sender
                            .output(EditOutput::Finished {
                                index,
                                new_identity,
                            })
                            .unwrap_or_default();
                    };
                }
            }
        }

        self.update_view(widgets, sender);
    }
}
