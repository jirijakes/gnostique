use secrecy::SecretString;

use crate::Gnostique;

/// Messages incoming into [`App`].
#[derive(Debug)]
pub enum AppInput {
    Unlock(SecretString),
    Quit,
}

#[derive(Debug)]
pub enum AppCmd {
    Unlocked(Gnostique),
    Error(String),
}
