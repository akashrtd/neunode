use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Decentralized Identifier. Stored as string, validated on construction.
/// Format: "did:neunode:0xABC123..." or "did:key:z6Mk..."
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct Did(pub String);

impl Did {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_neunode(&self) -> bool {
        self.0.starts_with("did:neunode:")
    }

    pub fn is_key(&self) -> bool {
        self.0.starts_with("did:key:")
    }
}

impl std::fmt::Display for Did {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Content-addressed identifier (CID v1).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct CID(pub String);

impl CID {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Construct a CID from a BLAKE3 hex hash string.
    /// Format: "blake3:{hex}" — a Neunode convention for Phase A.
    pub fn from_blake3_hex(hex_hash: &str) -> Self {
        CID(format!("blake3:{hex_hash}"))
    }

    /// Check if this CID uses the blake3 convention.
    pub fn is_blake3(&self) -> bool {
        self.0.starts_with("blake3:")
    }
}

impl std::fmt::Display for CID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// libp2p PeerId (multihash of Ed25519 public key, e.g. "12D3Koo...").
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct PeerId(pub String);

impl PeerId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique bounty identifier.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct BountyId(pub String);

impl std::fmt::Display for BountyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique event identifier (= CID of the event body).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct EventId(pub String);

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Token amount (u64, in smallest unit).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, TS)]
#[ts(export)]
pub struct TokenAmount(pub u64);

impl TokenAmount {
    pub const ZERO: TokenAmount = TokenAmount(0);

    pub fn checked_add(self, other: TokenAmount) -> Option<TokenAmount> {
        self.0.checked_add(other.0).map(TokenAmount)
    }

    pub fn checked_sub(self, other: TokenAmount) -> Option<TokenAmount> {
        self.0.checked_sub(other.0).map(TokenAmount)
    }
}

impl std::fmt::Display for TokenAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Token types in the Neunode economy.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub enum TokenType {
    Compute,
    Train,
    Bandwidth,
    Storage,
}

/// Agent lifecycle states.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub enum AgentLifecycle {
    Created,
    Active,
    Idle,
    Zombie,
    Dead,
}

/// Bounty lifecycle states.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub enum BountyState {
    Open,
    Claimed,
    Submitted,
    UnderReview,
    Revision,
    Accepted,
    Rejected,
    Disputed,
    Paid,
    Expired,
    Cancelled,
}

impl BountyState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            BountyState::Accepted
                | BountyState::Rejected
                | BountyState::Paid
                | BountyState::Expired
                | BountyState::Cancelled
        )
    }
}

/// Activity level for token decay calculation.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub enum ActivityLevel {
    Active,
    Moderate,
    Low,
    Inactive,
    Dead,
}

impl ActivityLevel {
    pub fn decay_rate(&self) -> f64 {
        match self {
            ActivityLevel::Active => 0.0,
            ActivityLevel::Moderate => 2.0,
            ActivityLevel::Low => 5.0,
            ActivityLevel::Inactive => 15.0,
            ActivityLevel::Dead => 50.0,
        }
    }

    pub fn from_days_since_activity(days: u64) -> Self {
        match days {
            0..=1 => ActivityLevel::Active,
            2..=7 => ActivityLevel::Moderate,
            8..=30 => ActivityLevel::Low,
            31..=90 => ActivityLevel::Inactive,
            _ => ActivityLevel::Dead,
        }
    }
}

/// SHA-256 hash, stored as hex string.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub struct Hash256(pub String);

/// Ed25519 signature, stored as hex string with "ed25519:" prefix.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, TS)]
#[ts(export)]
pub struct Signature(pub String);

/// Unix timestamp (seconds since epoch).
pub type Timestamp = u64;

/// Sequence number in an agent's sigchain (monotonically increasing).
pub type Sequence = u64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn did_is_neunode() {
        let did = Did("did:neunode:0xABC123".to_string());
        assert!(did.is_neunode());
        assert!(!did.is_key());
    }

    #[test]
    fn did_is_key() {
        let did = Did("did:key:z6Mkabc".to_string());
        assert!(did.is_key());
        assert!(!did.is_neunode());
    }

    #[test]
    fn did_as_str() {
        let did = Did("did:neunode:0xABC".to_string());
        assert_eq!(did.as_str(), "did:neunode:0xABC");
    }

    #[test]
    fn did_display() {
        let did = Did("did:neunode:0xABC".to_string());
        assert_eq!(format!("{did}"), "did:neunode:0xABC");
    }

    #[test]
    fn did_serde_roundtrip() {
        let did = Did("did:neunode:0xABC".to_string());
        let json = serde_json::to_string(&did).unwrap();
        let back: Did = serde_json::from_str(&json).unwrap();
        assert_eq!(did, back);
    }

    #[test]
    fn cid_as_str() {
        let cid = CID("bafkreihdwd".to_string());
        assert_eq!(cid.as_str(), "bafkreihdwd");
    }

    #[test]
    fn cid_display() {
        let cid = CID("bafkreihdwd".to_string());
        assert_eq!(format!("{cid}"), "bafkreihdwd");
    }

    #[test]
    fn cid_serde_roundtrip() {
        let cid = CID("bafkreihdwd".to_string());
        let json = serde_json::to_string(&cid).unwrap();
        let back: CID = serde_json::from_str(&json).unwrap();
        assert_eq!(cid, back);
    }

    #[test]
    fn cid_from_blake3_hex() {
        let hex = "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";
        let cid = CID::from_blake3_hex(hex);
        assert_eq!(
            cid.as_str(),
            "blake3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn cid_is_blake3() {
        let cid = CID::from_blake3_hex("abc123");
        assert!(cid.is_blake3());
        let legacy = CID("bafkreihdwd".to_string());
        assert!(!legacy.is_blake3());
    }

    #[test]
    fn cid_blake3_serde_roundtrip() {
        let cid = CID::from_blake3_hex("deadbeef");
        let json = serde_json::to_string(&cid).unwrap();
        let back: CID = serde_json::from_str(&json).unwrap();
        assert_eq!(cid, back);
    }

    #[test]
    fn peer_id_as_str() {
        let pid = PeerId("12D3Koo...".to_string());
        assert_eq!(pid.as_str(), "12D3Koo...");
    }

    #[test]
    fn peer_id_display() {
        let pid = PeerId("12D3KooABC".to_string());
        assert_eq!(format!("{pid}"), "12D3KooABC");
    }

    #[test]
    fn bounty_id_display() {
        let bid = BountyId("bnty_8f3a2c".to_string());
        assert_eq!(format!("{bid}"), "bnty_8f3a2c");
    }

    #[test]
    fn event_id_display() {
        let eid = EventId("bafkrei_evt".to_string());
        assert_eq!(format!("{eid}"), "bafkrei_evt");
    }

    #[test]
    fn token_amount_zero() {
        assert_eq!(TokenAmount::ZERO, TokenAmount(0));
    }

    #[test]
    fn token_amount_checked_add() {
        let a = TokenAmount(100);
        let b = TokenAmount(200);
        assert_eq!(a.checked_add(b), Some(TokenAmount(300)));
    }

    #[test]
    fn token_amount_checked_add_overflow() {
        let a = TokenAmount(u64::MAX);
        let b = TokenAmount(1);
        assert_eq!(a.checked_add(b), None);
    }

    #[test]
    fn token_amount_checked_sub() {
        let a = TokenAmount(300);
        let b = TokenAmount(100);
        assert_eq!(a.checked_sub(b), Some(TokenAmount(200)));
    }

    #[test]
    fn token_amount_checked_sub_underflow() {
        let a = TokenAmount(0);
        let b = TokenAmount(1);
        assert_eq!(a.checked_sub(b), None);
    }

    #[test]
    fn token_amount_ordering() {
        assert!(TokenAmount(10) < TokenAmount(20));
        assert!(TokenAmount(20) > TokenAmount(10));
        assert!(TokenAmount(5) == TokenAmount(5));
    }

    #[test]
    fn token_amount_display() {
        let amt = TokenAmount(500);
        assert_eq!(format!("{amt}"), "500");
    }

    #[test]
    fn token_amount_serde_roundtrip() {
        let amt = TokenAmount(42);
        let json = serde_json::to_string(&amt).unwrap();
        let back: TokenAmount = serde_json::from_str(&json).unwrap();
        assert_eq!(amt, back);
    }

    #[test]
    fn token_type_serde_roundtrip() {
        for tt in [TokenType::Compute, TokenType::Train, TokenType::Bandwidth, TokenType::Storage] {
            let json = serde_json::to_string(&tt).unwrap();
            let back: TokenType = serde_json::from_str(&json).unwrap();
            assert_eq!(tt, back);
        }
    }

    #[test]
    fn agent_lifecycle_serde_roundtrip() {
        for lc in [
            AgentLifecycle::Created,
            AgentLifecycle::Active,
            AgentLifecycle::Idle,
            AgentLifecycle::Zombie,
            AgentLifecycle::Dead,
        ] {
            let json = serde_json::to_string(&lc).unwrap();
            let back: AgentLifecycle = serde_json::from_str(&json).unwrap();
            assert_eq!(lc, back);
        }
    }

    #[test]
    fn bounty_state_terminal() {
        assert!(BountyState::Accepted.is_terminal());
        assert!(BountyState::Rejected.is_terminal());
        assert!(BountyState::Paid.is_terminal());
        assert!(BountyState::Expired.is_terminal());
        assert!(BountyState::Cancelled.is_terminal());
    }

    #[test]
    fn bounty_state_non_terminal() {
        assert!(!BountyState::Open.is_terminal());
        assert!(!BountyState::Claimed.is_terminal());
        assert!(!BountyState::Submitted.is_terminal());
        assert!(!BountyState::UnderReview.is_terminal());
        assert!(!BountyState::Revision.is_terminal());
        assert!(!BountyState::Disputed.is_terminal());
    }

    #[test]
    fn bounty_state_serde_roundtrip() {
        for st in [
            BountyState::Open,
            BountyState::Claimed,
            BountyState::Submitted,
            BountyState::UnderReview,
            BountyState::Revision,
            BountyState::Accepted,
            BountyState::Rejected,
            BountyState::Disputed,
            BountyState::Paid,
            BountyState::Expired,
            BountyState::Cancelled,
        ] {
            let json = serde_json::to_string(&st).unwrap();
            let back: BountyState = serde_json::from_str(&json).unwrap();
            assert_eq!(st, back);
        }
    }

    #[test]
    fn activity_level_from_days_boundary() {
        assert_eq!(ActivityLevel::from_days_since_activity(0), ActivityLevel::Active);
        assert_eq!(ActivityLevel::from_days_since_activity(1), ActivityLevel::Active);
        assert_eq!(ActivityLevel::from_days_since_activity(2), ActivityLevel::Moderate);
        assert_eq!(ActivityLevel::from_days_since_activity(7), ActivityLevel::Moderate);
        assert_eq!(ActivityLevel::from_days_since_activity(8), ActivityLevel::Low);
        assert_eq!(ActivityLevel::from_days_since_activity(30), ActivityLevel::Low);
        assert_eq!(ActivityLevel::from_days_since_activity(31), ActivityLevel::Inactive);
        assert_eq!(ActivityLevel::from_days_since_activity(90), ActivityLevel::Inactive);
        assert_eq!(ActivityLevel::from_days_since_activity(91), ActivityLevel::Dead);
        assert_eq!(ActivityLevel::from_days_since_activity(365), ActivityLevel::Dead);
    }

    #[test]
    fn activity_level_decay_rates() {
        assert_eq!(ActivityLevel::Active.decay_rate(), 0.0);
        assert_eq!(ActivityLevel::Moderate.decay_rate(), 2.0);
        assert_eq!(ActivityLevel::Low.decay_rate(), 5.0);
        assert_eq!(ActivityLevel::Inactive.decay_rate(), 15.0);
        assert_eq!(ActivityLevel::Dead.decay_rate(), 50.0);
    }

    #[test]
    fn activity_level_serde_roundtrip() {
        for al in [
            ActivityLevel::Active,
            ActivityLevel::Moderate,
            ActivityLevel::Low,
            ActivityLevel::Inactive,
            ActivityLevel::Dead,
        ] {
            let json = serde_json::to_string(&al).unwrap();
            let back: ActivityLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(al, back);
        }
    }

    #[test]
    fn hash256_serde_roundtrip() {
        let h = Hash256("abc123def456".to_string());
        let json = serde_json::to_string(&h).unwrap();
        let back: Hash256 = serde_json::from_str(&json).unwrap();
        assert_eq!(h, back);
    }

    #[test]
    fn signature_serde_roundtrip() {
        let s = Signature("ed25519:abc123".to_string());
        let json = serde_json::to_string(&s).unwrap();
        let back: Signature = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn timestamp_and_sequence_are_u64() {
        let ts: Timestamp = 1700000000;
        let seq: Sequence = 42;
        assert_eq!(ts, 1700000000u64);
        assert_eq!(seq, 42u64);
    }
}
