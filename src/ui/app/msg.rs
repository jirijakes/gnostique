use crate::Gnostique;

/// Messages incoming into [`App`].
#[derive(Debug)]
pub enum AppInput {
    Unlocked(Gnostique),
    Quit,
}
