use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::cf::CF_AUDIT_LOG;
use crate::db::NeunodeDb;
use crate::error::{Result, StorageError};

const GENESIS_HASH: [u8; 32] = [0; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditOutcome {
    Success,
    Failure,
}

/// A durable security-relevant event. Entries are ordered by sequence and
/// linked by hash so deletion, insertion, or modification is detectable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub sequence: u64,
    pub timestamp: u64,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub outcome: AuditOutcome,
    pub details: BTreeMap<String, String>,
    pub previous_hash: [u8; 32],
    pub entry_hash: [u8; 32],
}

#[derive(Serialize)]
struct AuditHashPayload<'a> {
    sequence: u64,
    timestamp: u64,
    actor: &'a str,
    action: &'a str,
    resource: &'a str,
    outcome: AuditOutcome,
    details: &'a BTreeMap<String, String>,
    previous_hash: [u8; 32],
}

pub struct NewAuditEntry {
    pub timestamp: u64,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub outcome: AuditOutcome,
    pub details: BTreeMap<String, String>,
}

pub struct AuditStore<'a> {
    db: &'a NeunodeDb,
}

impl<'a> AuditStore<'a> {
    pub fn new(db: &'a NeunodeDb) -> Self {
        Self { db }
    }

    /// Append an event while holding the shared ledger mutation lock. This
    /// makes sequence allocation safe under concurrent writers.
    pub fn append(&self, new_entry: NewAuditEntry) -> Result<AuditEntry> {
        self.db.with_ledger_write(|| {
            let (key, value, entry) = self.prepare_append(new_entry)?;
            self.db.put_raw(CF_AUDIT_LOG, &key, &value)?;
            Ok(entry)
        })
    }

    /// Prepare an audit record for inclusion in an existing ledger WriteBatch.
    /// The caller must hold the database's ledger write lock through commit.
    pub(crate) fn prepare_append(
        &self,
        new_entry: NewAuditEntry,
    ) -> Result<([u8; 8], Vec<u8>, AuditEntry)> {
        let (sequence, previous_hash) = match self.db.last_raw(CF_AUDIT_LOG)? {
            Some((key, value)) => {
                let previous = decode_entry(&key, &value)?;
                (
                    previous.sequence.checked_add(1).ok_or_else(|| {
                        StorageError::AuditVerificationFailed {
                            sequence: previous.sequence,
                            reason: "sequence exhausted".to_string(),
                        }
                    })?,
                    previous.entry_hash,
                )
            }
            None => (0, GENESIS_HASH),
        };

        let mut entry = AuditEntry {
            sequence,
            timestamp: new_entry.timestamp,
            actor: new_entry.actor,
            action: new_entry.action,
            resource: new_entry.resource,
            outcome: new_entry.outcome,
            details: new_entry.details,
            previous_hash,
            entry_hash: GENESIS_HASH,
        };
        entry.entry_hash = calculate_hash(&entry)?;
        let value = bincode::serialize(&entry)
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
        Ok((sequence.to_be_bytes(), value, entry))
    }

    pub fn entries(&self) -> Result<Vec<AuditEntry>> {
        self.raw_entries()?.iter().map(|(key, value)| decode_entry(key, value)).collect()
    }

    /// Read a bounded page in ascending sequence order.
    pub fn entries_from(&self, sequence: u64, limit: usize) -> Result<Vec<AuditEntry>> {
        self.db.scan_from_limit(CF_AUDIT_LOG, &sequence.to_be_bytes(), limit).and_then(|records| {
            records.iter().map(|(key, value)| decode_entry(key, value)).collect()
        })
    }

    /// Verify key ordering, contiguous sequences, previous-hash links, and
    /// every entry's content hash.
    pub fn verify_chain(&self) -> Result<()> {
        let mut expected_sequence = 0_u64;
        let mut expected_previous_hash = GENESIS_HASH;

        for (key, value) in self.raw_entries()? {
            let entry = decode_entry(&key, &value)?;
            if entry.sequence != expected_sequence {
                return audit_error(expected_sequence, "non-contiguous sequence");
            }
            if entry.previous_hash != expected_previous_hash {
                return audit_error(entry.sequence, "previous hash mismatch");
            }
            if entry.entry_hash != calculate_hash(&entry)? {
                return audit_error(entry.sequence, "entry hash mismatch");
            }
            expected_previous_hash = entry.entry_hash;
            expected_sequence = expected_sequence.checked_add(1).ok_or_else(|| {
                StorageError::AuditVerificationFailed {
                    sequence: entry.sequence,
                    reason: "sequence exhausted".to_string(),
                }
            })?;
        }
        Ok(())
    }

    fn raw_entries(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.db.prefix_scan(CF_AUDIT_LOG, b"")
    }
}

fn calculate_hash(entry: &AuditEntry) -> Result<[u8; 32]> {
    let payload = AuditHashPayload {
        sequence: entry.sequence,
        timestamp: entry.timestamp,
        actor: &entry.actor,
        action: &entry.action,
        resource: &entry.resource,
        outcome: entry.outcome,
        details: &entry.details,
        previous_hash: entry.previous_hash,
    };
    let bytes = bincode::serialize(&payload)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(neunode_crypto::hash::blake3_hash(&bytes))
}

fn decode_entry(key: &[u8], value: &[u8]) -> Result<AuditEntry> {
    let sequence = key.try_into().map(u64::from_be_bytes).map_err(|_| {
        StorageError::AuditVerificationFailed {
            sequence: 0,
            reason: format!("invalid key length: {}", key.len()),
        }
    })?;
    let entry: AuditEntry =
        bincode::deserialize(value).map_err(|error| StorageError::AuditVerificationFailed {
            sequence,
            reason: format!("invalid entry encoding: {error}"),
        })?;
    if entry.sequence != sequence {
        return audit_error(sequence, "key and entry sequence mismatch");
    }
    Ok(entry)
}

fn audit_error<T>(sequence: u64, reason: &str) -> Result<T> {
    Err(StorageError::AuditVerificationFailed { sequence, reason: reason.to_string() })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    use super::*;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("neunode_audit_store_{}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    fn event(timestamp: u64, action: &str) -> NewAuditEntry {
        NewAuditEntry {
            timestamp,
            actor: "did:neunode:alice".to_string(),
            action: action.to_string(),
            resource: "bounty:42".to_string(),
            outcome: AuditOutcome::Success,
            details: BTreeMap::from([("request_id".to_string(), "req-1".to_string())]),
        }
    }

    #[test]
    fn append_builds_verifiable_chain() {
        let db = temp_db();
        let store = AuditStore::new(&db);
        let first = store.append(event(10, "bounty.create")).unwrap();
        let second = store.append(event(11, "bounty.claim")).unwrap();

        assert_eq!(first.sequence, 0);
        assert_eq!(first.previous_hash, GENESIS_HASH);
        assert_eq!(second.sequence, 1);
        assert_eq!(second.previous_hash, first.entry_hash);
        assert_eq!(store.entries().unwrap(), vec![first, second]);
        store.verify_chain().unwrap();
    }

    #[test]
    fn detects_modified_entry() {
        let db = temp_db();
        let store = AuditStore::new(&db);
        store.append(event(10, "bounty.create")).unwrap();
        let mut entry = store.entries().unwrap().remove(0);
        entry.action = "bounty.delete".to_string();
        let bytes = bincode::serialize(&entry).unwrap();
        db.put_raw(CF_AUDIT_LOG, &0_u64.to_be_bytes(), &bytes).unwrap();

        assert!(matches!(
            store.verify_chain(),
            Err(StorageError::AuditVerificationFailed { sequence: 0, .. })
        ));
    }

    #[test]
    fn detects_deleted_entry() {
        let db = temp_db();
        let store = AuditStore::new(&db);
        store.append(event(10, "one")).unwrap();
        store.append(event(11, "two")).unwrap();
        db.delete(CF_AUDIT_LOG, &0_u64.to_be_bytes()).unwrap();

        assert!(matches!(
            store.verify_chain(),
            Err(StorageError::AuditVerificationFailed { sequence: 0, .. })
        ));
    }

    #[test]
    fn concurrent_appends_have_unique_contiguous_sequences() {
        let db = Arc::new(temp_db());
        let mut threads = Vec::new();
        for timestamp in 0..16 {
            let db = Arc::clone(&db);
            threads.push(std::thread::spawn(move || {
                AuditStore::new(&db).append(event(timestamp, "concurrent")).unwrap()
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }

        let store = AuditStore::new(&db);
        let entries = store.entries().unwrap();
        assert_eq!(entries.len(), 16);
        assert_eq!(
            entries.iter().map(|entry| entry.sequence).collect::<Vec<_>>(),
            (0..16).collect::<Vec<_>>()
        );
        store.verify_chain().unwrap();
    }
}
