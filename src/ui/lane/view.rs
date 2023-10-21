use gtk::prelude::*;
use relm4::factory::{AsyncFactoryComponent, FactoryVecDeque};
use relm4::prelude::*;
use relm4::{gtk, AsyncFactorySender};

use crate::nostr::subscriptions::Subscription;
use crate::nostr::{EventExt, Persona};
use crate::ui::lane::model::*;
use crate::ui::lane_header::{LaneHeader, LaneHeaderInput, LaneHeaderOutput};
use crate::ui::main::MainInput;
use crate::ui::note::{NoteInput, NoteOutput};
use crate::ui::profilebox;
use crate::ui::profilebox::model::Profilebox;

#[relm4::factory(pub async)]
impl AsyncFactoryComponent for Lane {
    type Init = LaneInit;
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
        init: LaneInit,
        index: &DynamicIndex,
        sender: AsyncFactorySender<Self>,
    ) -> Self {
        let LaneInit {
            subscription,
            focused,
        } = init;
        let profile_box = if let Subscription::Profile(pubkey, relays) = &subscription {
            // Since persona does not include avatar bitmap, it has to be obtained
            // from outside. Once #0464b5d7fa3bbbad is solved, this should not be
            // needed anymore.
            sender.output(LaneOutput::DemandProfile(*pubkey, relays.clone()));
            Some(Profilebox::builder().launch(*pubkey).detach())
        } else {
            None
        };

        // Each lane has a header.
        let header = {
            let index = index.clone();
            LaneHeader::builder().launch(subscription.clone()).forward(
                sender.output_sender(),
                move |out| match out {
                    LaneHeaderOutput::CloseLane => LaneOutput::CloseLane(index.clone()),
                },
            )
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

        // When a new lane is opened, it is passed a subscription, however Nostr client
        // is not yet subscribed to it. It is lane's responsibility to prepare subscription
        // and then notify parent about when it's done.
        sender.output(LaneOutput::SubscriptionsChanged);

        Self {
            subscription,
            focused,
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

        match self.subscription {
            Subscription::Profile(..) => root.add_css_class("profile"),
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
            LaneOutput::SubscriptionsChanged => Some(MainInput::RefreshSubscriptions),
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
                if self.subscription.pubkeys().contains(&author.pubkey) {
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
                if self.subscription.pubkeys().contains(&pubkey) {
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
                tracing::trace!("Text note received: {}", note.event().id);

                self.text_notes.broadcast(NoteInput::TextNote {
                    note: note.clone(),
                    content: content.clone(),
                    relays: relays.clone(),
                    repost: repost.clone(),
                    referenced_notes: referenced_notes.clone(),
                    referenced_profiles: referenced_profiles.clone(),
                });

                // If the received note is focused, let us also subscribe to its
                // thread root (if it's not subscribed to it yet).
                if self.focused == Some(note.event().id) {
                    let mut new_subscriptions = vec![];

                    if let Some((root, _relay)) = note.event().thread_root() {
                        if !self.subscription().events().contains(&root) {
                            new_subscriptions.push(Subscription::thread(root));
                        }
                    }

                    if let Some(replies_to) = note.event().replies_to() {
                        if !self.subscription().events().contains(&replies_to) {
                            new_subscriptions.push(Subscription::thread(replies_to));
                        }
                    }

                    if !new_subscriptions.is_empty() {
                        self.subscription = new_subscriptions
                            .into_iter()
                            .fold(self.subscription.clone(), |s1, s2| s1.add(s2));
                        sender.output(LaneOutput::SubscriptionsChanged);
                    }
                }

                // If the note is meant for this lane, add it.
                if self.subscription().accepts(note.event())
                    || repost
                        .as_ref()
                        .map(|r| self.subscription().accepts(r.event()))
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
