use std::sync::Arc;

use gtk::prelude::*;
use relm4::prelude::*;
use relm4::{gtk, ComponentParts};
use serde_json::Value;

/// A window that display all available information about a note.
/// One instance of it is created and reused, therefore everytime
/// the window shows, it has to be provided with fresh information
/// to display.
pub struct DetailsWindow {
    /// Whether the window is visible or hidden.
    visible: bool,

    /// All available information about a text note.
    details: Option<Details>,

    /// Buffer for `TextView` displaying event JSON.
    event_buffer: gtk::TextBuffer,

    /// Buffer for `TextView` displaying metadata JSON.
    metadata_buffer: gtk::TextBuffer,
}

/// Messages coming to [`DetailsWindow`].
#[derive(Debug)]
pub enum DetailsWindowInput {
    /// Update details and show the window, if hidden.
    Show(Details),

    /// Hide the window.
    Hide,
}

#[relm4::component(pub)]
impl Component for DetailsWindow {
    type Init = ();
    type Input = DetailsWindowInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Window {
            #[watch]
            set_visible: model.visible,
            // set_modal: true,

            connect_close_request[sender] => move |_| {
                sender.input(DetailsWindowInput::Hide);
                gtk::Inhibit(false)
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,

                gtk::StackSidebar {
                    set_stack: &stack
                },

                #[name(stack)]
                gtk::Stack {
                    set_hexpand: true,

                    add_child = &gtk::Box { } -> { set_title: "Text note" },

                    add_child = &gtk::Box { } -> { set_title: "Author" },

                    add_child = &gtk::ScrolledWindow {
                        #[wrap(Some)]
                        set_child = &gtk::TextView {
                            set_buffer: Some(&model.event_buffer),
                            set_editable: false,
                            set_monospace: true,
                        }
                    } -> { set_title: "Event" },

                    #[name(metadata)]
                    add_child = &gtk::ScrolledWindow {
                        #[wrap(Some)]
                        set_child = &gtk::TextView {
                            set_buffer: Some(&model.metadata_buffer),
                            set_editable: false,
                            set_monospace: true,
                        }
                    } -> { set_title: "Metadata" }

                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = DetailsWindow {
            visible: false,
            details: None,
            event_buffer: gtk::TextBuffer::new(None),
            metadata_buffer: gtk::TextBuffer::new(None),
        };
        let widgets = view_output!();

        ComponentParts { widgets, model }
    }

    fn update(
        &mut self,
        message: Self::Input,
        _sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            DetailsWindowInput::Show(details) => {
                self.details = Some(details);
                self.event_buffer.set_text(
                    self.details
                        .as_ref()
                        .map(|d| &d.event_json)
                        .unwrap_or(&String::new()),
                );

                let metadata_json = &self
                    .details
                    .as_ref()
                    .and_then(|d| d.metadata_json.clone())
                    .unwrap_or(Arc::new(String::new()));

                self.metadata_buffer.set_text(metadata_json);
                if let Some(x) = pretty_content(metadata_json) {
                    self.metadata_buffer.insert(
                        &mut self.metadata_buffer.end_iter(),
                        &format!("\n\n\n// Parsed content:\n\n{x}"),
                    );
                }
                self.visible = true;
            }
            DetailsWindowInput::Hide => self.visible = false,
        }
    }
}

fn pretty_content(metadata_json: &str) -> Option<String> {
    let metadata_value = serde_json::from_str::<Value>(metadata_json).ok()?;
    let content_str = metadata_value.get("content")?.as_str()?;
    let content_value = serde_json::from_str::<Value>(content_str).ok()?;
    serde_json::to_string_pretty(&content_value).ok()
}

/// All available information about a note.
// TODO: Could it be passed as input to Note widget?
#[derive(Clone, Debug)]
pub struct Details {
    /// Complete JSON of the note event.
    pub event_json: String,

    /// Complete JSON of the author metadata.
    pub metadata_json: Option<Arc<String>>,
}
