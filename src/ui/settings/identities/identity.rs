use gtk::prelude::*;
use relm4::prelude::*;

use super::model::IdentitiesInput;
use crate::identity::Identity;

#[derive(Debug)]
pub struct IdentityBox {
    pub enabled: bool,
    pub identity: Identity,
}

pub enum IdentityInit {
    New(usize),
    Existing(Identity),
}

#[derive(Debug)]
pub enum IdentityInput {
    Enable(bool),
}

#[derive(Debug)]
pub enum IdentityOutput {
    Remove(DynamicIndex),
    Edit(DynamicIndex),
}

#[relm4::factory(pub)]
impl FactoryComponent for IdentityBox {
    type Init = IdentityInit;
    type Input = IdentityInput;
    type Output = IdentityOutput;
    type CommandOutput = ();
    type ParentInput = IdentitiesInput;
    type ParentWidget = gtk::Box;

    view! {
        root = gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            add_css_class: "identity",

            gtk::Switch::new() {
                set_valign: gtk::Align::Center,
                set_active: self.enabled,
                connect_state_set[sender] => move |_, state| {
                    sender.input(IdentityInput::Enable(state));
                    gtk::Inhibit(false)
                }
            },

            gtk::EditableLabel {
                #[watch]
                set_text: self.identity.name(),
                set_hexpand: true,
            },

            gtk::Button::from_icon_name("document-edit-symbolic") {
                add_css_class: "flat",
                connect_clicked[sender, index] => move |_| {
                    sender.output(IdentityOutput::Edit(index.clone()))
                }
            },

            gtk::Button::from_icon_name("list-remove-symbolic") {
                add_css_class: "flat",
                inline_css: "color: red",
                connect_clicked[sender, index] => move |_| {
                    sender.output(IdentityOutput::Remove(index.clone()))
                }
            },

        }
    }

    fn forward_to_parent(output: Self::Output) -> Option<Self::ParentInput> {
        match output {
            IdentityOutput::Remove(idx) => Some(IdentitiesInput::Remove(idx)),
            IdentityOutput::Edit(idx) => Some(IdentitiesInput::Edit(idx)),
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let identity = match init {
            IdentityInit::New(n) => Identity::new_random(&format!("Identity {}", n)),
            IdentityInit::Existing(i) => i,
        };

        IdentityBox {
            enabled: true,
            identity,
        }
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {
            IdentityInput::Enable(state) => {
                self.enabled = state;
            }
        }
    }
}
