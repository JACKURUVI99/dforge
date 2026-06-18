// Ed25519 identity — NodeId = Blake3(public_key)
// Ed25519: 32-byte keys, 64-byte signatures, ~100k sign/verify per second
// Faster than RSA-2048 by 20x, same security as RSA-3072

use ed25519_dalek::{
    Signature, Signer, SigningKey, Verifier, VerifyingKey,
};
use rand::rngs::OsRng;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::hash::ContentId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 32]); // Blake3(public_key)

impl NodeId {
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn short(&self) -> String {
        hex::encode(&self.0[..6]) // 12 hex chars for display
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "12D3KooW{}", &self.to_hex()[..16])
    }
}

pub struct Identity {
    pub node_id: NodeId,
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl Identity {
    // Generate new identity — S/Kademlia style: NodeId = Blake3(pubkey)
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let node_id = NodeId(*ContentId::from_bytes(verifying_key.as_bytes()).as_bytes());
        Self { node_id, signing_key, verifying_key }
    }

    // Sign any data — used for PR/issue/commit authenticity
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        self.signing_key.sign(data).to_bytes().to_vec()
    }

    // Export keypair to bytes for persistence in ~/.dforge/identity
    pub fn to_bytes(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            anyhow::bail!("invalid signing key length");
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();
        let node_id = NodeId(*ContentId::from_bytes(verifying_key.as_bytes()).as_bytes());
        Ok(Self { node_id, signing_key, verifying_key })
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.verifying_key.as_bytes())
    }
}

pub fn verify_signature(public_key_bytes: &[u8], data: &[u8], sig_bytes: &[u8]) -> Result<bool> {
    let verifying_key = VerifyingKey::from_bytes(
        public_key_bytes.try_into().map_err(|_| anyhow::anyhow!("invalid public key length"))?
    )?;
    let signature = Signature::from_bytes(
        sig_bytes.try_into().map_err(|_| anyhow::anyhow!("invalid signature length"))?
    );
    Ok(verifying_key.verify(data, &signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify() {
        let id = Identity::generate();
        let data = b"commit abc123";
        let sig = id.sign(data);
        assert!(verify_signature(id.verifying_key.as_bytes(), data, &sig).unwrap());
    }

    #[test]
    fn keypair_persistence() {
        let id = Identity::generate();
        let bytes = id.to_bytes();
        let restored = Identity::from_bytes(&bytes).unwrap();
        assert_eq!(id.node_id.0, restored.node_id.0);
    }
}
