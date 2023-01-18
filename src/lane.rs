use std::sync::Arc;

use gtk::gdk;
use nostr_sdk::nostr::secp256k1::XOnlyPublicKey;
use nostr_sdk::nostr::Event;
use nostr_sdk::nostr::Sha256Hash;
// use nostr_sdk::sqlite::model::Profile;
use relm4::factory::FactoryVecDeque;
use relm4::gtk;
use relm4::prelude::*;

use crate::ui::details::Details;
use crate::ui::note::Note;
use crate::ui::note::NoteInit;
use crate::ui::note::NoteInput;
use crate::win::Msg;

#[derive(Debug)]
pub struct Lane {
    central_note: Option<Sha256Hash>,
    text_notes: FactoryVecDeque<Note>,
}

#[derive(Debug)]
pub enum LaneMsg {
    NewTextNote {
        event: Event,
        // profile: Option<Profile>,
    },
    UpdatedProfile {
        author_pubkey: XOnlyPublicKey,
        author_name: Option<String>,
        metadata_json: String,
    },
    ShowDetails(Details),
    AvatarBitmap {
        pubkey: XOnlyPublicKey,
        bitmap: Arc<gdk::Texture>,
    },
    Reaction {
        event: Sha256Hash,
        reaction: String,
    },
}

#[derive(Debug)]
pub enum LaneOutput {
    ShowDetails(Details),
}

impl FactoryComponent for Lane {
    type Init = Option<Sha256Hash>;
    type Input = LaneMsg;
    type Output = LaneOutput;
    type CommandOutput = ();
    type ParentInput = Msg;
    type ParentWidget = gtk::Box;
    type Root = gtk::ScrolledWindow;
    type Widgets = ();

    fn init_root(&self) -> Self::Root {
        gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_width(600)
            .build()
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: &Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        root.set_child(Some(self.text_notes.widget()));
    }

    fn init_model(
        central_note: Self::Init,
        _index: &DynamicIndex,
        sender: FactorySender<Self>,
    ) -> Self {
        Self {
            central_note,
            text_notes: FactoryVecDeque::new(
                gtk::ListBox::builder()
                    .selection_mode(gtk::SelectionMode::None)
                    .build(),
                sender.input_sender(),
            ),
        }
    }

    fn output_to_parent_input(output: Self::Output) -> Option<Self::ParentInput> {
        match output {
            LaneOutput::ShowDetails(details) => Some(Msg::ShowDetail(details)),
        }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            LaneMsg::ShowDetails(details) => {
                sender.output(LaneOutput::ShowDetails(details));
            }
            LaneMsg::UpdatedProfile {
                author_pubkey,
                author_name,
                metadata_json,
            } => {
                for i in 0..self.text_notes.len() {
                    self.text_notes.send(
                        i,
                        NoteInput::UpdatedProfile {
                            author_pubkey,
                            author_name: author_name.clone(),
                            metadata_json: metadata_json.clone(),
                        },
                    )
                }
            }
            LaneMsg::AvatarBitmap { pubkey, bitmap } => {
                for i in 0..self.text_notes.len() {
                    self.text_notes.send(
                        i,
                        NoteInput::AvatarBitmap {
                            pubkey,
                            bitmap: bitmap.clone(),
                        },
                    );
                }
            }
            LaneMsg::Reaction { event, reaction } => {
                for i in 0..self.text_notes.len() {
                    self.text_notes.send(
                        i,
                        NoteInput::Reaction {
                            event,
                            reaction: reaction.clone(),
                        },
                    );
                }
            }
            LaneMsg::NewTextNote {
                event, /*, profile*/
            } => {
                if !self.text_notes.iter().any(|tn| tn.event_id == event.id) {
                    let is_central = self.central_note.contains(&event.id);
                    let event_time = event.created_at;

                    let text_note = NoteInit {
                        event,
                        // profile,
                        is_central,
                    };

                    if is_central {
                        // Central text note always goes first.
                        self.text_notes.guard().push_front(text_note);
                    } else {
                        // Find index of first text note that was created later
                        // than the text note being inserted.
                        let idx = self
                            .text_notes
                            .iter()
                            .position(|tn| tn.time.timestamp() as u64 > event_time);

                        if let Some(idx) = idx {
                            // Inserting somewhere in the middle.
                            self.text_notes.guard().insert(idx, text_note);
                        } else {
                            // Appending to the end.
                            self.text_notes.guard().push_back(text_note);
                        }
                    }
                }
            }
        }
    }
}
