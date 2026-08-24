//! Validator vote verification, quorum certificates, and equivocation detection.

use std::collections::BTreeMap;

use alloy::primitives::{Address, B256};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::{BridgeError, Result, ValidatorSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VoteStep {
    Prevote,
    Precommit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedVote {
    pub height: u64,
    pub round: i64,
    pub step: VoteStep,
    pub block_hash: B256,
    pub validator: Address,
    pub signature: Vec<u8>,
}

impl SignedVote {
    pub fn sign(
        height: u64,
        round: i64,
        step: VoteStep,
        block_hash: B256,
        validator: Address,
        key: &SigningKey,
    ) -> Self {
        let bytes = sign_bytes(height, round, step, block_hash);
        Self {
            height,
            round,
            step,
            block_hash,
            validator,
            signature: key.sign(&bytes).to_bytes().to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleSignEvidence {
    pub first: SignedVote,
    pub second: SignedVote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitCertificate {
    pub height: u64,
    pub round: i64,
    pub block_hash: B256,
    pub signed_power: u64,
    pub total_power: u64,
    pub votes: Vec<SignedVote>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConsensusSnapshot {
    pub certificates: Vec<CommitCertificate>,
}

/// Collects authenticated votes for one height and rejects equivocation.
pub struct VoteCollector {
    validators: ValidatorSet,
    keys: BTreeMap<Address, VerifyingKey>,
    votes: BTreeMap<(u64, i64, VoteStep, Address), SignedVote>,
    evidence: Vec<DoubleSignEvidence>,
}

impl VoteCollector {
    pub fn new(
        validators: ValidatorSet,
        keys: impl IntoIterator<Item = (Address, VerifyingKey)>,
    ) -> Result<Self> {
        let keys = keys.into_iter().collect::<BTreeMap<_, _>>();
        if validators.validators.iter().any(|validator| !keys.contains_key(&validator.address)) {
            return Err(BridgeError::InvalidProposal(
                "validator set contains an address without a verification key".into(),
            ));
        }
        Ok(Self { validators, keys, votes: BTreeMap::new(), evidence: Vec::new() })
    }

    pub fn add_vote(&mut self, vote: SignedVote) -> Result<Option<CommitCertificate>> {
        self.validators
            .validators
            .iter()
            .find(|validator| validator.address == vote.validator)
            .map(|validator| validator.voting_power)
            .ok_or_else(|| BridgeError::InvalidProposal("vote from unknown validator".into()))?;
        let key = self.keys.get(&vote.validator).expect("keys checked at construction");
        let signature = Signature::from_slice(&vote.signature)
            .map_err(|error| BridgeError::InvalidProposal(error.to_string()))?;
        key.verify(&sign_bytes(vote.height, vote.round, vote.step, vote.block_hash), &signature)
            .map_err(|_| BridgeError::InvalidProposal("invalid validator signature".into()))?;

        let position = (vote.height, vote.round, vote.step, vote.validator);
        if let Some(existing) = self.votes.get(&position) {
            if existing.block_hash != vote.block_hash {
                self.evidence.push(DoubleSignEvidence { first: existing.clone(), second: vote });
                return Err(BridgeError::InvalidProposal("validator equivocation detected".into()));
            }
            return Ok(self.certificate_for(existing.height, existing.round, existing.block_hash));
        }
        let height = vote.height;
        let round = vote.round;
        let block_hash = vote.block_hash;
        let step = vote.step;
        self.votes.insert(position, vote);
        if step != VoteStep::Precommit {
            return Ok(None);
        }
        Ok(self.certificate_for(height, round, block_hash))
    }

    pub fn evidence(&self) -> &[DoubleSignEvidence] {
        &self.evidence
    }

    /// Verify an externally received certificate before using it for state sync.
    pub fn verify_certificate(&self, certificate: &CommitCertificate) -> Result<()> {
        let mut verifier = Self::new(
            self.validators.clone(),
            self.keys.iter().map(|(address, key)| (*address, *key)),
        )?;
        let mut verified = None;
        for vote in &certificate.votes {
            if vote.height != certificate.height
                || vote.round != certificate.round
                || vote.step != VoteStep::Precommit
                || vote.block_hash != certificate.block_hash
            {
                return Err(BridgeError::InvalidProposal(
                    "certificate contains a vote for a different decision".into(),
                ));
            }
            verified = verifier.add_vote(vote.clone())?.or(verified);
        }
        let verified = verified.ok_or_else(|| {
            BridgeError::InvalidProposal("certificate does not contain >2/3 voting power".into())
        })?;
        if verified.signed_power != certificate.signed_power
            || verified.total_power != certificate.total_power
        {
            return Err(BridgeError::InvalidProposal(
                "certificate voting-power metadata is inconsistent".into(),
            ));
        }
        Ok(())
    }

    /// Verify a consecutive sequence of finalized decisions received from a peer.
    pub fn verify_snapshot(&self, snapshot: &ConsensusSnapshot, after_height: u64) -> Result<()> {
        let mut expected = after_height + 1;
        for certificate in &snapshot.certificates {
            if certificate.height != expected {
                return Err(BridgeError::InvalidProposal(format!(
                    "state sync height gap: expected {expected}, got {}",
                    certificate.height
                )));
            }
            self.verify_certificate(certificate)?;
            expected += 1;
        }
        Ok(())
    }

    fn certificate_for(
        &self,
        height: u64,
        round: i64,
        block_hash: B256,
    ) -> Option<CommitCertificate> {
        let votes = self
            .votes
            .values()
            .filter(|vote| {
                vote.height == height
                    && vote.round == round
                    && vote.step == VoteStep::Precommit
                    && vote.block_hash == block_hash
            })
            .cloned()
            .collect::<Vec<_>>();
        let signed_power: u64 = votes
            .iter()
            .filter_map(|vote| {
                self.validators
                    .validators
                    .iter()
                    .find(|validator| validator.address == vote.validator)
            })
            .map(|validator| validator.voting_power)
            .sum();
        if signed_power.saturating_mul(3) <= self.validators.total_voting_power.saturating_mul(2) {
            return None;
        }
        Some(CommitCertificate {
            height,
            round,
            block_hash,
            signed_power,
            total_power: self.validators.total_voting_power,
            votes,
        })
    }
}

fn sign_bytes(height: u64, round: i64, step: VoteStep, block_hash: B256) -> Vec<u8> {
    let mut bytes = b"neunode-consensus-v1".to_vec();
    bytes.extend(height.to_be_bytes());
    bytes.extend(round.to_be_bytes());
    bytes.push(match step {
        VoteStep::Prevote => 0,
        VoteStep::Precommit => 1,
    });
    bytes.extend(block_hash.as_slice());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValidatorInfo;
    fn network(count: usize) -> (VoteCollector, Vec<(Address, SigningKey)>) {
        let signers = (0..count)
            .map(|index| {
                (
                    Address::from_word(B256::from(U256::from(index + 1))),
                    SigningKey::from_bytes(&[(index + 1) as u8; 32]),
                )
            })
            .collect::<Vec<_>>();
        let validators = ValidatorSet {
            validators: signers
                .iter()
                .map(|(address, _)| ValidatorInfo { address: *address, voting_power: 1 })
                .collect(),
            total_voting_power: count as u64,
        };
        let keys = signers.iter().map(|(address, key)| (*address, key.verifying_key()));
        (VoteCollector::new(validators, keys).unwrap(), signers)
    }

    use alloy::primitives::U256;

    #[test]
    fn four_validators_finalize_with_one_offline() {
        let (mut collector, validators) = network(4);
        let hash = B256::repeat_byte(7);
        for (index, (address, key)) in validators.iter().take(3).enumerate() {
            let certificate = collector
                .add_vote(SignedVote::sign(9, 0, VoteStep::Precommit, hash, *address, key))
                .unwrap();
            assert_eq!(certificate.is_some(), index == 2);
        }
        let certificate = collector.certificate_for(9, 0, hash).unwrap();
        assert_eq!(certificate.signed_power, 3);
        assert_eq!(certificate.total_power, 4);
    }

    #[test]
    fn two_of_four_cannot_finalize() {
        let (mut collector, validators) = network(4);
        let hash = B256::repeat_byte(8);
        for (address, key) in validators.iter().take(2) {
            assert!(collector
                .add_vote(SignedVote::sign(2, 0, VoteStep::Precommit, hash, *address, key))
                .unwrap()
                .is_none());
        }
    }

    #[test]
    fn detects_double_signing() {
        let (mut collector, validators) = network(4);
        let (address, key) = &validators[0];
        collector
            .add_vote(SignedVote::sign(
                3,
                1,
                VoteStep::Precommit,
                B256::repeat_byte(1),
                *address,
                key,
            ))
            .unwrap();
        let error = collector
            .add_vote(SignedVote::sign(
                3,
                1,
                VoteStep::Precommit,
                B256::repeat_byte(2),
                *address,
                key,
            ))
            .unwrap_err();
        assert!(error.to_string().contains("equivocation"));
        assert_eq!(collector.evidence().len(), 1);
    }

    #[test]
    fn rejects_forged_vote() {
        let (mut collector, validators) = network(4);
        let (address, _) = &validators[0];
        let forged = SignedVote::sign(
            1,
            0,
            VoteStep::Precommit,
            B256::repeat_byte(4),
            *address,
            &validators[1].1,
        );
        assert!(collector.add_vote(forged).unwrap_err().to_string().contains("signature"));
    }

    #[test]
    fn joining_validator_verifies_missed_certificates() {
        let (mut producer, validators) = network(4);
        let mut certificates = Vec::new();
        for height in 1..=5 {
            let hash = B256::from(U256::from(height));
            let mut certificate = None;
            for (address, key) in validators.iter().take(3) {
                certificate = producer
                    .add_vote(SignedVote::sign(height, 0, VoteStep::Precommit, hash, *address, key))
                    .unwrap()
                    .or(certificate);
            }
            certificates.push(certificate.unwrap());
        }

        let (joining_validator, _) = network(4);
        joining_validator.verify_snapshot(&ConsensusSnapshot { certificates }, 0).unwrap();
    }

    #[test]
    fn state_sync_rejects_height_gaps_and_tampering() {
        let (mut producer, validators) = network(4);
        let hash = B256::repeat_byte(9);
        let mut certificate = None;
        for (address, key) in validators.iter().take(3) {
            certificate = producer
                .add_vote(SignedVote::sign(2, 0, VoteStep::Precommit, hash, *address, key))
                .unwrap()
                .or(certificate);
        }
        let mut certificate = certificate.unwrap();
        assert!(producer
            .verify_snapshot(&ConsensusSnapshot { certificates: vec![certificate.clone()] }, 0)
            .unwrap_err()
            .to_string()
            .contains("height gap"));

        certificate.signed_power = 4;
        assert!(producer.verify_certificate(&certificate).is_err());
    }
}
