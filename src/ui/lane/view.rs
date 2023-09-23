use gtk::prelude::*;
use relm4::factory::{AsyncFactoryComponent, FactoryVecDeque};
use relm4::prelude::*;
use relm4::{gtk, AsyncFactorySender};

use crate::nostr::subscriptions::Subscription;
use crate::nostr::Persona;
use crate::ui::lane::model::*;
use crate::ui::lane_header::{LaneHeader, LaneHeaderInput, LaneHeaderOutput};
use crate::ui::main::MainInput;
use crate::ui::note::{NoteInput, NoteOutput};
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
            add_css_class: "lane",

            // HEADER
            self.header.widget() { },

            // PROFILE BOX (before text notes)

            // TEXT NOTES
            #[name = "text_notes"]
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
        kind: LaneKind,
        index: &DynamicIndex,
        sender: AsyncFactorySender<Self>,
    ) -> Self {
        let profile_box =
            if let LaneKind::Subscription(Subscription::Profile(pubkey, relays)) = &kind {
                // Since persona does not include avatar bitmap, it has to be obtained
                // from outside. Once #0464b5d7fa3bbbad is solved, this should not be
                // needed anymore.
                sender.output(LaneOutput::DemandProfile(*pubkey, relays.clone()));
                Some(Profilebox::builder().launch(*pubkey).detach())
            } else {
                None
            };

        let header = {
            let index = index.clone();
            LaneHeader::builder()
                .launch(kind.clone())
                .forward(sender.output_sender(), move |out| match out {
                    LaneHeaderOutput::CloseLane => LaneOutput::CloseLane(index.clone()),
                })
        };

        let text_notes = FactoryVecDeque::builder(
            gtk::ListBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .build(),
        )
        .launch()
        .forward(sender.input_sender(), |msg| match msg {
            NoteOutput::ShowDetails(details) => LaneMsg::ShowDetails(details),
            NoteOutput::LinkClicked(link) => LaneMsg::LinkClicked(link),
        });

        Self {
            kind,
            profile_box,
            index: index.clone(),
            header,
            text_notes,
            hash_index: Default::default(),
        }
    }

    fn init_widgets(
        &mut self,
        _di: &DynamicIndex,
        root: &Self::Root,
        _returned_widget: &gtk::Widget,
        _sender: AsyncFactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        match self.kind {
            LaneKind::Subscription(Subscription::Profile(..)) => root.add_css_class("profile"),
            LaneKind::Thread(_) => {}
            _ => {}
        };

        // Profile box will exist only if this lane is of kind Profile.
        if let Some(p) = &self.profile_box {
            p.widget().insert_before(root, Some(&widgets.text_notes));
        };

        widgets
    }

    fn forward_to_parent(output: Self::Output) -> Option<Self::ParentInput> {
        match output {
            LaneOutput::ShowDetails(details) => Some(MainInput::ShowDetail(details)),
            LaneOutput::WriteNote => Some(MainInput::WriteNote),
            LaneOutput::DemandProfile(pubkey, relays) => {
                Some(MainInput::DemandProfile(pubkey, relays))
            }
            LaneOutput::CloseLane(id) => Some(MainInput::CloseLane(id)),
            LaneOutput::LinkClicked(link) => Some(MainInput::LinkClicked(link)),
        }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncFactorySender<Self>) {
        match msg {
            LaneMsg::ShowDetails(details) => {
                sender.output(LaneOutput::ShowDetails(details));
            }

            LaneMsg::Preview(preview) => {
                self.text_notes.broadcast(NoteInput::Preview(preview));
            }

            LaneMsg::UpdatedProfile { author } => {
                if self.kind.is_profile(&author.pubkey) {
                    if let Some(p) = &self.profile_box {
                        p.emit(profilebox::Input::UpdatedProfile {
                            author: author.clone(),
                        });
                    }

                    self.header.emit(LaneHeaderInput::ChangeTitle(
                        author.show_name().unwrap_or(author.short_bech32(24)),
                    ));
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
                    if let Some(p) = &self.profile_box {
                        p.emit(profilebox::Input::MetadataBitmap {
                            url: url.clone(),
                            bitmap: bitmap.clone(),
                        });
                    }
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
                note,
                content,
                relays,
                repost,
                referenced_notes,
                referenced_profiles,
            } => {
                self.text_notes.broadcast(NoteInput::TextNote {
                    note: note.clone(),
                    content: content.clone(),
                    relays: relays.clone(),
                    repost: repost.clone(),
                    referenced_notes: referenced_notes.clone(),
                    referenced_profiles: referenced_profiles.clone(),
                });

                if self.kind.accepts(note.event())
                    || repost
                        .as_ref()
                        .map(|r| self.kind.accepts(r.event()))
                        .unwrap_or_default()
                {
                    self.text_note_received(
                        note,
                        content,
                        relays,
                        repost,
                        referenced_notes,
                        referenced_profiles,
                    )
                }
            }
            LaneMsg::LinkClicked(uri) => sender.output(LaneOutput::LinkClicked(uri)),
            LaneMsg::CloseLane => sender.output(LaneOutput::CloseLane(self.index.clone())),
        }
    }
}
