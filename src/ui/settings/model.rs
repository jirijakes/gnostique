use relm4::Controller;

use super::identities::model::Identities;
use crate::identity::Identity;

#[derive(Debug)]
pub struct Settings {
    pub visible: bool,
    pub identities: Controller<Identities>,
}

#[derive(Debug)]
pub enum SettingsInput {
    /// Display the settings dialog.
    Show(Vec<Identity>),
    /// Hide the settings dialog.
    Hide,
}
