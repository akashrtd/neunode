use crate::cf;
use crate::db::NeunodeDb;
use crate::error::{Result, StorageError};

pub struct IdentityStore<'a> {
    db: &'a NeunodeDb,
}

impl<'a> IdentityStore<'a> {
    pub fn new(db: &'a NeunodeDb) -> IdentityStore<'a> {
        IdentityStore { db }
    }

    pub fn put<V: serde::Serialize>(&self, did: &str, document: &V) -> Result<()> {
        let key_bytes =
            bincode::serialize(did).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let value_bytes =
            bincode::serialize(document).map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.db.put_raw(cf::CF_IDENTITY, &key_bytes, &value_bytes)
    }

    pub fn get<V: serde::de::DeserializeOwned>(&self, did: &str) -> Result<Option<V>> {
        let key_bytes =
            bincode::serialize(did).map_err(|e| StorageError::Serialization(e.to_string()))?;
        match self.db.get_raw(cf::CF_IDENTITY, &key_bytes)? {
            Some(bytes) => {
                let value: V = bincode::deserialize(&bytes)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub fn delete(&self, did: &str) -> Result<()> {
        let key_bytes =
            bincode::serialize(did).map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.db.delete(cf::CF_IDENTITY, &key_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NeunodeDb;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "neunode_storage_ident_{:?}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    struct AgentDoc {
        name: String,
        version: u32,
        capabilities: Vec<String>,
    }

    #[test]
    fn test_put_and_get() {
        let db = temp_db();
        let store = IdentityStore::new(&db);
        let doc = AgentDoc {
            name: "test-agent".to_string(),
            version: 1,
            capabilities: vec!["inference".to_string(), "training".to_string()],
        };

        store.put("did:neunode:agent1", &doc).unwrap();

        let fetched: Option<AgentDoc> = store.get("did:neunode:agent1").unwrap();
        assert_eq!(fetched, Some(doc));
    }

    #[test]
    fn test_get_missing() {
        let db = temp_db();
        let store = IdentityStore::new(&db);
        let result: Option<AgentDoc> = store.get("did:neunode:nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete() {
        let db = temp_db();
        let store = IdentityStore::new(&db);
        let doc = AgentDoc { name: "deleteme".to_string(), version: 2, capabilities: vec![] };

        store.put("did:neunode:del", &doc).unwrap();
        assert!(store.get::<AgentDoc>("did:neunode:del").unwrap().is_some());

        store.delete("did:neunode:del").unwrap();
        assert!(store.get::<AgentDoc>("did:neunode:del").unwrap().is_none());
    }

    #[test]
    fn test_overwrite() {
        let db = temp_db();
        let store = IdentityStore::new(&db);

        let doc_v1 = AgentDoc { name: "agent".to_string(), version: 1, capabilities: vec![] };
        let doc_v2 = AgentDoc {
            name: "agent".to_string(),
            version: 2,
            capabilities: vec!["updated".to_string()],
        };

        store.put("did:neunode:ow", &doc_v1).unwrap();
        store.put("did:neunode:ow", &doc_v2).unwrap();

        let fetched: AgentDoc = store.get("did:neunode:ow").unwrap().unwrap();
        assert_eq!(fetched.version, 2);
        assert_eq!(fetched.capabilities, vec!["updated".to_string()]);
    }

    #[test]
    fn test_string_value() {
        let db = temp_db();
        let store = IdentityStore::new(&db);

        store.put("did:neunode:str", &"just a string").unwrap();
        let fetched: Option<String> = store.get("did:neunode:str").unwrap();
        assert_eq!(fetched, Some("just a string".to_string()));
    }
}
