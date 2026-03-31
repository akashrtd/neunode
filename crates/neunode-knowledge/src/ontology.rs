use crate::dictionary::StringDictionary;
use crate::error::Result;
use crate::triple::Quad;

// ---------------------------------------------------------------------------
// Namespace
// ---------------------------------------------------------------------------

const NS: &str = "https://neunode.io/ontology/";

/// Shorthand for building a full URI: nn("Agent") → "https://neunode.io/ontology/Agent"
pub fn nn(suffix: &str) -> String {
    format!("{NS}{suffix}")
}

// ---------------------------------------------------------------------------
// Entity types (RDFS classes)
// ---------------------------------------------------------------------------

pub const CLASS_AGENT: &str = "Agent";
pub const CLASS_MODEL: &str = "Model";
pub const CLASS_BOUNTY: &str = "Bounty";
pub const CLASS_TRAINING_JOB: &str = "TrainingJob";
pub const CLASS_CAPABILITY: &str = "Capability";
pub const CLASS_KNOWLEDGE: &str = "Knowledge";

/// All entity class names.
pub fn all_classes() -> Vec<&'static str> {
    vec![
        CLASS_AGENT,
        CLASS_MODEL,
        CLASS_BOUNTY,
        CLASS_TRAINING_JOB,
        CLASS_CAPABILITY,
        CLASS_KNOWLEDGE,
    ]
}

// ---------------------------------------------------------------------------
// Predicates (RDF properties)
// ---------------------------------------------------------------------------

pub const PRED_HAS_CAPABILITY: &str = "hasCapability";
pub const PRED_OWNS_MODEL: &str = "ownsModel";
pub const PRED_CREATED_BOUNTY: &str = "createdBounty";
pub const PRED_CLAIMED_BOUNTY: &str = "claimedBounty";
pub const PRED_PARTICIPATES_IN: &str = "participatesIn";
pub const PRED_TRAINED_MODEL: &str = "trainedModel";
pub const PRED_REQUIRES_CAPABILITY: &str = "requiresCapability";
pub const PRED_DEPENDS_ON: &str = "dependsOn";
pub const PRED_KNOWS: &str = "knows";
pub const PRED_CONTRIBUTED_TO: &str = "contributedTo";
pub const PRED_TYPE: &str = "type";

/// All predicate names.
pub fn all_predicates() -> Vec<&'static str> {
    vec![
        PRED_HAS_CAPABILITY,
        PRED_OWNS_MODEL,
        PRED_CREATED_BOUNTY,
        PRED_CLAIMED_BOUNTY,
        PRED_PARTICIPATES_IN,
        PRED_TRAINED_MODEL,
        PRED_REQUIRES_CAPABILITY,
        PRED_DEPENDS_ON,
        PRED_KNOWS,
        PRED_CONTRIBUTED_TO,
        PRED_TYPE,
    ]
}

// ---------------------------------------------------------------------------
// Default graph
// ---------------------------------------------------------------------------

pub const DEFAULT_GRAPH: &str = "neunode";

// ---------------------------------------------------------------------------
// Quad construction helpers
// ---------------------------------------------------------------------------

/// Build a type assertion quad: (entity, rdf:type, class, graph).
pub fn type_quad(dict: &StringDictionary, entity_uri: &str, class: &str) -> Result<Quad> {
    Quad::from_strings(dict, entity_uri, &nn(PRED_TYPE), &nn(class), &nn(DEFAULT_GRAPH))
}

/// Build a relationship quad: (subject, predicate, object, graph).
pub fn relation_quad(
    dict: &StringDictionary,
    subject_uri: &str,
    predicate: &str,
    object_uri: &str,
) -> Result<Quad> {
    Quad::from_strings(dict, subject_uri, &nn(predicate), object_uri, &nn(DEFAULT_GRAPH))
}

/// Build an agent capability quad.
pub fn agent_has_capability(
    dict: &StringDictionary,
    agent_did: &str,
    capability: &str,
) -> Result<Quad> {
    relation_quad(dict, agent_did, PRED_HAS_CAPABILITY, &nn(capability))
}

/// Build an agent owns model quad.
pub fn agent_owns_model(dict: &StringDictionary, agent_did: &str, model_cid: &str) -> Result<Quad> {
    relation_quad(dict, agent_did, PRED_OWNS_MODEL, model_cid)
}

/// Build a model lineage quad.
pub fn model_depends_on(
    dict: &StringDictionary,
    child_cid: &str,
    parent_cid: &str,
) -> Result<Quad> {
    relation_quad(dict, child_cid, PRED_DEPENDS_ON, parent_cid)
}

/// Build a bounty requires capability quad.
pub fn bounty_requires_capability(
    dict: &StringDictionary,
    bounty_id: &str,
    capability: &str,
) -> Result<Quad> {
    relation_quad(dict, bounty_id, PRED_REQUIRES_CAPABILITY, &nn(capability))
}

/// Build an agent participates in training quad.
pub fn agent_participates_in(
    dict: &StringDictionary,
    agent_did: &str,
    job_id: &str,
) -> Result<Quad> {
    relation_quad(dict, agent_did, PRED_PARTICIPATES_IN, job_id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::StringDictionary;
    use neunode_storage::db::NeunodeDb;
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db() -> NeunodeDb {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "neunode_kg_ontology_{:?}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&dir);
        NeunodeDb::open(&dir).unwrap()
    }

    fn temp_dict<'a>(db: &'a NeunodeDb) -> StringDictionary<'a> {
        StringDictionary::new(db)
    }

    // ── Namespace tests ──

    #[test]
    fn nn_constructs_uri() {
        assert_eq!(nn("Agent"), "https://neunode.io/ontology/Agent");
    }

    #[test]
    fn nn_empty_suffix() {
        assert_eq!(nn(""), NS);
    }

    #[test]
    fn nn_uri_deterministic() {
        assert_eq!(nn("Model"), nn("Model"));
    }

    #[test]
    fn nn_different_suffixes() {
        assert_ne!(nn("Agent"), nn("Model"));
    }

    // ── Class tests ──

    #[test]
    fn all_classes_count() {
        assert_eq!(all_classes().len(), 6);
    }

    #[test]
    fn all_classes_unique() {
        let classes = all_classes();
        let set: HashSet<_> = classes.iter().copied().collect();
        assert_eq!(set.len(), classes.len(), "class names must be unique");
    }

    #[test]
    fn class_names_no_spaces() {
        for c in all_classes() {
            assert!(!c.contains(' '), "class name '{c}' must not contain spaces");
            assert!(!c.is_empty(), "class name must not be empty");
        }
    }

    #[test]
    fn class_constants_match_all_classes() {
        let classes = all_classes();
        assert!(classes.contains(&CLASS_AGENT));
        assert!(classes.contains(&CLASS_MODEL));
        assert!(classes.contains(&CLASS_BOUNTY));
        assert!(classes.contains(&CLASS_TRAINING_JOB));
        assert!(classes.contains(&CLASS_CAPABILITY));
        assert!(classes.contains(&CLASS_KNOWLEDGE));
    }

    #[test]
    fn class_uris_valid() {
        for c in all_classes() {
            let uri = nn(c);
            assert!(uri.starts_with(NS), "class URI must start with namespace");
            assert!(uri.ends_with(c), "class URI must end with class name");
        }
    }

    // ── Predicate tests ──

    #[test]
    fn all_predicates_count() {
        assert_eq!(all_predicates().len(), 11);
    }

    #[test]
    fn all_predicates_unique() {
        let preds = all_predicates();
        let set: HashSet<_> = preds.iter().copied().collect();
        assert_eq!(set.len(), preds.len(), "predicate names must be unique");
    }

    #[test]
    fn predicate_names_no_spaces() {
        for p in all_predicates() {
            assert!(!p.contains(' '), "predicate name '{p}' must not contain spaces");
            assert!(!p.is_empty(), "predicate name must not be empty");
        }
    }

    #[test]
    fn predicate_constants_match_all_predicates() {
        let preds = all_predicates();
        assert!(preds.contains(&PRED_HAS_CAPABILITY));
        assert!(preds.contains(&PRED_OWNS_MODEL));
        assert!(preds.contains(&PRED_CREATED_BOUNTY));
        assert!(preds.contains(&PRED_CLAIMED_BOUNTY));
        assert!(preds.contains(&PRED_PARTICIPATES_IN));
        assert!(preds.contains(&PRED_TRAINED_MODEL));
        assert!(preds.contains(&PRED_REQUIRES_CAPABILITY));
        assert!(preds.contains(&PRED_DEPENDS_ON));
        assert!(preds.contains(&PRED_KNOWS));
        assert!(preds.contains(&PRED_CONTRIBUTED_TO));
        assert!(preds.contains(&PRED_TYPE));
    }

    #[test]
    fn predicate_uris_valid() {
        for p in all_predicates() {
            let uri = nn(p);
            assert!(uri.starts_with(NS), "predicate URI must start with namespace");
        }
    }

    // ── Default graph tests ──

    #[test]
    fn default_graph_value() {
        assert_eq!(DEFAULT_GRAPH, "neunode");
    }

    #[test]
    fn default_graph_uri() {
        let uri = nn(DEFAULT_GRAPH);
        assert_eq!(uri, "https://neunode.io/ontology/neunode");
    }

    // ── Quad construction tests ──

    #[test]
    fn type_quad_creation() {
        let db = temp_db();
        let dict = temp_dict(&db);
        let q = type_quad(&dict, "did:neunode:abc123", CLASS_AGENT).unwrap();

        assert_eq!(q.subject, StringDictionary::hash("did:neunode:abc123"));
        assert_eq!(q.predicate, StringDictionary::hash(&nn(PRED_TYPE)));
        assert_eq!(q.object, StringDictionary::hash(&nn(CLASS_AGENT)));
        assert_eq!(q.graph, StringDictionary::hash(&nn(DEFAULT_GRAPH)));
    }

    #[test]
    fn type_quad_uses_graph() {
        let db = temp_db();
        let dict = temp_dict(&db);
        let q = type_quad(&dict, "s", CLASS_MODEL).unwrap();
        assert_eq!(q.graph, StringDictionary::hash(&nn(DEFAULT_GRAPH)));
    }

    #[test]
    fn relation_quad_creation() {
        let db = temp_db();
        let dict = temp_dict(&db);
        let q =
            relation_quad(&dict, "did:neunode:alice", PRED_OWNS_MODEL, "ipfs://QmHash").unwrap();

        assert_eq!(q.subject, StringDictionary::hash("did:neunode:alice"));
        assert_eq!(q.predicate, StringDictionary::hash(&nn(PRED_OWNS_MODEL)));
        assert_eq!(q.object, StringDictionary::hash("ipfs://QmHash"));
        assert_eq!(q.graph, StringDictionary::hash(&nn(DEFAULT_GRAPH)));
    }

    #[test]
    fn agent_has_capability_quad() {
        let db = temp_db();
        let dict = temp_dict(&db);
        let q = agent_has_capability(&dict, "did:neunode:agent1", "NLP").unwrap();

        assert_eq!(q.subject, StringDictionary::hash("did:neunode:agent1"));
        assert_eq!(q.predicate, StringDictionary::hash(&nn(PRED_HAS_CAPABILITY)));
        assert_eq!(q.object, StringDictionary::hash(&nn("NLP")));
    }

    #[test]
    fn agent_owns_model_quad() {
        let db = temp_db();
        let dict = temp_dict(&db);
        let q = agent_owns_model(&dict, "did:neunode:agent1", "ipfs://QmModel").unwrap();

        assert_eq!(q.subject, StringDictionary::hash("did:neunode:agent1"));
        assert_eq!(q.predicate, StringDictionary::hash(&nn(PRED_OWNS_MODEL)));
        assert_eq!(q.object, StringDictionary::hash("ipfs://QmModel"));
    }

    #[test]
    fn model_depends_on_quad() {
        let db = temp_db();
        let dict = temp_dict(&db);
        let q = model_depends_on(&dict, "ipfs://QmChild", "ipfs://QmParent").unwrap();

        assert_eq!(q.subject, StringDictionary::hash("ipfs://QmChild"));
        assert_eq!(q.predicate, StringDictionary::hash(&nn(PRED_DEPENDS_ON)));
        assert_eq!(q.object, StringDictionary::hash("ipfs://QmParent"));
    }

    #[test]
    fn bounty_requires_capability_quad() {
        let db = temp_db();
        let dict = temp_dict(&db);
        let q = bounty_requires_capability(&dict, "bounty:123", "RLHF").unwrap();

        assert_eq!(q.subject, StringDictionary::hash("bounty:123"));
        assert_eq!(q.predicate, StringDictionary::hash(&nn(PRED_REQUIRES_CAPABILITY)));
        assert_eq!(q.object, StringDictionary::hash(&nn("RLHF")));
    }

    #[test]
    fn agent_participates_in_quad() {
        let db = temp_db();
        let dict = temp_dict(&db);
        let q = agent_participates_in(&dict, "did:neunode:worker1", "job:789").unwrap();

        assert_eq!(q.subject, StringDictionary::hash("did:neunode:worker1"));
        assert_eq!(q.predicate, StringDictionary::hash(&nn(PRED_PARTICIPATES_IN)));
        assert_eq!(q.object, StringDictionary::hash("job:789"));
    }

    #[test]
    fn type_quad_interns_strings() {
        let db = temp_db();
        let dict = temp_dict(&db);
        type_quad(&dict, "did:neunode:x", CLASS_CAPABILITY).unwrap();

        // All 4 strings should be in the dictionary
        assert!(dict.contains(&StringDictionary::hash("did:neunode:x")).unwrap());
        assert!(dict.contains(&StringDictionary::hash(&nn(PRED_TYPE))).unwrap());
        assert!(dict.contains(&StringDictionary::hash(&nn(CLASS_CAPABILITY))).unwrap());
        assert!(dict.contains(&StringDictionary::hash(&nn(DEFAULT_GRAPH))).unwrap());
    }

    #[test]
    fn relation_quad_interns_strings() {
        let db = temp_db();
        let dict = temp_dict(&db);
        relation_quad(&dict, "s", PRED_KNOWS, "o").unwrap();

        assert!(dict.contains(&StringDictionary::hash("s")).unwrap());
        assert!(dict.contains(&StringDictionary::hash(&nn(PRED_KNOWS))).unwrap());
        assert!(dict.contains(&StringDictionary::hash("o")).unwrap());
        assert!(dict.contains(&StringDictionary::hash(&nn(DEFAULT_GRAPH))).unwrap());
    }

    #[test]
    fn multiple_quads_share_graph_hash() {
        let db = temp_db();
        let dict = temp_dict(&db);
        let q1 = type_quad(&dict, "s1", CLASS_AGENT).unwrap();
        let q2 = agent_owns_model(&dict, "s2", "model1").unwrap();
        // Both should use the same graph hash
        assert_eq!(q1.graph, q2.graph);
    }
}
