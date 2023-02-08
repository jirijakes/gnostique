use secrecy::SecretString;

/// Messages incoming into [`App`].
#[derive(Debug)]
pub enum AppInput {
    Unlock(SecretString),
    Quit
}
