use neunode_storage::db::NeunodeDb;

use crate::dictionary::StringDictionary;
use crate::error::Result;
use crate::ontology::*;
use crate::triple::Quad;

#[cfg(test)]
use neunode_storage::cf::CF_KG_SPOG;

/// Batch of KG mutations to apply atomically.
///
/// Each mutation is a quad to insert or delete. When [`apply`](MutationBatch::apply) is called,
/// every quad is written to (or removed from) all 6 index column families. Inserts run before
/// deletes so that a "move" (delete + re-insert with different graph) can be expressed in a
/// single batch.
pub struct MutationBatch {
    quads_to_insert: Vec<Quad>,
    quads_to_delete: Vec<Quad>,
}

impl Default for MutationBatch {
    fn default() -> Self {
        Self::new()
    }
}

impl MutationBatch {
    /// Create an empty batch.
    pub fn new() -> Self {
        Self { quads_to_insert: Vec::new(), quads_to_delete: Vec::new() }
    }

    /// Add a quad to insert.
    pub fn insert(&mut self, quad: Quad) {
        self.quads_to_insert.push(quad);
    }

    /// Add a quad to delete.
    pub fn delete(&mut self, quad: Quad) {
        self.quads_to_delete.push(quad);
    }

    /// Number of mutations (inserts + deletes).
    pub fn len(&self) -> usize {
        self.quads_to_insert.len() + self.quads_to_delete.len()
    }

    /// Whether the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Apply all mutations to the database.
    ///
    /// Each quad's 6 index entries are written atomically via [`Quad::insert_indexes`] /
    /// [`Quad::delete_indexes`]. Inserts are applied before deletes.
    pub fn apply(&self, db: &NeunodeDb) -> Result<()> {
        for quad in &self.quads_to_insert {
            quad.insert_indexes(db)?;
        }
        for quad in &self.quads_to_delete {
            quad.delete_indexes(db)?;
        }
        Ok(())
    }

    /// Clear the batch.
    pub fn clear(&mut self) {
        self.quads_to_insert.clear();
        self.quads_to_delete.clear();
    }
}

// ---------------------------------------------------------------------------
// High-level domain mutation functions
// ---------------------------------------------------------------------------

/// Register a new agent in the knowledge graph.
///
/// Creates a type assertion `(agent, rdf:type, Agent)` plus one
/// `(agent, hasCapability, cap)` quad per capability.
pub fn register_agent(
    dict: &StringDictionary,
    agent_did: &str,
    capabilities: &[&str],
) -> Result<MutationBatch> {
    let mut batch = MutationBatch::new();
    batch.insert(type_quad(dict, agent_did, CLASS_AGENT)?);
    for cap in capabilities {
        batch.insert(agent_has_capability(dict, agent_did, cap)?);
    }
    Ok(batch)
}

/// Register a model in the knowledge graph with optional parent (lineage).
///
/// Creates `(model, rdf:type, Model)` and `(owner, ownsModel, model)`.
/// If `parent_cid` is provided, also creates `(model, dependsOn, parent)`.
pub fn register_model(
    dict: &StringDictionary,
    owner_did: &str,
    model_cid: &str,
    parent_cid: Option<&str>,
) -> Result<MutationBatch> {
    let mut batch = MutationBatch::new();
    batch.insert(type_quad(dict, model_cid, CLASS_MODEL)?);
    batch.insert(agent_owns_model(dict, owner_did, model_cid)?);
    if let Some(parent) = parent_cid {
        batch.insert(model_depends_on(dict, model_cid, parent)?);
    }
    Ok(batch)
}

/// Register a bounty in the knowledge graph.
///
/// Creates `(bounty, rdf:type, Bounty)` plus one `(bounty, requiresCapability, cap)`
/// quad per required capability.
pub fn register_bounty(
    dict: &StringDictionary,
    bounty_id: &str,
    required_capabilities: &[&str],
) -> Result<MutationBatch> {
    let mut batch = MutationBatch::new();
    batch.insert(type_quad(dict, bounty_id, CLASS_BOUNTY)?);
    for cap in required_capabilities {
        batch.insert(bounty_requires_capability(dict, bounty_id, cap)?);
    }
    Ok(batch)
}

/// Register agent participation in a training job.
///
/// Creates `(agent, participatesIn, job)`.
pub fn join_training_job(
    dict: &StringDictionary,
    agent_did: &str,
    job_id: &str,
) -> Result<MutationBatch> {
    let mut batch = MutationBatch::new();
    batch.insert(agent_participates_in(dict, agent_did, job_id)?);
    Ok(batch)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::StringDictionary as Dict;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "neunode_kg_mutations_{:?}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    // ── MutationBatch unit tests ──

    #[test]
    fn batch_new_empty() {
        let batch = MutationBatch::new();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn batch_insert_increments_len() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let mut batch = MutationBatch::new();
        let q = type_quad(&dict, "s", CLASS_AGENT).unwrap();
        batch.insert(q);
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());
    }

    #[test]
    fn batch_delete_increments_len() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let mut batch = MutationBatch::new();
        let q = type_quad(&dict, "s", CLASS_AGENT).unwrap();
        batch.delete(q);
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());
    }

    #[test]
    fn batch_clear() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let mut batch = MutationBatch::new();
        batch.insert(type_quad(&dict, "s1", CLASS_AGENT).unwrap());
        batch.delete(type_quad(&dict, "s2", CLASS_MODEL).unwrap());
        assert_eq!(batch.len(), 2);
        batch.clear();
        assert!(batch.is_empty());
    }

    #[test]
    fn batch_apply_inserts() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let q = type_quad(&dict, "did:neunode:test", CLASS_AGENT).unwrap();
        let mut batch = MutationBatch::new();
        batch.insert(q.clone());
        batch.apply(&db).unwrap();
        // Verify quad exists in all 6 indexes
        assert!(q.exists_in(&db, CF_KG_SPOG).unwrap());
    }

    #[test]
    fn batch_apply_deletes() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let q = type_quad(&dict, "did:neunode:temp", CLASS_MODEL).unwrap();
        // Insert first
        q.insert_indexes(&db).unwrap();
        assert!(q.exists_in(&db, CF_KG_SPOG).unwrap());
        // Now delete via batch
        let mut batch = MutationBatch::new();
        batch.delete(q.clone());
        batch.apply(&db).unwrap();
        assert!(!q.exists_in(&db, CF_KG_SPOG).unwrap());
    }

    #[test]
    fn batch_mixed_insert_delete() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);

        let q_del = type_quad(&dict, "old_entity", CLASS_AGENT).unwrap();
        q_del.insert_indexes(&db).unwrap();

        let q_ins = type_quad(&dict, "new_entity", CLASS_MODEL).unwrap();

        let mut batch = MutationBatch::new();
        batch.insert(q_ins.clone());
        batch.delete(q_del.clone());
        assert_eq!(batch.len(), 2);

        batch.apply(&db).unwrap();
        assert!(q_ins.exists_in(&db, CF_KG_SPOG).unwrap());
        assert!(!q_del.exists_in(&db, CF_KG_SPOG).unwrap());
    }

    // ── register_agent tests ──

    #[test]
    fn register_agent_type() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let batch = register_agent(&dict, "did:neunode:alice", &[]).unwrap();
        // 1 type quad, 0 capabilities
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn register_agent_capabilities() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let batch = register_agent(&dict, "did:neunode:bob", &["NLP", "Vision", "RL"]).unwrap();
        // 1 type + 3 capabilities
        assert_eq!(batch.len(), 4);
    }

    #[test]
    fn register_agent_apply() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let batch = register_agent(&dict, "did:neunode:carol", &["Training"]).unwrap();
        batch.apply(&db).unwrap();

        // Verify type quad exists via prefix scan on SPOG (subject = agent did)
        let agent_hash = Dict::hash("did:neunode:carol");
        let prefix = Quad::prefix_for(CF_KG_SPOG, &agent_hash);
        let results = db.prefix_scan(CF_KG_SPOG, &prefix).unwrap();
        // type quad + capability quad = 2 entries
        assert_eq!(results.len(), 2);
    }

    // ── register_model tests ──

    #[test]
    fn register_model_type_and_owner() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let batch = register_model(&dict, "did:neunode:dev", "ipfs://QmModel", None).unwrap();
        // type + ownsModel = 2
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn register_model_with_parent() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let batch =
            register_model(&dict, "did:neunode:dev", "ipfs://QmChild", Some("ipfs://QmParent"))
                .unwrap();
        // type + ownsModel + dependsOn = 3
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn register_model_no_parent() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let batch = register_model(&dict, "did:neunode:dev", "ipfs://QmStandalone", None).unwrap();
        // Only type + ownsModel, no dependsOn
        assert_eq!(batch.len(), 2);
    }

    // ── register_bounty tests ──

    #[test]
    fn register_bounty_capabilities() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let batch = register_bounty(&dict, "bounty:42", &["RLHF", "DataLabeling"]).unwrap();
        // type + 2 capabilities = 3
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn register_bounty_no_capabilities() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let batch = register_bounty(&dict, "bounty:99", &[]).unwrap();
        assert_eq!(batch.len(), 1);
    }

    // ── join_training_job tests ──

    #[test]
    fn join_training_job_creates_quad() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let batch = join_training_job(&dict, "did:neunode:worker", "job:101").unwrap();
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn join_training_job_apply() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);
        let batch = join_training_job(&dict, "did:neunode:worker", "job:202").unwrap();
        batch.apply(&db).unwrap();

        // Verify via prefix scan on SPOG for the agent
        let agent_hash = Dict::hash("did:neunode:worker");
        let prefix = Quad::prefix_for(CF_KG_SPOG, &agent_hash);
        let results = db.prefix_scan(CF_KG_SPOG, &prefix).unwrap();
        assert_eq!(results.len(), 1);
    }

    // ── Full flow integration test ──

    #[test]
    fn full_workflow() {
        let db = temp_db();
        let dict = StringDictionary::new(&db);

        // Register agent with 2 capabilities
        let agent_batch =
            register_agent(&dict, "did:neunode:agent1", &["NLP", "Training"]).unwrap();
        assert_eq!(agent_batch.len(), 3);
        agent_batch.apply(&db).unwrap();

        // Register a model with lineage
        let model_batch = register_model(
            &dict,
            "did:neunode:agent1",
            "ipfs://QmFineTuned",
            Some("ipfs://QmBaseModel"),
        )
        .unwrap();
        assert_eq!(model_batch.len(), 3);
        model_batch.apply(&db).unwrap();

        // Register a bounty
        let bounty_batch = register_bounty(&dict, "bounty:sentiment", &["NLP", "RLHF"]).unwrap();
        assert_eq!(bounty_batch.len(), 3);
        bounty_batch.apply(&db).unwrap();

        // Join a training job
        let job_batch =
            join_training_job(&dict, "did:neunode:agent1", "job:distributed-001").unwrap();
        assert_eq!(job_batch.len(), 1);
        job_batch.apply(&db).unwrap();

        // Verify total triples for agent1 via prefix scan
        let agent_hash = Dict::hash("did:neunode:agent1");
        let prefix = Quad::prefix_for(CF_KG_SPOG, &agent_hash);
        let results = db.prefix_scan(CF_KG_SPOG, &prefix).unwrap();
        // type + 2 capabilities + ownsModel + participatesIn = 5
        assert_eq!(results.len(), 5);

        // Verify model has 2 triples: type + dependsOn (ownsModel is from agent, not model)
        let model_hash = Dict::hash("ipfs://QmFineTuned");
        let model_prefix = Quad::prefix_for(CF_KG_SPOG, &model_hash);
        let model_results = db.prefix_scan(CF_KG_SPOG, &model_prefix).unwrap();
        // type(Model) + dependsOn(parent) = 2
        assert_eq!(model_results.len(), 2);
    }
}
