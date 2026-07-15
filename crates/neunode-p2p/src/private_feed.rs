use neunode_crypto::{aead, hash};

use crate::error::{P2pError, Result};

const MAGIC: &[u8; 4] = b"NNPF";
const VERSION: u8 = 1;
const HEADER_SIZE: usize = MAGIC.len() + 1 + KEY_ID_SIZE;
const AEAD_OVERHEAD: usize = 12 + 16;
const MIN_FRAME_SIZE: usize = HEADER_SIZE + AEAD_OVERHEAD;
const KEY_ID_DOMAIN: &[u8] = b"neunode-private-feed-key-v1";
const AAD_DOMAIN: &[u8] = b"neunode-private-feed-payload-v1";
pub const KEY_ID_SIZE: usize = 16;
pub const MAX_PRIVATE_FEED_FRAME_SIZE: usize = 1024 * 1024;
pub const MAX_PRIVATE_FEED_PLAINTEXT_SIZE: usize = MAX_PRIVATE_FEED_FRAME_SIZE - MIN_FRAME_SIZE;

pub type PrivateFeedKey = [u8; 32];
pub type PrivateFeedKeyId = [u8; KEY_ID_SIZE];

/// Versioned end-to-end encrypted payload carried inside an otherwise public gossip topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateFeedEnvelope {
    key_id: PrivateFeedKeyId,
    ciphertext: Vec<u8>,
}

impl PrivateFeedEnvelope {
    /// Encrypt a payload and bind it to its exact gossip topic.
    pub fn seal(topic: &str, key: &PrivateFeedKey, plaintext: &[u8]) -> Result<Self> {
        validate_key(key)?;
        validate_topic(topic)?;
        if plaintext.is_empty() {
            return Err(P2pError::EncryptionError("private payload is empty".to_string()));
        }
        if plaintext.len() > MAX_PRIVATE_FEED_PLAINTEXT_SIZE {
            return Err(P2pError::EncryptionError(format!(
                "private payload size {} exceeds max {}",
                plaintext.len(),
                MAX_PRIVATE_FEED_PLAINTEXT_SIZE
            )));
        }
        let aad = associated_data(topic);
        let ciphertext = aead::encrypt_with_aad(key, plaintext, &aad)
            .map_err(|error| P2pError::EncryptionError(error.to_string()))?;
        Ok(Self { key_id: key_id(key), ciphertext })
    }

    /// Parse an encrypted frame without attempting decryption.
    pub fn decode(frame: &[u8]) -> Result<Self> {
        if frame.len() < MIN_FRAME_SIZE || frame.len() > MAX_PRIVATE_FEED_FRAME_SIZE {
            return Err(P2pError::WireFormat("invalid private feed frame size".to_string()));
        }
        if &frame[..MAGIC.len()] != MAGIC {
            return Err(P2pError::WireFormat("invalid private feed magic".to_string()));
        }
        if frame[MAGIC.len()] != VERSION {
            return Err(P2pError::WireFormat(format!(
                "unsupported private feed version {}",
                frame[MAGIC.len()]
            )));
        }
        let mut key_id = [0u8; KEY_ID_SIZE];
        key_id.copy_from_slice(&frame[MAGIC.len() + 1..HEADER_SIZE]);
        Ok(Self { key_id, ciphertext: frame[HEADER_SIZE..].to_vec() })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut frame = Vec::with_capacity(HEADER_SIZE + self.ciphertext.len());
        frame.extend_from_slice(MAGIC);
        frame.push(VERSION);
        frame.extend_from_slice(&self.key_id);
        frame.extend_from_slice(&self.ciphertext);
        frame
    }

    /// Authenticate and decrypt this envelope for the topic where it was received.
    pub fn open(&self, topic: &str, key: &PrivateFeedKey) -> Result<Vec<u8>> {
        validate_key(key)?;
        validate_topic(topic)?;
        if key_id(key) != self.key_id {
            return Err(P2pError::EncryptionError("private feed key does not match key id".into()));
        }
        aead::decrypt_with_aad(key, &self.ciphertext, &associated_data(topic))
            .map_err(|_| P2pError::EncryptionError("private feed authentication failed".into()))
    }

    pub fn key_id(&self) -> PrivateFeedKeyId {
        self.key_id
    }
}

pub fn is_private_feed_frame(payload: &[u8]) -> bool {
    payload.starts_with(MAGIC)
}

pub fn key_id(key: &PrivateFeedKey) -> PrivateFeedKeyId {
    let digest = hash::blake3_hash_domain(KEY_ID_DOMAIN, key);
    let mut id = [0u8; KEY_ID_SIZE];
    id.copy_from_slice(&digest[..KEY_ID_SIZE]);
    id
}

fn associated_data(topic: &str) -> Vec<u8> {
    let mut aad = Vec::with_capacity(AAD_DOMAIN.len() + 4 + topic.len());
    aad.extend_from_slice(AAD_DOMAIN);
    aad.extend_from_slice(&(topic.len() as u32).to_be_bytes());
    aad.extend_from_slice(topic.as_bytes());
    aad
}

fn validate_key(key: &PrivateFeedKey) -> Result<()> {
    if key.iter().all(|byte| *byte == 0) {
        return Err(P2pError::EncryptionError("private feed key cannot be all zero".into()));
    }
    Ok(())
}

fn validate_topic(topic: &str) -> Result<()> {
    if topic.is_empty() {
        return Err(P2pError::EncryptionError("private feed topic is empty".into()));
    }
    if topic.len() > u32::MAX as usize {
        return Err(P2pError::EncryptionError("private feed topic is too long".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: PrivateFeedKey = [0x42; 32];

    #[test]
    fn envelope_roundtrip() {
        let sealed =
            PrivateFeedEnvelope::seal("neunode/inference", &KEY, b"proprietary prompt").unwrap();
        let decoded = PrivateFeedEnvelope::decode(&sealed.encode()).unwrap();
        assert_eq!(decoded.key_id(), key_id(&KEY));
        assert_eq!(decoded.open("neunode/inference", &KEY).unwrap(), b"proprietary prompt");
    }

    #[test]
    fn ciphertext_is_randomized_and_hides_plaintext() {
        let first =
            PrivateFeedEnvelope::seal("neunode/training", &KEY, b"secret").unwrap().encode();
        let second =
            PrivateFeedEnvelope::seal("neunode/training", &KEY, b"secret").unwrap().encode();
        assert_ne!(first, second);
        assert!(!first.windows(6).any(|window| window == b"secret"));
    }

    #[test]
    fn topic_is_cryptographically_bound() {
        let sealed = PrivateFeedEnvelope::seal("neunode/inference", &KEY, b"secret").unwrap();
        assert!(sealed.open("neunode/training", &KEY).is_err());
    }

    #[test]
    fn wrong_key_is_rejected_before_decryption() {
        let sealed = PrivateFeedEnvelope::seal("neunode/inference", &KEY, b"secret").unwrap();
        let error = sealed.open("neunode/inference", &[0x43; 32]).unwrap_err();
        assert!(error.to_string().contains("key does not match"));
    }

    #[test]
    fn tampering_is_rejected() {
        let sealed = PrivateFeedEnvelope::seal("neunode/inference", &KEY, b"secret").unwrap();
        let mut frame = sealed.encode();
        let last = frame.len() - 1;
        frame[last] ^= 1;
        let decoded = PrivateFeedEnvelope::decode(&frame).unwrap();
        assert!(decoded.open("neunode/inference", &KEY).is_err());
    }

    #[test]
    fn malformed_and_future_frames_are_rejected() {
        assert!(PrivateFeedEnvelope::decode(b"NNPF").is_err());
        let mut frame = PrivateFeedEnvelope::seal("topic", &KEY, b"secret").unwrap().encode();
        frame[4] = 2;
        assert!(PrivateFeedEnvelope::decode(&frame).is_err());
    }

    #[test]
    fn private_frame_detection_preserves_public_payloads() {
        let private = PrivateFeedEnvelope::seal("topic", &KEY, b"secret").unwrap().encode();
        assert!(is_private_feed_frame(&private));
        assert!(!is_private_feed_frame(b"ordinary public feed payload"));
    }

    #[test]
    fn rejects_empty_payload_and_zero_key() {
        assert!(PrivateFeedEnvelope::seal("topic", &KEY, b"").is_err());
        assert!(PrivateFeedEnvelope::seal("topic", &[0; 32], b"secret").is_err());
    }
}
