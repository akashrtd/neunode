use ed25519_dalek::VerifyingKey as Ed25519VerifyingKey;
use k256::ecdsa::SigningKey as Secp256k1SigningKey;
use neunode_core::{Did, NeunodeError, Result};
use neunode_crypto::ed25519;
use neunode_crypto::secp256k1;
use serde::{Deserialize, Serialize};

use crate::did::{generate_did_key, generate_did_neunode};
use ts_rs::TS;

/// Public key bundle — serializable, contains NO private key material.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct PublicKeyBundle {
    pub ed25519: Vec<u8>,
    pub secp256k1: Vec<u8>,
    pub did: Did,
}

/// A signed key rotation message proving the new keyring is authorized by the old one.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct KeyRotation {
    pub old_did: Did,
    #[serde(default)]
    pub old_ed25519_public: Vec<u8>,
    #[serde(default)]
    pub old_secp256k1_public: Vec<u8>,
    pub new_did: Did,
    pub new_ed25519_public: Vec<u8>,
    pub new_secp256k1_public: Vec<u8>,
    pub timestamp: u64,
    pub ed25519_signature: Vec<u8>,
    pub secp256k1_signature: Vec<u8>,
    #[serde(default)]
    pub new_ed25519_signature: Vec<u8>,
    #[serde(default)]
    pub new_secp256k1_signature: Vec<u8>,
}

/// Dual-key keyring holding Ed25519 (P2P) and secp256k1 (on-chain) keypairs.
///
/// Both key types are stored as their native signing key structs, ensuring
/// private key material is zeroized on drop (`ed25519_dalek::SigningKey` and
/// `k256::ecdsa::SigningKey` both implement `Drop` via the `zeroize` crate).
pub struct Keyring {
    ed25519_signing: ed25519_dalek::SigningKey,
    secp256k1_signing: Secp256k1SigningKey,
}

impl Keyring {
    /// Generate a fresh random dual-key keyring.
    pub fn generate() -> Self {
        let (ed_sk, _) = ed25519::generate_keypair();
        let (secp_sk, _) = secp256k1::generate_keypair();
        Self { ed25519_signing: ed_sk, secp256k1_signing: secp_sk }
    }

    /// Reconstruct a keyring from raw private key bytes.
    ///
    /// Validates both keys on construction. The parsed `SigningKey` structs
    /// are stored directly, so no re-parsing is needed for subsequent operations.
    pub fn from_bytes(ed25519_bytes: &[u8; 32], secp256k1_bytes: &[u8; 32]) -> Result<Self> {
        let ed_sk = ed25519::signing_key_from_bytes(ed25519_bytes)
            .map_err(|e| NeunodeError::CryptoError(format!("invalid ed25519 key: {e}")))?;
        let secp_sk = secp256k1::signing_key_from_bytes(secp256k1_bytes)
            .map_err(|e| NeunodeError::CryptoError(format!("invalid secp256k1 key: {e}")))?;
        Ok(Self { ed25519_signing: ed_sk, secp256k1_signing: secp_sk })
    }

    /// Ed25519 verifying (public) key.
    pub fn ed25519_public_key(&self) -> Ed25519VerifyingKey {
        self.ed25519_signing.verifying_key()
    }

    /// secp256k1 verifying (public) key, as uncompressed SEC1 bytes (65 bytes).
    pub fn secp256k1_public_key(&self) -> Vec<u8> {
        let vk = self.secp256k1_signing.verifying_key();
        vk.to_encoded_point(false).as_bytes().to_vec()
    }

    /// Ethereum address derived from the secp256k1 key (0x-prefixed, lowercase).
    pub fn ethereum_address(&self) -> String {
        let vk = self.secp256k1_signing.verifying_key();
        let addr = secp256k1::verifying_key_to_address(vk);
        format!("0x{}", bytes_to_hex(&addr))
    }

    /// Sign a message with Ed25519.
    pub fn sign_ed25519(&self, message: &[u8]) -> ed25519_dalek::Signature {
        ed25519::sign(&self.ed25519_signing, message)
    }

    /// Sign a domain-separated payload with Ed25519.
    pub fn sign_ed25519_domain(&self, domain: &[u8], message: &[u8]) -> ed25519_dalek::Signature {
        ed25519::sign_domain(&self.ed25519_signing, domain, message)
    }

    /// Sign a message with secp256k1 (raw ECDSA, SHA-256 challenge).
    pub fn sign_secp256k1(&self, message: &[u8]) -> neunode_crypto::secp256k1::Signature {
        secp256k1::sign_message(&self.secp256k1_signing, message)
    }

    /// Return the `did:neunode` derived from the secp256k1 Ethereum address.
    pub fn to_did(&self) -> Did {
        generate_did_neunode(&self.ethereum_address())
    }

    /// Also derive the bootstrap `did:key` from the Ed25519 key.
    pub fn to_did_key(&self) -> Did {
        generate_did_key(&self.ed25519_public_key())
    }

    /// Export public keys and DID (no private material).
    pub fn export_public(&self) -> PublicKeyBundle {
        PublicKeyBundle {
            ed25519: self.ed25519_public_key().to_bytes().to_vec(),
            secp256k1: self.secp256k1_public_key(),
            did: self.to_did(),
        }
    }

    /// Export both private keys as raw bytes.
    ///
    /// **Warning:** The returned bytes contain sensitive private key material.
    /// The caller is responsible for zeroizing these bytes after use.
    pub fn to_bytes(&self) -> (Vec<u8>, Vec<u8>) {
        let ed = ed25519::signing_key_to_bytes(&self.ed25519_signing).to_vec();
        let secp = secp256k1::signing_key_to_bytes(&self.secp256k1_signing).to_vec();
        (ed, secp)
    }

    /// Export a 64-byte recovery seed (ed25519 || secp256k1).
    /// This seed can reconstruct the entire keyring via `from_recovery_seed`.
    pub fn to_recovery_seed(&self) -> [u8; 64] {
        let (ed, secp) = self.to_bytes();
        let mut seed = [0u8; 64];
        seed[..32].copy_from_slice(&ed);
        seed[32..].copy_from_slice(&secp);
        seed
    }

    /// Reconstruct keyring from a 64-byte recovery seed.
    pub fn from_recovery_seed(seed: &[u8; 64]) -> Result<Self> {
        let ed_bytes: [u8; 32] = seed[..32].try_into().unwrap();
        let secp_bytes: [u8; 32] = seed[32..].try_into().unwrap();
        Self::from_bytes(&ed_bytes, &secp_bytes)
    }

    /// Export recovery seed as hex string (128 hex chars).
    pub fn to_recovery_phrase(&self) -> String {
        let seed = self.to_recovery_seed();
        seed.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Reconstruct keyring from a hex recovery phrase.
    pub fn from_recovery_phrase(phrase: &str) -> Result<Self> {
        let bytes = hex_to_bytes(phrase)
            .map_err(|e| NeunodeError::CryptoError(format!("invalid recovery phrase: {e}")))?;
        if bytes.len() != 64 {
            return Err(NeunodeError::CryptoError(
                "recovery phrase must be 128 hex characters (64 bytes)".to_string(),
            ));
        }
        let mut seed = [0u8; 64];
        seed.copy_from_slice(&bytes);
        Self::from_recovery_seed(&seed)
    }

    /// Create a signed key rotation message from old keyring to new keyring.
    /// Both old and new keyrings must sign the transition.
    pub fn create_rotation(old: &Keyring, new: &Keyring, timestamp: u64) -> Result<KeyRotation> {
        let mut rotation = KeyRotation {
            old_did: old.to_did(),
            new_did: new.to_did(),
            old_ed25519_public: old.ed25519_public_key().to_bytes().to_vec(),
            old_secp256k1_public: old.secp256k1_public_key(),
            new_ed25519_public: new.ed25519_public_key().to_bytes().to_vec(),
            new_secp256k1_public: new.secp256k1_public_key(),
            timestamp,
            ed25519_signature: Vec::new(),
            secp256k1_signature: Vec::new(),
            new_ed25519_signature: Vec::new(),
            new_secp256k1_signature: Vec::new(),
        };
        let message = rotation_message(&rotation);
        rotation.ed25519_signature = old.sign_ed25519(&message).to_bytes().to_vec();
        rotation.secp256k1_signature = old.sign_secp256k1(&message).to_bytes().to_vec();
        rotation.new_ed25519_signature = new.sign_ed25519(&message).to_bytes().to_vec();
        rotation.new_secp256k1_signature = new.sign_secp256k1(&message).to_bytes().to_vec();
        Ok(rotation)
    }

    /// Verify authorization, DID bindings, and possession of both replacement keys.
    /// Legacy messages lacking the old keys or possession proofs fail closed.
    /// Callers must also enforce timestamp/replay policy when applying a rotation.
    pub fn verify_rotation(rotation: &KeyRotation) -> bool {
        let message = rotation_message(rotation);
        for (did, ed, secp, ed_signature, secp_signature) in [
            (
                &rotation.old_did,
                &rotation.old_ed25519_public,
                &rotation.old_secp256k1_public,
                &rotation.ed25519_signature,
                &rotation.secp256k1_signature,
            ),
            (
                &rotation.new_did,
                &rotation.new_ed25519_public,
                &rotation.new_secp256k1_public,
                &rotation.new_ed25519_signature,
                &rotation.new_secp256k1_signature,
            ),
        ] {
            let Ok(ed_bytes) = ed.as_slice().try_into() else { return false };
            let Ok(ed_key) = ed25519_dalek::VerifyingKey::from_bytes(ed_bytes) else {
                return false;
            };
            let Ok(secp_key) = k256::ecdsa::VerifyingKey::from_sec1_bytes(secp) else {
                return false;
            };
            let address =
                format!("0x{}", bytes_to_hex(&secp256k1::verifying_key_to_address(&secp_key)));
            if generate_did_neunode(&address) != *did {
                return false;
            }
            let Ok(ed_sig) = ed25519_dalek::Signature::from_slice(ed_signature) else {
                return false;
            };
            let Ok(secp_sig) = secp256k1::Signature::from_slice(secp_signature) else {
                return false;
            };
            if ed_key.verify_strict(&message, &ed_sig).is_err()
                || !secp256k1::verify_signature(&secp_key, &message, &secp_sig)
            {
                return false;
            }
        }
        rotation.old_did != rotation.new_did
    }
}

fn rotation_message(rotation: &KeyRotation) -> Vec<u8> {
    // Public keys are fixed-length after validation; JSON length delimiters prevent ambiguity.
    let mut message = b"NEUNODE_KEY_ROTATION_V2\0".to_vec();
    message.extend(
        serde_json::to_vec(&(
            &rotation.old_did,
            &rotation.new_did,
            &rotation.old_ed25519_public,
            &rotation.old_secp256k1_public,
            &rotation.new_ed25519_public,
            &rotation.new_secp256k1_public,
            rotation.timestamp,
        ))
        .expect("public key rotation tuple is serializable"),
    );
    message
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_to_bytes(hex: &str) -> std::result::Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err("odd length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_rejects_forged_signatures_keys_identity_and_timestamp() {
        let old = Keyring::generate();
        let new = Keyring::generate();
        let valid = Keyring::create_rotation(&old, &new, 123).unwrap();
        assert!(Keyring::verify_rotation(&valid));
        let mut forged = valid.clone();
        forged.ed25519_signature = vec![0; 64];
        assert!(!Keyring::verify_rotation(&forged));
        let mut forged = valid.clone();
        forged.secp256k1_signature[0] ^= 1;
        assert!(!Keyring::verify_rotation(&forged));
        let mut forged = valid.clone();
        forged.new_ed25519_signature = vec![0; 64];
        assert!(!Keyring::verify_rotation(&forged));
        let mut forged = valid.clone();
        forged.old_did = Keyring::generate().to_did();
        assert!(!Keyring::verify_rotation(&forged));
        let mut forged = valid.clone();
        forged.new_secp256k1_public = old.secp256k1_public_key();
        assert!(!Keyring::verify_rotation(&forged));
        let mut forged = valid.clone();
        forged.timestamp += 1;
        assert!(!Keyring::verify_rotation(&forged));
        let mut legacy = valid;
        legacy.old_ed25519_public.clear();
        assert!(!Keyring::verify_rotation(&legacy));
    }

    #[test]
    fn generate_creates_valid_keyring() {
        let kr = Keyring::generate();
        assert_eq!(kr.ed25519_public_key().to_bytes().len(), 32);
        assert_eq!(kr.secp256k1_public_key().len(), 65);
        assert!(kr.ethereum_address().starts_with("0x"));
        assert!(kr.to_did().is_neunode());
    }

    #[test]
    fn ed25519_sign_verify_roundtrip() {
        let kr = Keyring::generate();
        let msg = b"hello neunode";
        let sig = kr.sign_ed25519(msg);
        assert!(ed25519::verify(&kr.ed25519_public_key(), msg, &sig));
    }

    #[test]
    fn ed25519_wrong_message_fails() {
        let kr = Keyring::generate();
        let sig = kr.sign_ed25519(b"message A");
        assert!(!ed25519::verify(&kr.ed25519_public_key(), b"message B", &sig));
    }

    #[test]
    fn secp256k1_sign_verify_roundtrip() {
        let kr = Keyring::generate();
        let msg = b"hello ethereum";
        let sig = kr.sign_secp256k1(msg);
        let vk = *kr.secp256k1_signing.verifying_key();
        assert!(secp256k1::verify_signature(&vk, msg, &sig));
    }

    #[test]
    fn secp256k1_wrong_message_fails() {
        let kr = Keyring::generate();
        let sig = kr.sign_secp256k1(b"message A");
        let vk = *kr.secp256k1_signing.verifying_key();
        assert!(!secp256k1::verify_signature(&vk, b"message B", &sig));
    }

    #[test]
    fn ethereum_address_format() {
        let kr = Keyring::generate();
        let addr = kr.ethereum_address();
        assert!(addr.starts_with("0x"));
        assert_eq!(addr.len(), 42);
        assert!(addr[2..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn ethereum_address_deterministic() {
        let kr = Keyring::generate();
        let a1 = kr.ethereum_address();
        let a2 = kr.ethereum_address();
        assert_eq!(a1, a2);
    }

    #[test]
    fn to_did_returns_neunode() {
        let kr = Keyring::generate();
        let did = kr.to_did();
        assert!(did.is_neunode());
        assert!(did.as_str().contains(&kr.ethereum_address()));
    }

    #[test]
    fn to_did_key_returns_key() {
        let kr = Keyring::generate();
        let did = kr.to_did_key();
        assert!(did.is_key());
        assert!(did.as_str().starts_with("did:key:z6Mk"));
    }

    #[test]
    fn export_public_no_private_keys() {
        let kr = Keyring::generate();
        let bundle = kr.export_public();
        assert_eq!(bundle.ed25519.len(), 32);
        assert_eq!(bundle.secp256k1.len(), 65);
        assert_eq!(bundle.did, kr.to_did());
    }

    #[test]
    fn export_public_serializable() {
        let kr = Keyring::generate();
        let bundle = kr.export_public();
        let json = serde_json::to_string(&bundle).expect("serialize");
        let back: PublicKeyBundle = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(bundle, back);
    }

    #[test]
    fn to_bytes_from_bytes_roundtrip() {
        let kr = Keyring::generate();
        let (ed, secp) = kr.to_bytes();
        assert_eq!(ed.len(), 32);
        assert_eq!(secp.len(), 32);

        let ed_arr: [u8; 32] = ed.try_into().unwrap();
        let secp_arr: [u8; 32] = secp.try_into().unwrap();
        let kr2 = Keyring::from_bytes(&ed_arr, &secp_arr).expect("from_bytes");

        assert_eq!(kr.ed25519_public_key().to_bytes(), kr2.ed25519_public_key().to_bytes());
        assert_eq!(kr.ethereum_address(), kr2.ethereum_address());
    }

    #[test]
    fn from_bytes_invalid_secp256k1_fails() {
        let ed_bytes = [1u8; 32];
        let bad_secp = [0xFFu8; 32]; // scalar > curve order
        assert!(Keyring::from_bytes(&ed_bytes, &bad_secp).is_err());
    }

    #[test]
    fn different_keyrings_produce_different_dids() {
        let kr1 = Keyring::generate();
        let kr2 = Keyring::generate();
        assert_ne!(kr1.to_did(), kr2.to_did());
        assert_ne!(kr1.to_did_key(), kr2.to_did_key());
    }

    #[test]
    fn secp256k1_public_key_is_uncompressed() {
        let kr = Keyring::generate();
        let pk_bytes = kr.secp256k1_public_key();
        assert_eq!(pk_bytes.len(), 65);
        assert_eq!(pk_bytes[0], 0x04); // uncompressed SEC1 prefix
    }

    // ─── Recovery & Rotation ──────────────────────────────────────────────

    #[test]
    fn recovery_seed_roundtrip() {
        let kr = Keyring::generate();
        let seed = kr.to_recovery_seed();
        let kr2 = Keyring::from_recovery_seed(&seed).unwrap();
        assert_eq!(kr.ed25519_public_key().to_bytes(), kr2.ed25519_public_key().to_bytes());
        assert_eq!(kr.ethereum_address(), kr2.ethereum_address());
    }

    #[test]
    fn recovery_phrase_roundtrip() {
        let kr = Keyring::generate();
        let phrase = kr.to_recovery_phrase();
        assert_eq!(phrase.len(), 128);
        assert!(phrase.chars().all(|c| c.is_ascii_hexdigit()));
        let kr2 = Keyring::from_recovery_phrase(&phrase).unwrap();
        assert_eq!(kr.to_did(), kr2.to_did());
    }

    #[test]
    fn recovery_phrase_invalid_length() {
        let result = Keyring::from_recovery_phrase("abcd");
        assert!(result.is_err());
    }

    #[test]
    fn recovery_phrase_invalid_hex() {
        let result = Keyring::from_recovery_phrase(&"zz".repeat(64));
        assert!(result.is_err());
    }

    #[test]
    fn key_rotation_creates_valid_message() {
        let old = Keyring::generate();
        let new = Keyring::generate();
        let rotation = Keyring::create_rotation(&old, &new, 1700000000).unwrap();
        assert_eq!(rotation.old_did, old.to_did());
        assert_eq!(rotation.new_did, new.to_did());
        assert_eq!(rotation.new_ed25519_public.len(), 32);
        assert_eq!(rotation.new_secp256k1_public.len(), 65);
        assert_eq!(rotation.ed25519_signature.len(), 64);
        assert_eq!(rotation.secp256k1_signature.len(), 64);
    }

    #[test]
    fn verify_rotation_valid() {
        let old = Keyring::generate();
        let new = Keyring::generate();
        let rotation = Keyring::create_rotation(&old, &new, 1700000000).unwrap();
        assert!(Keyring::verify_rotation(&rotation));
    }
}
