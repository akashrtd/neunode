use k256::ecdsa::{signature::Signer, signature::Verifier, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use sha3::{Digest, Keccak256};

use crate::CryptoError;

pub use k256::ecdsa::Signature;

pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = *signing_key.verifying_key();
    (signing_key, verifying_key)
}

pub fn verifying_key_to_address(verifying_key: &VerifyingKey) -> [u8; 20] {
    let encoded = verifying_key.to_encoded_point(false);
    let bytes = encoded.as_bytes();
    // SEC1 uncompressed: [0x04, x(32), y(32)] = 65 bytes
    let hash = Keccak256::digest(&bytes[1..65]);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..32]);
    addr
}

pub fn sign_message(signing_key: &SigningKey, message: &[u8]) -> Signature {
    signing_key.sign(message)
}

pub fn verify_signature(
    verifying_key: &VerifyingKey,
    message: &[u8],
    signature: &Signature,
) -> bool {
    verifying_key.verify(message, signature).is_ok()
}

pub fn signing_key_to_bytes(key: &SigningKey) -> [u8; 32] {
    let bytes = key.to_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    out
}

pub fn signing_key_from_bytes(bytes: &[u8; 32]) -> Result<SigningKey, CryptoError> {
    SigningKey::from_bytes(bytes.into()).map_err(|e| CryptoError::InvalidKey(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify_roundtrip() {
        let (sk, vk) = generate_keypair();
        let message = b"hello ethereum";
        let sig = sign_message(&sk, message);
        assert!(verify_signature(&vk, message, &sig));
    }

    #[test]
    fn wrong_key_fails() {
        let (sk1, _) = generate_keypair();
        let (_, vk2) = generate_keypair();
        let message = b"hello ethereum";
        let sig = sign_message(&sk1, message);
        assert!(!verify_signature(&vk2, message, &sig));
    }

    #[test]
    fn wrong_message_fails() {
        let (sk, vk) = generate_keypair();
        let sig = sign_message(&sk, b"message A");
        assert!(!verify_signature(&vk, b"message B", &sig));
    }

    #[test]
    fn verifying_key_to_address_is_20_bytes() {
        let (_, vk) = generate_keypair();
        let addr = verifying_key_to_address(&vk);
        assert_eq!(addr.len(), 20);
    }

    #[test]
    fn deterministic_address_from_known_key() {
        // Known test vector: private key = 1
        let sk_bytes =
            hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap();
        let mut sk_arr = [0u8; 32];
        sk_arr.copy_from_slice(&sk_bytes);
        let sk = signing_key_from_bytes(&sk_arr).unwrap();
        let vk = *sk.verifying_key();
        let addr = verifying_key_to_address(&vk);
        // Expected Ethereum address for private key 1:
        // public key (uncompressed) x: 79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798
        //                         y: 483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8
        // keccak256(pubkey[1:]) last 20 bytes = 7E5F4552091A69125d5DfCb7b8C2659029395Bdf
        let expected = hex::decode("7e5f4552091a69125d5dfcb7b8c2659029395bdf").unwrap();
        assert_eq!(&addr[..], &expected[..]);
    }

    #[test]
    fn signing_key_bytes_roundtrip() {
        let (sk, _) = generate_keypair();
        let bytes = signing_key_to_bytes(&sk);
        let restored = signing_key_from_bytes(&bytes).unwrap();
        assert_eq!(signing_key_to_bytes(&restored), bytes);
    }

    #[test]
    fn same_key_same_address() {
        let (sk, vk) = generate_keypair();
        let addr1 = verifying_key_to_address(&vk);
        let bytes = signing_key_to_bytes(&sk);
        let sk2 = signing_key_from_bytes(&bytes).unwrap();
        let vk2 = *sk2.verifying_key();
        let addr2 = verifying_key_to_address(&vk2);
        assert_eq!(addr1, addr2);
    }
}
