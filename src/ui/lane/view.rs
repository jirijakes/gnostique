use gtk::prelude::*;
use relm4::factory::{AsyncFactoryComponent, FactoryVecDeque};
use relm4::prelude::*;
use relm4::{gtk, AsyncFactorySender};

use crate::ui::lane::model::*;
use crate::ui::lane_header::LaneHeader;
use crate::ui::main::MainInput;
use crate::ui::note::NoteInput;
use crate::ui::profilebox;
use crate::ui::profilebox::model::Profilebox;

#[relm4::factory(pub async)]
impl AsyncFactoryComponent for Lane {
    type Init = LaneKind;
    type Input = LaneMsg;
    type Output = LaneOutput;
    type CommandOutput = ();
    type ParentInput = MainInput;
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            self.header.widget() { },

            // profile box
            self.profile_box.widget() {
                set_visible: self.kind.is_a_profile(),
            },

            // notes
            gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_min_content_width: 600,
                set_hexpand: true,
                set_vexpand: true,
                #[wrap(Some)]
                set_child = self.text_notes.widget() {}
            }
        }
    }

    async fn init_model(
        init: Self::Init,
        _index: &DynamicIndex,
        sender: AsyncFactorySender<Self>,
    ) -> Self {
        Self {
            kind: init.clone(),
            profile_box: Profilebox::builder().launch(()).detach(),
            header: LaneHeader::builder()
                .launch(init)
                .forward(sender.output_sender(), |_| LaneOutput::WriteNote),

            text_notes: FactoryVecDeque::new(
                gtk::ListBox::builder()
                    .selection_mode(gtk::SelectionMode::None)
                    .build(),
                sender.input_sender(),
            ),
            hash_index: Default::default(),
        }
    }

    // fn output_to_parent_input(output: Self::Output) -> Option<Self::ParentInput> {
    //     match output {
    //         _ => Some(())
    //         // LaneOutput::ShowDetails(details) => Some(Msg::ShowDetail(details)),
    //         // LaneOutput::WriteNote => Some(Msg::WriteNote),
    //     }
    // }

    async fn update(&mut self, msg: Self::Input, sender: AsyncFactorySender<Self>) {
        match msg {
            LaneMsg::ShowDetails(details) => {
                sender.output(LaneOutput::ShowDetails(details));
            }

            LaneMsg::UpdatedProfile { author } => {
                if self.kind.is_profile(&author.pubkey) {
                    self.profile_box.emit(profilebox::Input::UpdatedProfile {
                        author: author.clone(),
                    });
                }
                self.text_notes
                    .broadcast(NoteInput::UpdatedProfile { author });
            }

            LaneMsg::MetadataBitmap {
                pubkey,
                url,
                bitmap,
            } => {
                if self.kind.is_profile(&pubkey) {
                    self.profile_box.emit(profilebox::Input::MetadataBitmap {
                        url: url.clone(),
                        bitmap: bitmap.clone(),
                    })
                };

                self.text_notes.broadcast(NoteInput::MetadataBitmap {
                    pubkey,
                    url,
                    bitmap,
                });
            }

            LaneMsg::Reaction { event, reaction } => self
                .text_notes
                .broadcast(NoteInput::Reaction { event, reaction }),

            LaneMsg::Nip05Verified(pubkey) => {
                self.text_notes.broadcast(NoteInput::Nip05Verified(pubkey))
            }

            LaneMsg::NewTextNote {
                event,
                relays,
                author,
                repost,
            } => {
                self.text_notes.broadcast(NoteInput::TextNote {
                    event: event.clone(),
                    relays: relays.clone(),
                    author: author.clone(),
                    repost: repost.clone(),
                });

                if self.kind.accepts(&event)
                    || repost
                        .as_ref()
                        .map(|r| self.kind.accepts(&r.event))
                        .unwrap_or_default()
                {
                    self.text_note_received(event, relays, author, repost)
                }
            }
            LaneMsg::LinkClicked(uri) => println!("Clicked: {uri}"),
        }
    }
}
