use std::time::Duration;

use libp2p::kad::{PeerRecord, Record, RecordKey};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

use crate::error::{P2pError, Result};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DhtKey(pub Vec<u8>);

impl DhtKey {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn from_bytes_str(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_record_key(self) -> RecordKey {
        RecordKey::new(&self.0)
    }
}

impl AsRef<[u8]> for DhtKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RecordValue(pub Vec<u8>);

impl RecordValue {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn from_bytes_str(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn to_record(&self, key: DhtKey) -> Record {
        Record { key: key.into_record_key(), value: self.0.clone(), publisher: None, expires: None }
    }

    pub fn to_record_with_publisher(&self, key: DhtKey, publisher: PeerId) -> Record {
        Record {
            key: key.into_record_key(),
            value: self.0.clone(),
            publisher: Some(publisher),
            expires: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DhtManager {
    local_peer_id: String,
    replication_factor: usize,
    query_timeout_secs: u64,
}

impl DhtManager {
    pub fn new(local_peer_id: PeerId) -> Self {
        Self {
            local_peer_id: local_peer_id.to_string(),
            replication_factor: 3,
            query_timeout_secs: 30,
        }
    }

    pub fn with_replication_factor(mut self, factor: usize) -> Self {
        self.replication_factor = factor;
        self
    }

    pub fn with_query_timeout(mut self, secs: u64) -> Self {
        self.query_timeout_secs = secs;
        self
    }

    pub fn replication_factor(&self) -> usize {
        self.replication_factor
    }

    pub fn query_timeout(&self) -> Duration {
        Duration::from_secs(self.query_timeout_secs)
    }

    pub fn local_peer_id(&self) -> &str {
        &self.local_peer_id
    }

    pub fn prepare_put_record(&self, key: DhtKey, value: RecordValue) -> Record {
        value.to_record(key)
    }

    pub fn prepare_put_record_signed(
        &self,
        key: DhtKey,
        value: RecordValue,
        peer_id: PeerId,
    ) -> Record {
        value.to_record_with_publisher(key, peer_id)
    }

    pub fn extract_record_value(record: &Record) -> RecordValue {
        RecordValue::new(record.value.clone())
    }

    pub fn extract_peer_record(peer_record: &PeerRecord) -> RecordValue {
        RecordValue::new(peer_record.record.value.clone())
    }
}

pub fn validate_record_value(value: &RecordValue, max_size: usize) -> Result<()> {
    if value.0.is_empty() {
        return Err(P2pError::DhtError("record value is empty".to_string()));
    }
    if value.0.len() > max_size {
        return Err(P2pError::DhtError(format!(
            "record value size {} exceeds max {}",
            value.0.len(),
            max_size
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_peer_id() -> PeerId {
        libp2p::identity::Keypair::generate_ed25519().public().to_peer_id()
    }

    #[test]
    fn dht_key_new() {
        let key = DhtKey::new(vec![1, 2, 3]);
        assert_eq!(key.as_bytes(), &[1, 2, 3]);
    }

    #[test]
    fn dht_key_from_str() {
        let key = DhtKey::from_bytes_str("hello");
        assert_eq!(key.as_bytes(), b"hello");
    }

    #[test]
    fn dht_key_into_record_key() {
        let key = DhtKey::from_bytes_str("test-key");
        let record_key = key.into_record_key();
        assert!(!record_key.as_ref().is_empty());
    }

    #[test]
    fn dht_key_as_ref() {
        let key = DhtKey::new(vec![1, 2, 3]);
        assert_eq!(key.as_ref(), &[1, 2, 3]);
    }

    #[test]
    fn record_value_new() {
        let val = RecordValue::new(vec![4, 5, 6]);
        assert_eq!(val.as_bytes(), &[4, 5, 6]);
    }

    #[test]
    fn record_value_from_str() {
        let val = RecordValue::from_bytes_str("hello");
        assert_eq!(val.as_bytes(), b"hello");
    }

    #[test]
    fn record_value_to_record() {
        let key = DhtKey::from_bytes_str("key");
        let val = RecordValue::from_bytes_str("value");
        let record = val.to_record(key);
        assert_eq!(record.value, b"value");
        assert!(record.publisher.is_none());
    }

    #[test]
    fn record_value_to_record_with_publisher() {
        let key = DhtKey::from_bytes_str("key");
        let val = RecordValue::from_bytes_str("value");
        let peer_id = random_peer_id();
        let record = val.to_record_with_publisher(key, peer_id);
        assert_eq!(record.value, b"value");
        assert_eq!(record.publisher, Some(peer_id));
    }

    #[test]
    fn dht_manager_new() {
        let peer_id = random_peer_id();
        let manager = DhtManager::new(peer_id);
        assert_eq!(manager.replication_factor(), 3);
        assert_eq!(manager.query_timeout(), Duration::from_secs(30));
        assert!(!manager.local_peer_id().is_empty());
    }

    #[test]
    fn dht_manager_with_options() {
        let peer_id = random_peer_id();
        let manager = DhtManager::new(peer_id).with_replication_factor(5).with_query_timeout(60);
        assert_eq!(manager.replication_factor(), 5);
        assert_eq!(manager.query_timeout(), Duration::from_secs(60));
    }

    #[test]
    fn dht_manager_prepare_put_record() {
        let peer_id = random_peer_id();
        let manager = DhtManager::new(peer_id);
        let key = DhtKey::from_bytes_str("test-key");
        let val = RecordValue::from_bytes_str("test-value");
        let record = manager.prepare_put_record(key, val);
        assert_eq!(record.value, b"test-value");
    }

    #[test]
    fn dht_manager_prepare_put_record_signed() {
        let peer_id = random_peer_id();
        let manager = DhtManager::new(peer_id);
        let key = DhtKey::from_bytes_str("test-key");
        let val = RecordValue::from_bytes_str("test-value");
        let record = manager.prepare_put_record_signed(key, val, peer_id);
        assert_eq!(record.value, b"test-value");
        assert!(record.publisher.is_some());
    }

    #[test]
    fn extract_record_value_from_record() {
        let key = DhtKey::from_bytes_str("key");
        let val = RecordValue::from_bytes_str("payload");
        let record = val.to_record(key);
        let extracted = DhtManager::extract_record_value(&record);
        assert_eq!(extracted.as_bytes(), b"payload");
    }

    #[test]
    fn validate_record_value_accepts_valid() {
        let val = RecordValue::from_bytes_str("valid");
        assert!(validate_record_value(&val, 1024).is_ok());
    }

    #[test]
    fn validate_record_value_rejects_empty() {
        let val = RecordValue::new(vec![]);
        let err = validate_record_value(&val, 1024).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn validate_record_value_rejects_oversized() {
        let val = RecordValue::new(vec![0u8; 2048]);
        let err = validate_record_value(&val, 1024).unwrap_err();
        assert!(err.to_string().contains("exceeds max"));
    }

    #[test]
    fn dht_key_serde_roundtrip() {
        let key = DhtKey::new(vec![1, 2, 3]);
        let json = serde_json::to_string(&key).unwrap();
        let back: DhtKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key, back);
    }

    #[test]
    fn record_value_serde_roundtrip() {
        let val = RecordValue::from_bytes_str("test-value");
        let json = serde_json::to_string(&val).unwrap();
        let back: RecordValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, back);
    }

    #[test]
    fn dht_manager_serde_roundtrip() {
        let peer_id = random_peer_id();
        let manager = DhtManager::new(peer_id);
        let json = serde_json::to_string(&manager).unwrap();
        let back: DhtManager = serde_json::from_str(&json).unwrap();
        assert_eq!(manager, back);
    }
}
