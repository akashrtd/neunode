use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};

use crate::CryptoError;

const NONCE_SIZE: usize = 12;

/// Derive a 256-bit AES key from machine-specific identity (hostname + username).
pub fn derive_machine_key() -> [u8; 32] {
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "localhost".to_string());
    let username = whoami();
    let combined = format!("{hostname}:{username}");
    let hash = crate::hash::sha256(combined.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Encrypt plaintext using AES-256-GCM with the provided key.
/// Returns `[nonce (12 bytes) | ciphertext + tag]`.
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> std::result::Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    use rand::RngCore;
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::SerializationError(format!("encryption failed: {e}")))?;

    let mut output = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// Decrypt data encrypted by `encrypt`. Expects `[nonce (12 bytes) | ciphertext + tag]`.
pub fn decrypt(key: &[u8; 32], data: &[u8]) -> std::result::Result<Vec<u8>, CryptoError> {
    if data.len() < NONCE_SIZE + 16 {
        return Err(CryptoError::SerializationError("ciphertext too short".to_string()));
    }
    let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::SerializationError(format!("decryption failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = derive_machine_key();
        let plaintext = b"hello world secret data";
        let encrypted = encrypt(&key, plaintext).unwrap();
        assert_ne!(&encrypted[..], plaintext);
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(&decrypted[..], plaintext);
    }

    #[test]
    fn encrypt_produces_different_ciphertexts() {
        let key = derive_machine_key();
        let plaintext = b"same data";
        let c1 = encrypt(&key, plaintext).unwrap();
        let c2 = encrypt(&key, plaintext).unwrap();
        assert_ne!(c1, c2, "different nonces should produce different ciphertexts");
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let key1 = [0u8; 32];
        let key2 = [1u8; 32];
        let encrypted = encrypt(&key1, b"secret").unwrap();
        assert!(decrypt(&key2, &encrypted).is_err());
    }

    #[test]
    fn decrypt_tampered_data_fails() {
        let key = derive_machine_key();
        let mut encrypted = encrypt(&key, b"secret").unwrap();
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF;
        assert!(decrypt(&key, &encrypted).is_err());
    }

    #[test]
    fn decrypt_too_short_fails() {
        let key = derive_machine_key();
        assert!(decrypt(&key, &[0u8; 10]).is_err());
    }

    #[test]
    fn derive_machine_key_is_deterministic() {
        let k1 = derive_machine_key();
        let k2 = derive_machine_key();
        assert_eq!(k1, k2);
    }

    #[test]
    fn derive_machine_key_is_32_bytes() {
        let key = derive_machine_key();
        assert_eq!(key.len(), 32);
    }
}
