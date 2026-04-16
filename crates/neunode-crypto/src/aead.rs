use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};

use crate::CryptoError;

const NONCE_SIZE: usize = 12;
const SALT_SIZE: usize = 32;
const KEY_DERIVATION_CONTEXT: &str = "neunode-aead-v1";

/// Derive a 256-bit AES key from a user-supplied passphrase and random salt.
///
/// Uses BLAKE3 key derivation (`blake3::derive_key`) which is a proper KDF
/// with a protocol-fixed context string. The salt should be cryptographically
/// random and at least 16 bytes (32 recommended).
///
/// # Security
///
/// This provides real key separation: two different passphrases produce
/// completely unrelated keys. The BLAKE3 KDF is resistant to length-extension
/// attacks and provides 256-bit output.
pub fn derive_key_from_passphrase(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    let mut material = Vec::with_capacity(passphrase.len() + salt.len());
    material.extend_from_slice(passphrase.as_bytes());
    material.extend_from_slice(salt);
    blake3::derive_key(KEY_DERIVATION_CONTEXT, &material)
}

/// Derive a 256-bit AES key from machine-specific identity (hostname + username).
///
/// # Deprecation
///
/// This function is deprecated because the derived key has very low entropy
/// (~20 bits from hostname + username). An attacker who knows the machine
/// identity can reconstruct the key. Use [`derive_key_from_passphrase`] with
/// a user-supplied secret instead.
#[deprecated(
    since = "0.2.0",
    note = "Low-entropy key derivation from hostname+username. Use derive_key_from_passphrase instead."
)]
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

/// Encrypt plaintext using AES-256-GCM with a passphrase-derived key.
///
/// Generates a random 32-byte salt and derives the encryption key using
/// BLAKE3 KDF. The output format is:
///
/// `[salt (32 bytes) | nonce (12 bytes) | ciphertext + tag]`
///
/// # Security
///
/// The random salt ensures that encrypting the same plaintext with the same
/// passphrase produces different ciphertext every time. Each encryption
/// operation uses a fresh key derivation, preventing key reuse across calls.
pub fn encrypt_with_passphrase(
    passphrase: &str,
    plaintext: &[u8],
) -> std::result::Result<Vec<u8>, CryptoError> {
    let mut salt = [0u8; SALT_SIZE];
    use rand::RngCore;
    OsRng.fill_bytes(&mut salt);
    let key = derive_key_from_passphrase(passphrase, &salt);
    let ciphertext = encrypt(&key, plaintext)?;
    let mut result = Vec::with_capacity(SALT_SIZE + ciphertext.len());
    result.extend_from_slice(&salt);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt data encrypted by `encrypt_with_passphrase`.
///
/// Expects `[salt (32 bytes) | nonce (12 bytes) | ciphertext + tag]`.
/// Extracts the salt, re-derives the key from the passphrase, and decrypts.
pub fn decrypt_with_passphrase(
    passphrase: &str,
    data: &[u8],
) -> std::result::Result<Vec<u8>, CryptoError> {
    if data.len() < SALT_SIZE + NONCE_SIZE + 16 {
        return Err(CryptoError::SerializationError(
            "ciphertext too short for passphrase decryption".to_string(),
        ));
    }
    let (salt, rest) = data.split_at(SALT_SIZE);
    let key = derive_key_from_passphrase(passphrase, salt);
    decrypt(&key, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = [0xAB_u8; 32];
        let plaintext = b"hello world secret data";
        let encrypted = encrypt(&key, plaintext).unwrap();
        assert_ne!(&encrypted[..], plaintext);
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(&decrypted[..], plaintext);
    }

    #[test]
    fn encrypt_produces_different_ciphertexts() {
        let key = [0xCD_u8; 32];
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
        let key = [0xEF_u8; 32];
        let mut encrypted = encrypt(&key, b"secret").unwrap();
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF;
        assert!(decrypt(&key, &encrypted).is_err());
    }

    #[test]
    fn decrypt_too_short_fails() {
        let key = [0u8; 32];
        assert!(decrypt(&key, &[0u8; 10]).is_err());
    }

    #[test]
    fn derive_machine_key_is_deterministic() {
        #[allow(deprecated)]
        let k1 = derive_machine_key();
        #[allow(deprecated)]
        let k2 = derive_machine_key();
        assert_eq!(k1, k2);
    }

    #[test]
    fn derive_machine_key_is_32_bytes() {
        #[allow(deprecated)]
        let key = derive_machine_key();
        assert_eq!(key.len(), 32);
    }

    // --- Passphrase-based encryption tests ---

    #[test]
    fn derive_key_from_passphrase_is_32_bytes() {
        let key = derive_key_from_passphrase("hunter2", &[0u8; 32]);
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn derive_key_from_passphrase_different_salts_differ() {
        let k1 = derive_key_from_passphrase("same-passphrase", &[0u8; 32]);
        let k2 = derive_key_from_passphrase("same-passphrase", &[1u8; 32]);
        assert_ne!(k1, k2, "different salts must produce different keys");
    }

    #[test]
    fn derive_key_from_passphrase_different_passphrases_differ() {
        let salt = [0x42_u8; 32];
        let k1 = derive_key_from_passphrase("passphrase-A", &salt);
        let k2 = derive_key_from_passphrase("passphrase-B", &salt);
        assert_ne!(k1, k2, "different passphrases must produce different keys");
    }

    #[test]
    fn derive_key_from_passphrase_deterministic() {
        let salt = [0x99_u8; 32];
        let k1 = derive_key_from_passphrase("my-secret", &salt);
        let k2 = derive_key_from_passphrase("my-secret", &salt);
        assert_eq!(k1, k2, "same inputs must produce same key");
    }

    #[test]
    fn encrypt_with_passphrase_roundtrip() {
        let passphrase = "correct-horse-battery-staple";
        let plaintext = b"sensitive key material";
        let encrypted = encrypt_with_passphrase(passphrase, plaintext).unwrap();
        assert_ne!(&encrypted[..], plaintext);
        let decrypted = decrypt_with_passphrase(passphrase, &encrypted).unwrap();
        assert_eq!(&decrypted[..], plaintext);
    }

    #[test]
    fn encrypt_with_passphrase_different_ciphertexts() {
        let passphrase = "same-pass";
        let plaintext = b"same data";
        let c1 = encrypt_with_passphrase(passphrase, plaintext).unwrap();
        let c2 = encrypt_with_passphrase(passphrase, plaintext).unwrap();
        assert_ne!(c1, c2, "random salt should produce different ciphertexts");
    }

    #[test]
    fn decrypt_with_passphrase_wrong_passphrase_fails() {
        let encrypted = encrypt_with_passphrase("correct-password", b"secret").unwrap();
        assert!(
            decrypt_with_passphrase("wrong-password", &encrypted).is_err(),
            "wrong passphrase must fail decryption"
        );
    }

    #[test]
    fn decrypt_with_passphrase_tampered_data_fails() {
        let mut encrypted = encrypt_with_passphrase("my-pass", b"secret").unwrap();
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF;
        assert!(decrypt_with_passphrase("my-pass", &encrypted).is_err());
    }

    #[test]
    fn decrypt_with_passphrase_too_short_fails() {
        assert!(decrypt_with_passphrase("pass", &[0u8; 10]).is_err());
        assert!(decrypt_with_passphrase("pass", &[0u8; 50]).is_err());
    }

    #[test]
    fn passphrase_output_format_includes_salt() {
        let encrypted = encrypt_with_passphrase("pass", b"hello").unwrap();
        // Format: [salt(32) | nonce(12) | ciphertext + tag(16+)]
        assert!(encrypted.len() > SALT_SIZE + NONCE_SIZE + 16);
        // First 32 bytes are the salt — different per call
        let encrypted2 = encrypt_with_passphrase("pass", b"hello").unwrap();
        assert_ne!(&encrypted[..SALT_SIZE], &encrypted2[..SALT_SIZE]);
    }
}
