use sha2::{Digest, Sha256};
use siphasher::sip128::{Hasher128, SipHasher24};
use std::hash::Hasher;

pub const DOMAIN_FEED_EVENT: &[u8; 8] = b"NNEVT001";
pub const DOMAIN_ATTESTATION: &[u8; 8] = b"NNATT001";
pub const DOMAIN_P2P_MESSAGE: &[u8; 8] = b"NNP2P001";
pub const DOMAIN_BOUNTY: &[u8; 8] = b"NNBNT001";
pub const DOMAIN_MODEL_LINEAGE: &[u8; 8] = b"NNMOD001";
pub const DOMAIN_AGENT_CARD: &[u8; 8] = b"NNACD001";

const SIPHASH_DEFAULT_KEY0: u64 = 0x_5be5_0b7d_6c3a_f093;
const SIPHASH_DEFAULT_KEY1: u64 = 0x_62b2_8e4b_f352_7a1e;

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

pub fn sha256_domain(domain: &[u8], data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

pub fn siphash24_128(key0: u64, key1: u64, data: &[u8]) -> [u8; 16] {
    let mut hasher = SipHasher24::new_with_keys(key0, key1);
    hasher.write(data);
    let hash128 = hasher.finish128();
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&hash128.h1.to_le_bytes());
    out[8..].copy_from_slice(&hash128.h2.to_le_bytes());
    out
}

pub fn siphash24_128_default(data: &[u8]) -> [u8; 16] {
    siphash24_128(SIPHASH_DEFAULT_KEY0, SIPHASH_DEFAULT_KEY1, data)
}

pub fn agent_did_hash(did: &str) -> [u8; 16] {
    siphash24_128_default(did.as_bytes())
}

pub fn multihash_sha256(data: &[u8]) -> Vec<u8> {
    let hash = sha256(data);
    let mut out = Vec::with_capacity(34);
    out.push(0x12);
    out.push(0x20);
    out.extend_from_slice(&hash);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_empty_string() {
        let expected =
            hex::decode("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
                .unwrap();
        let result = sha256(b"");
        assert_eq!(result[..], expected[..]);
    }

    #[test]
    fn sha256_abc() {
        let expected =
            hex::decode("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
                .unwrap();
        let result = sha256(b"abc");
        assert_eq!(result[..], expected[..]);
    }

    #[test]
    fn sha256_domain_separation_differs() {
        let data = b"test data";
        let h1 = sha256_domain(DOMAIN_FEED_EVENT, data);
        let h2 = sha256_domain(DOMAIN_ATTESTATION, data);
        let h3 = sha256_domain(DOMAIN_P2P_MESSAGE, data);
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h1, h3);
    }

    #[test]
    fn sha256_domain_differs_from_raw() {
        let data = b"test data";
        let raw = sha256(data);
        let domained = sha256_domain(DOMAIN_FEED_EVENT, data);
        assert_ne!(raw, domained);
    }

    #[test]
    fn siphash24_128_deterministic() {
        let key0 = 0x_0123_4567_89ab_cdef;
        let key1 = 0x_fedc_ba98_7654_3210;
        let data = b"hello world";
        let h1 = siphash24_128(key0, key1, data);
        let h2 = siphash24_128(key0, key1, data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
    }

    #[test]
    fn siphash24_128_different_keys_differ() {
        let data = b"test data";
        let h1 = siphash24_128(0, 0, data);
        let h2 = siphash24_128(1, 0, data);
        assert_ne!(h1, h2);
    }

    #[test]
    fn siphash24_128_default_deterministic() {
        let data = b"some agent data";
        let h1 = siphash24_128_default(data);
        let h2 = siphash24_128_default(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn agent_did_hash_is_16_bytes() {
        let h = agent_did_hash("did:neunode:0xABC123");
        assert_eq!(h.len(), 16);
    }

    #[test]
    fn agent_did_hash_deterministic() {
        let did = "did:neunode:0xABC123def456";
        let h1 = agent_did_hash(did);
        let h2 = agent_did_hash(did);
        assert_eq!(h1, h2);
    }

    #[test]
    fn agent_did_hash_different_dids_differ() {
        let h1 = agent_did_hash("did:neunode:agent1");
        let h2 = agent_did_hash("did:neunode:agent2");
        assert_ne!(h1, h2);
    }

    #[test]
    fn multihash_sha256_format() {
        let mh = multihash_sha256(b"hello");
        assert_eq!(mh[0], 0x12);
        assert_eq!(mh[1], 0x20);
        assert_eq!(mh.len(), 34);
    }

    #[test]
    fn multihash_sha256_empty() {
        let mh = multihash_sha256(b"");
        let expected_hash =
            hex::decode("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
                .unwrap();
        assert_eq!(&mh[2..], &expected_hash[..]);
    }

    #[test]
    fn multihash_sha256_abc() {
        let mh = multihash_sha256(b"abc");
        let expected_hash =
            hex::decode("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
                .unwrap();
        assert_eq!(&mh[2..], &expected_hash[..]);
    }

    #[test]
    fn all_domain_constants_are_8_bytes() {
        assert_eq!(DOMAIN_FEED_EVENT.len(), 8);
        assert_eq!(DOMAIN_ATTESTATION.len(), 8);
        assert_eq!(DOMAIN_P2P_MESSAGE.len(), 8);
        assert_eq!(DOMAIN_BOUNTY.len(), 8);
        assert_eq!(DOMAIN_MODEL_LINEAGE.len(), 8);
        assert_eq!(DOMAIN_AGENT_CARD.len(), 8);
    }
}
