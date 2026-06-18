// AES-256-GCM — authenticated encryption
// Client-side: plaintext NEVER leaves the user's machine
// Key is always split via SSS before any network transmission

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlob {
    pub nonce: Vec<u8>,   // 96-bit GCM nonce
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoKey {
    pub key: [u8; 32],   // AES-256 key
    pub nonce: [u8; 12], // GCM nonce — stored with key for SSS bundling
}

impl RepoKey {
    pub fn generate() -> Self {
        let key = Aes256Gcm::generate_key(OsRng);
        let nonce = Aes256Gcm::generate_nonce(OsRng);
        Self {
            key: key.into(),
            nonce: nonce.into(),
        }
    }

    // Serialize key+nonce as 44 bytes for SSS input
    // Secret = key(32) || nonce(12) — treated as single secret
    pub fn to_secret_bytes(&self) -> Vec<u8> {
        let mut secret = Vec::with_capacity(44);
        secret.extend_from_slice(&self.key);
        secret.extend_from_slice(&self.nonce);
        secret
    }

    pub fn from_secret_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 44 {
            anyhow::bail!("invalid secret length: expected 44, got {}", bytes.len());
        }
        let mut key = [0u8; 32];
        let mut nonce = [0u8; 12];
        key.copy_from_slice(&bytes[..32]);
        nonce.copy_from_slice(&bytes[32..44]);
        Ok(Self { key, nonce })
    }
}

// Encrypt arbitrary data — O(n) single pass
pub fn encrypt(data: &[u8], repo_key: &RepoKey) -> Result<EncryptedBlob> {
    let key = Key::<Aes256Gcm>::from_slice(&repo_key.key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&repo_key.nonce);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|_| anyhow::anyhow!("AES-256-GCM encryption failed"))?;

    Ok(EncryptedBlob {
        nonce: repo_key.nonce.to_vec(),
        ciphertext,
    })
}

// Decrypt — O(n), verifies GCM authentication tag automatically
pub fn decrypt(blob: &EncryptedBlob, repo_key: &RepoKey) -> Result<Vec<u8>> {
    let key = Key::<Aes256Gcm>::from_slice(&repo_key.key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&blob.nonce);

    let plaintext = cipher
        .decrypt(nonce, blob.ciphertext.as_slice())
        .map_err(|_| anyhow::anyhow!("AES-256-GCM decryption failed — data tampered or wrong key"))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = RepoKey::generate();
        let plaintext = b"secret source code";
        let blob = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&blob, &key).unwrap();
        assert_eq!(plaintext.as_ref(), decrypted.as_slice());
    }

    #[test]
    fn wrong_key_fails() {
        let key1 = RepoKey::generate();
        let key2 = RepoKey::generate();
        let blob = encrypt(b"secret", &key1).unwrap();
        assert!(decrypt(&blob, &key2).is_err());
    }

    #[test]
    fn secret_bytes_roundtrip() {
        let key = RepoKey::generate();
        let secret = key.to_secret_bytes();
        let recovered = RepoKey::from_secret_bytes(&secret).unwrap();
        assert_eq!(key.key, recovered.key);
        assert_eq!(key.nonce, recovered.nonce);
    }
}
