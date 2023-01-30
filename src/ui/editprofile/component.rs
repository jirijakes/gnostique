use gtk::prelude::*;
use nostr_sdk::prelude::Metadata;
use relm4::*;

use super::model::*;

#[relm4::component(pub)]
impl Component for EditProfile {
    type Init = ();
    type Input = EditProfileInput;
    type Output = EditProfileResult;
    type CommandOutput = ();

    view! {
        gtk::Window {
            set_widget_name: "editprofile",
            set_default_size: (400, 400),
            #[watch] set_visible: model.visible,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "form",

                gtk::Grid {
                    set_column_spacing: 16,
                    set_row_spacing: 16,

                    attach[0, 0, 1, 1] = &gtk::Label {
                        set_text: "Name",
                        set_xalign: 1.0,
                        set_valign: gtk::Align::Center,
                        add_css_class: "label",
                    },

                    #[name(name)]
                    attach[1, 0, 1, 1] = &gtk::Entry {
                        add_css_class: "name",
                    },

                    attach[0, 1, 1, 1] = &gtk::Label {
                        set_text: "Bio",
                        set_xalign: 1.0,
                        set_valign: gtk::Align::Start,
                        add_css_class: "label",
                    },

                    attach[1, 1, 1, 1] = &gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        set_min_content_height: 90,

                        #[name(bio)]
                        gtk::TextView {
                            // set_buffer: Some(&model.buffer),
                            set_top_margin: 4,
                            set_left_margin: 4,
                            set_right_margin: 4,
                            set_bottom_margin: 4,
                            set_wrap_mode: gtk::WrapMode::WordChar,
                            add_css_class: "multiline",
                        }
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_hexpand: true,
                    set_spacing: 8,

                    gtk::Box { set_hexpand: true },

                    gtk::Button::with_label("Cancel") {
                        connect_clicked => EditProfileInput::Cancel
                    },

                    gtk::Button::with_label("Apply") {
                        add_css_class: "suggested-action",
                        connect_clicked => EditProfileInput::Apply
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = EditProfile { visible: false };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            EditProfileInput::Show => self.visible = true,
            EditProfileInput::Apply => {
                let metadata = form_to_metadata(widgets);

                sender
                    .output(EditProfileResult::Apply(metadata))
                    .unwrap_or_default();
            }
            EditProfileInput::Cancel => self.visible = false,
        };

        self.update(message, sender.clone(), root);
        self.update_view(widgets, sender);
    }
}

fn form_to_metadata(widgets: &EditProfileWidgets) -> Metadata {
    let mut metadata = Metadata::new();

    let buffer = widgets.bio.buffer();
    let bio = buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string();

    if !bio.is_empty() {
        metadata.about = Some(bio);
    }

    let name = widgets.name.text().to_string();
    if !name.is_empty() {
        metadata.name = Some(name);
    }

    metadata
}
