use serde::{Deserialize, Serialize};

use crate::error::{NeunodeError, Result};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Kind {
    AgentMetadata = 0,
    CapabilityUpdate = 1,
    ReputationChange = 2,
    IdentityRotation = 3,
    Lifecycle = 5,

    BountyPost = 1000,
    BountyClaim = 1001,
    BountySubmit = 1002,
    BountyReview = 1003,
    BountyDispute = 1004,
    BountyResolved = 1005,
    EscrowDeposit = 1100,
    EscrowRelease = 1101,
    EscrowRefund = 1102,

    JobSubmit = 2000,
    Checkpoint = 2001,
    TrainingResult = 2002,
    GradientUpdate = 2010,
    EvalScore = 2020,

    Attest = 3000,
    CounterAttest = 3001,
    DisputeInit = 3002,
    VerificationResult = 3010,

    ModelAnnounce = 4000,
    ServeOffer = 4001,
    ServeResult = 4002,
    BenchmarkClaim = 4010,

    Proposal = 5000,
    Vote = 5001,
    Delegate = 5002,
    ParameterChange = 5010,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KindCategory {
    System,
    Bounty,
    Training,
    Attestation,
    Inference,
    Governance,
    Custom,
    Unknown,
}

impl Kind {
    pub fn from_u16(value: u16) -> Result<Self> {
        match value {
            0 => Ok(Kind::AgentMetadata),
            1 => Ok(Kind::CapabilityUpdate),
            2 => Ok(Kind::ReputationChange),
            3 => Ok(Kind::IdentityRotation),
            5 => Ok(Kind::Lifecycle),
            1000 => Ok(Kind::BountyPost),
            1001 => Ok(Kind::BountyClaim),
            1002 => Ok(Kind::BountySubmit),
            1003 => Ok(Kind::BountyReview),
            1004 => Ok(Kind::BountyDispute),
            1005 => Ok(Kind::BountyResolved),
            1100 => Ok(Kind::EscrowDeposit),
            1101 => Ok(Kind::EscrowRelease),
            1102 => Ok(Kind::EscrowRefund),
            2000 => Ok(Kind::JobSubmit),
            2001 => Ok(Kind::Checkpoint),
            2002 => Ok(Kind::TrainingResult),
            2010 => Ok(Kind::GradientUpdate),
            2020 => Ok(Kind::EvalScore),
            3000 => Ok(Kind::Attest),
            3001 => Ok(Kind::CounterAttest),
            3002 => Ok(Kind::DisputeInit),
            3010 => Ok(Kind::VerificationResult),
            4000 => Ok(Kind::ModelAnnounce),
            4001 => Ok(Kind::ServeOffer),
            4002 => Ok(Kind::ServeResult),
            4010 => Ok(Kind::BenchmarkClaim),
            5000 => Ok(Kind::Proposal),
            5001 => Ok(Kind::Vote),
            5002 => Ok(Kind::Delegate),
            5010 => Ok(Kind::ParameterChange),
            _ => Err(NeunodeError::InvalidKind(value)),
        }
    }

    pub fn as_u16(self) -> u16 {
        self as u16
    }

    pub fn category(self) -> KindCategory {
        let v = self.as_u16();
        match v {
            0..=99 => KindCategory::System,
            1000..=1999 => KindCategory::Bounty,
            2000..=2999 => KindCategory::Training,
            3000..=3999 => KindCategory::Attestation,
            4000..=4999 => KindCategory::Inference,
            5000..=5999 => KindCategory::Governance,
            9000..=9999 => KindCategory::Custom,
            _ => KindCategory::Unknown,
        }
    }

    pub fn schema_nsid(self) -> &'static str {
        match self {
            Kind::BountyPost => "neunode.bounty.post.v1",
            Kind::BountyClaim => "neunode.bounty.claim.v1",
            Kind::BountySubmit => "neunode.bounty.submit.v1",
            Kind::BountyReview => "neunode.bounty.review.v1",
            Kind::BountyDispute => "neunode.bounty.dispute.v1",
            Kind::BountyResolved => "neunode.bounty.resolved.v1",
            Kind::Attest => "neunode.attest.positive.v1",
            Kind::CounterAttest => "neunode.attest.negative.v1",
            Kind::ModelAnnounce => "neunode.inference.announce.v1",
            Kind::Proposal => "neunode.governance.proposal.v1",
            Kind::Vote => "neunode.governance.vote.v1",
            _ => "neunode.unknown.v1",
        }
    }

    pub fn gossipsub_topic(self) -> &'static str {
        match self.category() {
            KindCategory::System => "neunode/system",
            KindCategory::Bounty => "neunode/bounty",
            KindCategory::Training => "neunode/training",
            KindCategory::Attestation => "neunode/attestation",
            KindCategory::Inference => "neunode/inference",
            KindCategory::Governance => "neunode/governance",
            KindCategory::Custom => "neunode/custom",
            KindCategory::Unknown => "neunode/unknown",
        }
    }
}

impl TryFrom<u16> for Kind {
    type Error = NeunodeError;

    fn try_from(value: u16) -> Result<Self> {
        Kind::from_u16(value)
    }
}

impl From<Kind> for u16 {
    fn from(kind: Kind) -> u16 {
        kind.as_u16()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_u16_all_defined_kinds() {
        let cases: Vec<(u16, Kind)> = vec![
            (0, Kind::AgentMetadata),
            (1, Kind::CapabilityUpdate),
            (2, Kind::ReputationChange),
            (3, Kind::IdentityRotation),
            (5, Kind::Lifecycle),
            (1000, Kind::BountyPost),
            (1001, Kind::BountyClaim),
            (1002, Kind::BountySubmit),
            (1003, Kind::BountyReview),
            (1004, Kind::BountyDispute),
            (1005, Kind::BountyResolved),
            (1100, Kind::EscrowDeposit),
            (1101, Kind::EscrowRelease),
            (1102, Kind::EscrowRefund),
            (2000, Kind::JobSubmit),
            (2001, Kind::Checkpoint),
            (2002, Kind::TrainingResult),
            (2010, Kind::GradientUpdate),
            (2020, Kind::EvalScore),
            (3000, Kind::Attest),
            (3001, Kind::CounterAttest),
            (3002, Kind::DisputeInit),
            (3010, Kind::VerificationResult),
            (4000, Kind::ModelAnnounce),
            (4001, Kind::ServeOffer),
            (4002, Kind::ServeResult),
            (4010, Kind::BenchmarkClaim),
            (5000, Kind::Proposal),
            (5001, Kind::Vote),
            (5002, Kind::Delegate),
            (5010, Kind::ParameterChange),
        ];
        for (val, expected) in cases {
            assert_eq!(Kind::from_u16(val).unwrap(), expected);
        }
    }

    #[test]
    fn from_u16_unknown_returns_error() {
        assert!(Kind::from_u16(4).is_err());
        assert!(Kind::from_u16(99).is_err());
        assert!(Kind::from_u16(100).is_err());
        assert!(Kind::from_u16(500).is_err());
        assert!(Kind::from_u16(999).is_err());
        assert!(Kind::from_u16(1500).is_err());
        assert!(Kind::from_u16(9999).is_err());
        assert!(Kind::from_u16(65535).is_err());
    }

    #[test]
    fn from_u16_error_contains_value() {
        let err = Kind::from_u16(4).unwrap_err();
        match err {
            NeunodeError::InvalidKind(v) => assert_eq!(v, 4),
            other => panic!("expected InvalidKind, got {other}"),
        }
    }

    #[test]
    fn as_u16_roundtrip() {
        let kinds = [
            Kind::AgentMetadata,
            Kind::Lifecycle,
            Kind::BountyPost,
            Kind::EscrowRefund,
            Kind::JobSubmit,
            Kind::EvalScore,
            Kind::Attest,
            Kind::VerificationResult,
            Kind::ModelAnnounce,
            Kind::BenchmarkClaim,
            Kind::Proposal,
            Kind::ParameterChange,
        ];
        for k in kinds {
            assert_eq!(Kind::from_u16(k.as_u16()).unwrap(), k);
        }
    }

    #[test]
    fn category_system() {
        assert_eq!(Kind::AgentMetadata.category(), KindCategory::System);
        assert_eq!(Kind::CapabilityUpdate.category(), KindCategory::System);
        assert_eq!(Kind::ReputationChange.category(), KindCategory::System);
        assert_eq!(Kind::IdentityRotation.category(), KindCategory::System);
        assert_eq!(Kind::Lifecycle.category(), KindCategory::System);
    }

    #[test]
    fn category_bounty() {
        assert_eq!(Kind::BountyPost.category(), KindCategory::Bounty);
        assert_eq!(Kind::BountyClaim.category(), KindCategory::Bounty);
        assert_eq!(Kind::BountySubmit.category(), KindCategory::Bounty);
        assert_eq!(Kind::BountyReview.category(), KindCategory::Bounty);
        assert_eq!(Kind::BountyDispute.category(), KindCategory::Bounty);
        assert_eq!(Kind::BountyResolved.category(), KindCategory::Bounty);
        assert_eq!(Kind::EscrowDeposit.category(), KindCategory::Bounty);
        assert_eq!(Kind::EscrowRelease.category(), KindCategory::Bounty);
        assert_eq!(Kind::EscrowRefund.category(), KindCategory::Bounty);
    }

    #[test]
    fn category_training() {
        assert_eq!(Kind::JobSubmit.category(), KindCategory::Training);
        assert_eq!(Kind::Checkpoint.category(), KindCategory::Training);
        assert_eq!(Kind::TrainingResult.category(), KindCategory::Training);
        assert_eq!(Kind::GradientUpdate.category(), KindCategory::Training);
        assert_eq!(Kind::EvalScore.category(), KindCategory::Training);
    }

    #[test]
    fn category_attestation() {
        assert_eq!(Kind::Attest.category(), KindCategory::Attestation);
        assert_eq!(Kind::CounterAttest.category(), KindCategory::Attestation);
        assert_eq!(Kind::DisputeInit.category(), KindCategory::Attestation);
        assert_eq!(Kind::VerificationResult.category(), KindCategory::Attestation);
    }

    #[test]
    fn category_inference() {
        assert_eq!(Kind::ModelAnnounce.category(), KindCategory::Inference);
        assert_eq!(Kind::ServeOffer.category(), KindCategory::Inference);
        assert_eq!(Kind::ServeResult.category(), KindCategory::Inference);
        assert_eq!(Kind::BenchmarkClaim.category(), KindCategory::Inference);
    }

    #[test]
    fn category_governance() {
        assert_eq!(Kind::Proposal.category(), KindCategory::Governance);
        assert_eq!(Kind::Vote.category(), KindCategory::Governance);
        assert_eq!(Kind::Delegate.category(), KindCategory::Governance);
        assert_eq!(Kind::ParameterChange.category(), KindCategory::Governance);
    }

    #[test]
    fn schema_nsid() {
        assert_eq!(Kind::BountyPost.schema_nsid(), "neunode.bounty.post.v1");
        assert_eq!(Kind::BountyClaim.schema_nsid(), "neunode.bounty.claim.v1");
        assert_eq!(Kind::BountySubmit.schema_nsid(), "neunode.bounty.submit.v1");
        assert_eq!(Kind::BountyReview.schema_nsid(), "neunode.bounty.review.v1");
        assert_eq!(Kind::BountyDispute.schema_nsid(), "neunode.bounty.dispute.v1");
        assert_eq!(Kind::BountyResolved.schema_nsid(), "neunode.bounty.resolved.v1");
        assert_eq!(Kind::Attest.schema_nsid(), "neunode.attest.positive.v1");
        assert_eq!(Kind::CounterAttest.schema_nsid(), "neunode.attest.negative.v1");
        assert_eq!(Kind::ModelAnnounce.schema_nsid(), "neunode.inference.announce.v1");
        assert_eq!(Kind::Proposal.schema_nsid(), "neunode.governance.proposal.v1");
        assert_eq!(Kind::Vote.schema_nsid(), "neunode.governance.vote.v1");
    }

    #[test]
    fn schema_nsid_unknown_kinds() {
        assert_eq!(Kind::AgentMetadata.schema_nsid(), "neunode.unknown.v1");
        assert_eq!(Kind::JobSubmit.schema_nsid(), "neunode.unknown.v1");
        assert_eq!(Kind::ServeOffer.schema_nsid(), "neunode.unknown.v1");
    }

    #[test]
    fn gossipsub_topic() {
        assert_eq!(Kind::AgentMetadata.gossipsub_topic(), "neunode/system");
        assert_eq!(Kind::BountyPost.gossipsub_topic(), "neunode/bounty");
        assert_eq!(Kind::JobSubmit.gossipsub_topic(), "neunode/training");
        assert_eq!(Kind::Attest.gossipsub_topic(), "neunode/attestation");
        assert_eq!(Kind::ModelAnnounce.gossipsub_topic(), "neunode/inference");
        assert_eq!(Kind::Proposal.gossipsub_topic(), "neunode/governance");
    }

    #[test]
    fn try_from_u16() {
        assert_eq!(Kind::try_from(1000).unwrap(), Kind::BountyPost);
        assert!(Kind::try_from(4).is_err());
    }

    #[test]
    fn from_kind_to_u16() {
        let val: u16 = Kind::BountyPost.into();
        assert_eq!(val, 1000);
    }

    #[test]
    fn kind_serde_roundtrip() {
        let kinds = [
            Kind::AgentMetadata,
            Kind::BountyPost,
            Kind::Checkpoint,
            Kind::Attest,
            Kind::ModelAnnounce,
            Kind::Proposal,
            Kind::ParameterChange,
        ];
        for k in kinds {
            let json = serde_json::to_string(&k).unwrap();
            let back: Kind = serde_json::from_str(&json).unwrap();
            assert_eq!(k, back);
        }
    }

    #[test]
    fn kind_category_serde_roundtrip() {
        for cat in [
            KindCategory::System,
            KindCategory::Bounty,
            KindCategory::Training,
            KindCategory::Attestation,
            KindCategory::Inference,
            KindCategory::Governance,
            KindCategory::Custom,
            KindCategory::Unknown,
        ] {
            let json = serde_json::to_string(&cat).unwrap();
            let back: KindCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(cat, back);
        }
    }

    #[test]
    fn kind_category_boundary_values() {
        assert_eq!(Kind::AgentMetadata.category(), KindCategory::System);
        assert_eq!(Kind::Lifecycle.category(), KindCategory::System);
        assert_eq!(Kind::BountyPost.category(), KindCategory::Bounty);
        assert_eq!(Kind::EscrowRefund.category(), KindCategory::Bounty);
        assert_eq!(Kind::JobSubmit.category(), KindCategory::Training);
        assert_eq!(Kind::EvalScore.category(), KindCategory::Training);
        assert_eq!(Kind::Attest.category(), KindCategory::Attestation);
        assert_eq!(Kind::VerificationResult.category(), KindCategory::Attestation);
        assert_eq!(Kind::ModelAnnounce.category(), KindCategory::Inference);
        assert_eq!(Kind::BenchmarkClaim.category(), KindCategory::Inference);
        assert_eq!(Kind::Proposal.category(), KindCategory::Governance);
        assert_eq!(Kind::ParameterChange.category(), KindCategory::Governance);
    }
}
