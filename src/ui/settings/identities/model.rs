use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

use super::edit::Edit;
use super::identity::IdentityBox;
use crate::identity::Identity;

#[derive(Debug)]
pub struct Identities {
    /// List of identity widgets.
    pub(super) identities: FactoryVecDeque<IdentityBox>,

    /// Number of identity widgets created in the session,
    /// only purpose is to generate a new name for
    /// newly created identities (like “Identity 12”).
    pub(super) identity_counter: usize,

    pub(super) edit: Controller<Edit>,
}

#[derive(Debug)]
pub enum IdentitiesInput {
    /// Create new identity and add it to identity list.
    Add,

    /// Remove identity with given index from identity list.
    Remove(DynamicIndex),

    /// Edit identity with given index.
    Edit(DynamicIndex),

    /// Editing was canceled.
    EditCanceled,

    /// Editing was finished and confirmed.
    EditFinished {
        index: DynamicIndex,
        new_identity: Identity,
    },
    Load(Vec<Identity>),
}
