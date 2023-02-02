use gtk::prelude::*;
use relm4::*;

use super::model::*;

#[relm4::component(pub)]
impl SimpleComponent for WriteNote {
    type Init = ();
    type Input = WriteNoteInput;
    type Output = WriteNoteResult;

    view! {
        gtk::Window {
            set_widget_name: "writenote",
            // set_modal: true,
            // set_decorated: false,
            set_default_size: (400, 400),
            #[watch] set_visible: model.visible,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "form",

                gtk::Grid {
                    set_column_spacing: 16,
                    set_row_spacing: 16,

                    attach[0, 0, 1, 1] = &gtk::Label {
                        set_text: "Content",
                        set_xalign: 1.0,
                        set_valign: gtk::Align::Start,
                        add_css_class: "label",
                    },

                    attach[1, 0, 1, 1] = &gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        set_min_content_height: 180,

                        gtk::TextView {
                            set_buffer: Some(&model.buffer),
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
                        connect_clicked => WriteNoteInput::Cancel
                    },

                    gtk::Button::with_label("Send") {
                        add_css_class: "suggested-action",
                        connect_clicked => WriteNoteInput::Send
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
        let model = WriteNote {
            visible: false,
            buffer: gtk::TextBuffer::new(None),
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            WriteNoteInput::Hide => {
                self.visible = false;
                self.buffer.set_text("");
            }
            WriteNoteInput::Show => self.visible = true,
            WriteNoteInput::Cancel => {
                sender.output(WriteNoteResult::Cancel).unwrap_or_default();
                sender.input(WriteNoteInput::Hide)
            }
            WriteNoteInput::Send => {
                let content = self
                    .buffer
                    .text(&self.buffer.start_iter(), &self.buffer.end_iter(), true)
                    .to_string();
                sender
                    .output(WriteNoteResult::Send(content))
                    .unwrap_or_default();
                sender.input(WriteNoteInput::Hide)
            }
        }
    }
}
