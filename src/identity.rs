use std::str::FromStr;

use nostr_sdk::prelude::{Keys, SecretKey};
use secrecy::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Key(String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub secret_key: Secret<Key>,
}

impl Identity {
    pub fn nostr_key(&self) -> Keys {
        Keys::new(SecretKey::from_str(&self.secret_key.expose_secret().0).unwrap())
    }
}

impl Zeroize for Key {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl DebugSecret for Key {}
impl CloneableSecret for Key {}
impl SerializableSecret for Key {}
