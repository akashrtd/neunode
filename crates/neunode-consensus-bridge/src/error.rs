use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("engine API error: {0}")]
    EngineApi(#[from] neunode_engine_api_client::EngineApiError),

    #[error("chain not started: no genesis block found")]
    NotStarted,

    #[error("block production failed: {0}")]
    BlockProduction(String),

    #[error("invalid forkchoice state: head={head} safe={safe} finalized={finalized}")]
    InvalidForkchoice { head: String, safe: String, finalized: String },

    #[error("payload building timed out after {timeout_ms}ms")]
    PayloadTimeout { timeout_ms: u64 },

    #[error("consensus driver stopped")]
    Stopped,
}

pub type Result<T> = std::result::Result<T, BridgeError>;
