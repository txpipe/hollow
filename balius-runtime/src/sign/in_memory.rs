use std::collections::HashMap;

use pallas::crypto::key::ed25519;
use rand::TryRng as _;

use crate::wit::balius::app::sign as wit;

use super::SignerProvider;

#[derive(Default, Clone)]
pub struct Signer {
    map: HashMap<String, HashMap<String, SignerKey>>,
}

impl Signer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<HashMap<String, HashMap<String, SignerKey>>> for Signer {
    fn from(value: HashMap<String, HashMap<String, SignerKey>>) -> Self {
        Self { map: value }
    }
}

#[async_trait::async_trait]
impl SignerProvider for Signer {
    async fn add_key(&mut self, worker_id: &str, key_name: String, algorithm: String) -> Vec<u8> {
        if algorithm != "ed25519" {
            panic!("Unsupported algorithm")
        }
        let keys = self.map.entry(worker_id.to_string()).or_default();
        let secret_key = keys.entry(key_name).or_insert_with(|| {
            let mut bytes = [0; ed25519::SecretKey::SIZE];
            rand::rngs::SysRng
                .try_fill_bytes(&mut bytes)
                .expect("system RNG failed while generating ed25519 key");
            ed25519::SecretKey::from(bytes).into()
        });
        secret_key.public_key()
    }

    async fn sign_payload(
        &mut self,
        worker_id: &str,
        key_name: String,
        payload: wit::Payload,
    ) -> Result<wit::Signature, wit::SignError> {
        let Some(key) = self
            .map
            .entry(worker_id.to_string())
            .or_default()
            .get(&key_name)
        else {
            return Err(wit::SignError::KeyNotFound(key_name.to_string()));
        };
        key.sign_payload(payload)
    }
}

#[derive(Clone, Debug)]
pub enum SignerKey {
    Ed25519(Ed25519Key),
}

impl From<ed25519::SecretKey> for SignerKey {
    fn from(value: ed25519::SecretKey) -> Self {
        Self::Ed25519(Ed25519Key::SecretKey(value))
    }
}

impl From<ed25519::SecretKeyExtended> for SignerKey {
    fn from(value: ed25519::SecretKeyExtended) -> Self {
        Self::Ed25519(Ed25519Key::SecretKeyExtended(value))
    }
}

impl SignerKey {
    pub fn sign_payload(&self, payload: wit::Payload) -> Result<wit::Signature, wit::SignError> {
        match self {
            Self::Ed25519(key) => Ok(key.sign_payload(payload)),
        }
    }

    pub fn public_key(&self) -> Vec<u8> {
        match self {
            Self::Ed25519(key) => key.public_key(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Ed25519Key {
    SecretKey(ed25519::SecretKey),
    SecretKeyExtended(ed25519::SecretKeyExtended),
}
impl Ed25519Key {
    fn sign_payload(&self, payload: wit::Payload) -> wit::Signature {
        let signature = match self {
            Self::SecretKey(key) => key.sign(payload),
            Self::SecretKeyExtended(key) => key.sign(payload),
        };
        signature.as_ref().to_vec()
    }
    fn public_key(&self) -> Vec<u8> {
        match self {
            Self::SecretKey(key) => key.public_key().as_ref().to_vec(),
            Self::SecretKeyExtended(key) => key.public_key().as_ref().to_vec(),
        }
    }
}
