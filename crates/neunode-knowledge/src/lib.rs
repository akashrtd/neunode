pub mod authorization;
pub mod cache;
pub mod dictionary;
pub mod error;
pub mod mutations;
pub mod ontology;
pub mod query;
pub mod triple;

pub use authorization::MutationAuthorization;
pub use cache::KgCache;
pub use dictionary::StringDictionary;
pub use error::{KnowledgeError, Result};
pub use mutations::{
    apply_authorized, join_training_job, register_agent, register_bounty, register_model,
    MutationBatch,
};
pub use ontology::{
    agent_has_capability, agent_owns_model, agent_participates_in, all_classes, all_predicates,
    bounty_requires_capability, model_depends_on, nn, relation_quad, type_quad, CLASS_AGENT,
    CLASS_BOUNTY, CLASS_CAPABILITY, CLASS_KNOWLEDGE, CLASS_MODEL, CLASS_TRAINING_JOB,
    DEFAULT_GRAPH, PRED_CLAIMED_BOUNTY, PRED_CONTRIBUTED_TO, PRED_CREATED_BOUNTY, PRED_DEPENDS_ON,
    PRED_HAS_CAPABILITY, PRED_KNOWS, PRED_OWNS_MODEL, PRED_PARTICIPATES_IN,
    PRED_REQUIRES_CAPABILITY, PRED_TRAINED_MODEL, PRED_TYPE,
};
pub use query::{QueryEngine, QueryPattern, QueryResult};
pub use triple::{Quad, TripleCodec};
