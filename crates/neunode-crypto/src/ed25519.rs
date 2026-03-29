use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;

use crate::CryptoError;

pub use ed25519_dalek::Signature;

pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

pub fn keypair_from_seed(seed: &[u8; 32]) -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::from_bytes(seed);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

pub fn sign(signing_key: &SigningKey, message: &[u8]) -> Signature {
    signing_key.sign(message)
}

pub fn verify(verifying_key: &VerifyingKey, message: &[u8], signature: &Signature) -> bool {
    verifying_key.verify(message, signature).is_ok()
}

pub fn sign_domain(signing_key: &SigningKey, domain: &[u8], data: &[u8]) -> Signature {
    let message = crate::hash::sha256_domain(domain, data);
    signing_key.sign(&message)
}

pub fn verify_domain(
    verifying_key: &VerifyingKey,
    domain: &[u8],
    data: &[u8],
    signature: &Signature,
) -> bool {
    let message = crate::hash::sha256_domain(domain, data);
    verifying_key.verify(&message, signature).is_ok()
}

pub fn signing_key_to_bytes(key: &SigningKey) -> [u8; 32] {
    key.to_bytes()
}

pub fn signing_key_from_bytes(bytes: &[u8; 32]) -> Result<SigningKey, CryptoError> {
    Ok(SigningKey::from_bytes(bytes))
}

pub fn verifying_key_to_bytes(key: &VerifyingKey) -> [u8; 32] {
    key.to_bytes()
}

pub fn verifying_key_from_bytes(bytes: &[u8; 32]) -> Result<VerifyingKey, CryptoError> {
    VerifyingKey::from_bytes(bytes).map_err(|e| CryptoError::InvalidKey(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify_roundtrip() {
        let (sk, vk) = generate_keypair();
        let message = b"hello neunode";
        let sig = sign(&sk, message);
        assert!(verify(&vk, message, &sig));
    }

    #[test]
    fn wrong_key_fails() {
        let (sk1, _) = generate_keypair();
        let (_, vk2) = generate_keypair();
        let message = b"hello neunode";
        let sig = sign(&sk1, message);
        assert!(!verify(&vk2, message, &sig));
    }

    #[test]
    fn wrong_message_fails() {
        let (sk, vk) = generate_keypair();
        let sig = sign(&sk, b"message A");
        assert!(!verify(&vk, b"message B", &sig));
    }

    #[test]
    fn domain_sign_differs_from_raw() {
        let (sk, _) = generate_keypair();
        let data = b"test data";
        let raw_sig = sign(&sk, data);
        let domain_sig = sign_domain(&sk, crate::hash::DOMAIN_FEED_EVENT, data);
        assert_ne!(raw_sig, domain_sig);
    }

    #[test]
    fn domain_sign_verify_roundtrip() {
        let (sk, vk) = generate_keypair();
        let data = b"feed event data";
        let sig = sign_domain(&sk, crate::hash::DOMAIN_FEED_EVENT, data);
        assert!(verify_domain(&vk, crate::hash::DOMAIN_FEED_EVENT, data, &sig,));
    }

    #[test]
    fn domain_wrong_domain_fails() {
        let (sk, vk) = generate_keypair();
        let data = b"feed event data";
        let sig = sign_domain(&sk, crate::hash::DOMAIN_FEED_EVENT, data);
        assert!(!verify_domain(&vk, crate::hash::DOMAIN_ATTESTATION, data, &sig,));
    }

    #[test]
    fn deterministic_keygen_from_seed() {
        let seed = [42u8; 32];
        let (sk1, vk1) = keypair_from_seed(&seed);
        let (sk2, vk2) = keypair_from_seed(&seed);
        assert_eq!(signing_key_to_bytes(&sk1), signing_key_to_bytes(&sk2));
        assert_eq!(verifying_key_to_bytes(&vk1), verifying_key_to_bytes(&vk2));
    }

    #[test]
    fn different_seeds_produce_different_keys() {
        let seed1 = [1u8; 32];
        let seed2 = [2u8; 32];
        let (_, vk1) = keypair_from_seed(&seed1);
        let (_, vk2) = keypair_from_seed(&seed2);
        assert_ne!(verifying_key_to_bytes(&vk1), verifying_key_to_bytes(&vk2));
    }

    #[test]
    fn signing_key_bytes_roundtrip() {
        let (sk, _) = generate_keypair();
        let bytes = signing_key_to_bytes(&sk);
        let restored = signing_key_from_bytes(&bytes).unwrap();
        assert_eq!(signing_key_to_bytes(&restored), bytes);
    }

    #[test]
    fn verifying_key_bytes_roundtrip() {
        let (_, vk) = generate_keypair();
        let bytes = verifying_key_to_bytes(&vk);
        let restored = verifying_key_from_bytes(&bytes).unwrap();
        assert_eq!(verifying_key_to_bytes(&restored), bytes);
    }

    #[test]
    fn rfc8032_test_vector_1() {
        // RFC 8032 Ed25519 Test Vector 1
        let seed_hex = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        let seed: [u8; 32] = {
            let mut s = [0u8; 32];
            let bytes = hex::decode(seed_hex).unwrap();
            s.copy_from_slice(&bytes);
            s
        };

        let expected_pub_hex = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let expected_sig_hex = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";

        let (sk, vk) = keypair_from_seed(&seed);
        let pub_bytes = verifying_key_to_bytes(&vk);
        assert_eq!(hex::encode(pub_bytes), expected_pub_hex);

        let message = b"";
        let sig = sign(&sk, message);
        assert_eq!(hex::encode(sig.to_bytes()), expected_sig_hex);
        assert!(verify(&vk, message, &sig));
    }

    #[test]
    fn rfc8032_test_vector_2_roundtrip() {
        // RFC 8032 Ed25519 Test Vector 2 seed and expected pubkey
        let seed_hex = "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb";
        let seed: [u8; 32] = {
            let mut s = [0u8; 32];
            let bytes = hex::decode(seed_hex).unwrap();
            s.copy_from_slice(&bytes);
            s
        };

        let expected_pub_hex = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";

        let (sk, vk) = keypair_from_seed(&seed);
        let pub_bytes = verifying_key_to_bytes(&vk);
        assert_eq!(hex::encode(pub_bytes), expected_pub_hex);

        let message = [0x72u8];
        let sig = sign(&sk, &message);
        assert!(verify(&vk, &message, &sig));
    }

    #[test]
    fn verifying_key_from_invalid_bytes_fails() {
        // 0xFF in last byte is non-canonical per RFC 8032 §5.1.2
        let mut bad = [0u8; 32];
        bad[31] = 0xFF;
        let result = verifying_key_from_bytes(&bad);
        assert!(result.is_err());
    }
}
