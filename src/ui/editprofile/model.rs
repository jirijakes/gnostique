use nostr_sdk::prelude::Metadata;

#[derive(Debug)]
pub struct EditProfile {
    pub visible: bool,
}

#[derive(Debug)]
pub enum EditProfileInput {
    Show,
    Cancel,
    Apply,
}

#[derive(Debug)]
pub enum EditProfileResult {
    Apply(Metadata),
}
