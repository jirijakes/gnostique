use gtk::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

use super::edit::{Edit, EditInput, EditOutput};
use super::identity::IdentityInit;
use super::model::{Identities, IdentitiesInput};

/// Component that contains a settings section for managing identities.
/// It is only displayed as part of Settings.
#[relm4::component(pub)]
impl Component for Identities {
    type Init = ();
    type Input = IdentitiesInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Stack {
            #[name(overview)]
            gtk::Box {
                set_valign: gtk::Align::Start,
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_widget_name: "identities",
                set_hexpand: true,

                gtk::Label {
                    set_label: "List of identities:",
                    set_halign: gtk::Align::Start,
                    set_xalign: 0.0,
                },

                gtk::Box {
                    set_orientation:  gtk::Orientation::Horizontal,

                    gtk::ScrolledWindow {
                        set_min_content_height: 400,

                        #[local_ref]
                        identities -> gtk::Box {
                            set_vexpand: true,
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 12,
                            set_widget_name: "identity-list"
                        },

                    },
                    gtk::Box {
                        set_valign: gtk::Align::Start,

                        gtk::Button::with_label("Create") {
                            set_hexpand: false,
                            set_vexpand: false,
                            connect_clicked[sender] => move |_| sender.input(IdentitiesInput::Add),
                        }
                    }
                }
            },

            #[local_ref]
            edit -> gtk::Box { },
        }

    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Identities {
            identities: FactoryVecDeque::new(gtk::Box::default(), sender.input_sender()),
            identity_counter: 1,
            edit: Edit::builder().launch(()).forward(
                sender.input_sender(),
                |result| match result {
                    EditOutput::Canceled => IdentitiesInput::EditCanceled,
                    EditOutput::Finished {
                        index,
                        new_identity,
                    } => IdentitiesInput::EditFinished {
                        index,
                        new_identity,
                    },
                },
            ),
        };

        let identities = model.identities.widget();
        let edit = model.edit.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        _sender: ComponentSender<Self>,
        stack: &Self::Root,
    ) {
        match message {
            IdentitiesInput::Add => {
                self.identities
                    .guard()
                    .push_back(IdentityInit::New(self.identity_counter));
                self.identity_counter += 1;
            }
            IdentitiesInput::Remove(idx) => {
                self.identities.guard().remove(idx.current_index());
            }
            IdentitiesInput::Edit(idx) => {
                if let Some(id) = self.identities.get(idx.current_index()) {
                    self.edit.emit(EditInput::Edit {
                        identity: id.identity.clone(),
                        index: idx,
                    });
                    stack.set_visible_child(&widgets.edit);
                }
            }
            IdentitiesInput::EditCanceled => {
                stack.set_visible_child(&widgets.overview);
            }
            IdentitiesInput::EditFinished {
                index,
                new_identity,
            } => {
                if let Some(id) = self.identities.guard().get_mut(index.current_index()) {
                    id.identity = new_identity
                }
                stack.set_visible_child(&widgets.overview);
            }
            IdentitiesInput::Load(identities) => {
                let mut q = self.identities.guard();
                q.clear();
                for i in identities {
                    q.push_back(IdentityInit::Existing(i));
                }
            }
        }
    }
}
