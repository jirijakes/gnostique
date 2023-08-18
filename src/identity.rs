use nostr_sdk::prelude::{FromMnemonic, Keys};
use nostr_sdk::secp256k1::rand::rngs::OsRng;
use nostr_sdk::secp256k1::rand::Rng;
use secrecy::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Key(String);

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Mnemonic(String);

impl Mnemonic {
    pub fn reveal(&self) -> &str {
        &self.0
    }
}

/// Identity of a Nostr user containing
/// all the secrets and identity-specific settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    /// Mnemonic used to create secret key.
    mnemonic: Secret<Mnemonic>,

    /// Name of the identity.
    name: String,
}

impl Identity {
    pub fn new_random(name: &str) -> Identity {
        Identity {
            name: name.to_string(),
            mnemonic: Secret::new(Mnemonic(
                bip39::Mnemonic::from_entropy(&OsRng.gen::<[_; 32]>())
                    .unwrap()
                    .to_string(),
            )),
        }
    }

    pub fn from_bip39_mnemonic(name: &str, mnemonic: &bip39::Mnemonic) -> Identity {
        Identity {
            mnemonic: Secret::new(Mnemonic(mnemonic.to_string())),
            name: name.to_string(),
        }
    }

    /// Name of the identity.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Mnemonic of the identity.
    pub fn mnemonic(&self) -> &Secret<Mnemonic> {
        &self.mnemonic
    }

    pub fn nostr_key(&self) -> Keys {
        Keys::from_mnemonic(self.mnemonic.expose_secret().reveal(), None).unwrap()
    }
}

impl Zeroize for Mnemonic {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl DebugSecret for Mnemonic {}
impl CloneableSecret for Mnemonic {}
impl SerializableSecret for Mnemonic {}
