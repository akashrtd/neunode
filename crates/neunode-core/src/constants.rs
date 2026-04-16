pub mod p2p {
    pub const MESH_DEGREE: usize = 6;
    pub const MESH_DEGREE_LOW: usize = 4;
    pub const MESH_DEGREE_HIGH: usize = 12;
    pub const GOSSIP_FACTOR: f64 = 0.25;
    pub const DEFAULT_PORT: u16 = 41000;
    pub const DHT_PROTOCOL: &str = "/neunode/kad/1.0.0";
    pub const IDENTIFY_PROTOCOL: &str = "/neunode/identify/1.0.0";
    pub const PEER_SCORE_P5_WEIGHT: f64 = 10.0;
    pub const PEER_SCORE_P6_WEIGHT: f64 = 5.0;
    pub const BOOTSTRAP_TIMEOUT_SECS: u64 = 30;
    pub const MAX_PEER_CONNECTIONS: usize = 100;
}

pub mod feed {
    pub const GENESIS_PREV_HASH: &str = "0";
    pub const MAX_CONTENT_SIZE: usize = 1024 * 1024;
    pub const MAX_TAGS: usize = 100;
    pub const MAX_REFS: usize = 50;
    pub const MAX_TAG_KEY_LEN: usize = 100;
    pub const MAX_TAG_VALUE_LEN: usize = 200;
}

pub mod token {
    pub const DECAY_RATE_ACTIVE: f64 = 0.0;
    pub const DECAY_RATE_MODERATE: f64 = 2.0;
    pub const DECAY_RATE_LOW: f64 = 5.0;
    pub const DECAY_RATE_INACTIVE: f64 = 15.0;
    pub const DECAY_RATE_DEAD: f64 = 50.0;

    pub const ACTIVE_THRESHOLD_DAYS: u64 = 1;
    pub const MODERATE_THRESHOLD_DAYS: u64 = 7;
    pub const LOW_THRESHOLD_DAYS: u64 = 30;
    pub const INACTIVE_THRESHOLD_DAYS: u64 = 90;

    pub const DECAY_TREASURY_PCT: f64 = 40.0;
    pub const DECAY_STAKING_REWARDS_PCT: f64 = 30.0;
    pub const DECAY_BURN_PCT: f64 = 20.0;
    pub const DECAY_DEV_FUND_PCT: f64 = 10.0;

    pub const UNBONDING_PERIOD_SECS: u64 = 7 * 24 * 3600;
    pub const MIN_STAKE: u64 = 100;
}

pub mod bounty {
    pub const DEFAULT_CLAIM_DEADLINE_SECS: u64 = 7 * 24 * 3600;
    pub const DEFAULT_WORK_DEADLINE_SECS: u64 = 14 * 24 * 3600;
    pub const DEFAULT_REVIEW_DEADLINE_SECS: u64 = 3 * 24 * 3600;
    pub const DEFAULT_DISPUTE_DEADLINE_SECS: u64 = 5 * 24 * 3600;
    pub const DEFAULT_REVISION_DEADLINE_SECS: u64 = 3 * 24 * 3600;
    pub const GRACE_PERIOD_SECS: u64 = 3600;
    pub const PROVIDER_BOND_PCT: f64 = 15.0;
    pub const PROTOCOL_FEE_PCT: f64 = 3.0;
    pub const REVIEWER_FEE_PCT: f64 = 4.0;
    pub const REVIEWER_COUNT: usize = 3;
    pub const REVIEWER_WEIGHT_CAPABILITY: f64 = 40.0;
    pub const REVIEWER_WEIGHT_REPUTATION: f64 = 30.0;
    pub const REVIEWER_WEIGHT_STAKE: f64 = 20.0;
    pub const REVIEWER_WEIGHT_AVAILABILITY: f64 = 10.0;
    pub const REVIEWER_MIN_STAKE: f64 = 1000.0;
}

pub mod reputation {
    pub const WEIGHT_STAKE: f64 = 30.0;
    pub const WEIGHT_ATTEST: f64 = 25.0;
    pub const WEIGHT_ACTIVITY: f64 = 20.0;
    pub const WEIGHT_VERIFY: f64 = 15.0;
    pub const WEIGHT_TENURE: f64 = 10.0;
    pub const MAX_SCORE: f64 = 100.0;
    pub const MAX_ATTESTATION_DEPTH: usize = 5;
}

pub mod storage {
    pub const DID_HASH_LEN: usize = 16;
    pub const SEQ_LEN: usize = 8;
    pub const FEED_KEY_LEN: usize = DID_HASH_LEN + SEQ_LEN;

    pub mod cf {
        pub const IDENTITY: &str = "identity";
        pub const CONFIG: &str = "config";
        pub const FEED_EVENTS: &str = "feed_events";
        pub const FEED_INDEX: &str = "feed_index";
        pub const FEED_STATE: &str = "feed_state";
        pub const TOKENS: &str = "tokens";
        pub const REPUTATION: &str = "reputation";
        pub const MODELS: &str = "models";
        pub const TRAINING: &str = "training";
        pub const BOUNTIES: &str = "bounties";
        pub const P2P_STATE: &str = "p2p_state";
        pub const MERKLE_NODES: &str = "merkle_nodes";
        pub const SNAPSHOTS: &str = "snapshots";
        pub const KG_ID2STR: &str = "kg_id2str";
        pub const KG_SPOG: &str = "kg_spog";
        pub const KG_POSG: &str = "kg_posg";
        pub const KG_OSPG: &str = "kg_ospg";
        pub const KG_GSPO: &str = "kg_gspo";
        pub const KG_GPOS: &str = "kg_gpos";
        pub const KG_GOSP: &str = "kg_gosp";
    }

    pub const NUM_COLUMN_FAMILIES: usize = 20;
}
