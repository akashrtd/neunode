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
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
}
