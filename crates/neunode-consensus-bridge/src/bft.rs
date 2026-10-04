//! Validator vote verification, quorum certificates, and equivocation detection.

use std::collections::BTreeMap;

use alloy::primitives::{Address, B256};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::{BridgeError, Result, ValidatorSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VoteStep {
    Prevote,
    Precommit,
}

/// Scope prevents certificates being replayed across networks or validator epochs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusDomain {
    pub chain_id: u64,
    pub genesis_hash: B256,
    pub epoch: u64,
    pub validator_set_hash: B256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedVote {
    pub domain: ConsensusDomain,
    pub height: u64,
    pub round: i64,
    pub step: VoteStep,
    pub block_hash: B256,
    pub validator: Address,
    pub signature: Vec<u8>,
}

impl SignedVote {
    pub fn sign(
        domain: &ConsensusDomain,
        height: u64,
        round: i64,
        step: VoteStep,
        block_hash: B256,
        validator: Address,
        key: &SigningKey,
    ) -> Self {
        let bytes = sign_bytes(domain, height, round, step, block_hash);
        Self {
            domain: domain.clone(),
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
    pub domain: ConsensusDomain,
    validators: ValidatorSet,
    keys: BTreeMap<Address, VerifyingKey>,
    votes: BTreeMap<(u64, i64, VoteStep, Address), SignedVote>,
    evidence: Vec<DoubleSignEvidence>,
}

impl VoteCollector {
    pub fn new(
        validators: ValidatorSet,
        keys: impl IntoIterator<Item = (Address, VerifyingKey)>,
        chain_id: u64,
        genesis_hash: B256,
        epoch: u64,
    ) -> Result<Self> {
        let invalid = |reason: &str| BridgeError::InvalidProposal(reason.into());
        let mut members = BTreeMap::new();
        let mut total = 0u64;
        for validator in &validators.validators {
            if validator.voting_power == 0
                || members.insert(validator.address, validator.voting_power).is_some()
            {
                return Err(invalid("zero-power or duplicate validator"));
            }
            total = total
                .checked_add(validator.voting_power)
                .ok_or_else(|| invalid("validator power overflow"))?;
        }
        if total == 0 || total != validators.total_voting_power {
            return Err(invalid("inconsistent validator total power"));
        }
        let mut verified_keys = BTreeMap::new();
        for (address, key) in keys {
            if !members.contains_key(&address) || verified_keys.insert(address, key).is_some() {
                return Err(invalid("unknown or duplicate validator key"));
            }
        }
        let keys = verified_keys;
        let mut binding = Vec::new();
        for (address, power) in &members {
            binding.extend(address.as_slice());
            binding.extend(power.to_be_bytes());
            let key =
                keys.get(address).ok_or_else(|| invalid("validator verification key missing"))?;
            binding.extend(key.as_bytes());
        }
        let domain = ConsensusDomain {
            chain_id,
            genesis_hash,
            epoch,
            validator_set_hash: alloy::primitives::keccak256(binding),
        };
        if validators.validators.iter().any(|validator| !keys.contains_key(&validator.address)) {
            return Err(BridgeError::InvalidProposal(
                "validator set contains an address without a verification key".into(),
            ));
        }
        Ok(Self { domain, validators, keys, votes: BTreeMap::new(), evidence: Vec::new() })
    }

    pub fn add_vote(&mut self, vote: SignedVote) -> Result<Option<CommitCertificate>> {
        if vote.domain != self.domain || vote.height == 0 || vote.round < 0 {
            return Err(BridgeError::InvalidProposal(
                "invalid consensus domain, height or round".into(),
            ));
        }
        self.validators
            .validators
            .iter()
            .find(|validator| validator.address == vote.validator)
            .map(|validator| validator.voting_power)
            .ok_or_else(|| BridgeError::InvalidProposal("vote from unknown validator".into()))?;
        let key = self.keys.get(&vote.validator).expect("keys checked at construction");
        let signature = Signature::from_slice(&vote.signature)
            .map_err(|error| BridgeError::InvalidProposal(error.to_string()))?;
        key.verify_strict(
            &sign_bytes(&vote.domain, vote.height, vote.round, vote.step, vote.block_hash),
            &signature,
        )
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
            self.domain.chain_id,
            self.domain.genesis_hash,
            self.domain.epoch,
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
        let mut expected = after_height;
        for certificate in &snapshot.certificates {
            expected = expected
                .checked_add(1)
                .ok_or_else(|| BridgeError::InvalidProposal("state sync height overflow".into()))?;
            if certificate.height != expected {
                return Err(BridgeError::InvalidProposal(format!(
                    "state sync height gap: expected {expected}, got {}",
                    certificate.height
                )));
            }
            self.verify_certificate(certificate)?;
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
        if u128::from(signed_power) * 3 <= u128::from(self.validators.total_voting_power) * 2 {
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

fn sign_bytes(
    domain: &ConsensusDomain,
    height: u64,
    round: i64,
    step: VoteStep,
    block_hash: B256,
) -> Vec<u8> {
    let mut bytes = b"neunode-consensus-v2".to_vec();
    bytes.extend(domain.chain_id.to_be_bytes());
    bytes.extend(domain.genesis_hash.as_slice());
    bytes.extend(domain.epoch.to_be_bytes());
    bytes.extend(domain.validator_set_hash.as_slice());
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
        (VoteCollector::new(validators, keys, 2026001, B256::repeat_byte(42), 0).unwrap(), signers)
    }

    use alloy::primitives::U256;

    #[test]
    fn votes_are_scoped_and_validator_metadata_is_checked() {
        let (mut collector, signers) = network(4);
        for field in 0..4 {
            let mut domain = collector.domain.clone();
            match field {
                0 => domain.chain_id += 1,
                1 => domain.genesis_hash = B256::ZERO,
                2 => domain.epoch += 1,
                _ => domain.validator_set_hash = B256::ZERO,
            }
            let vote = SignedVote::sign(
                &domain,
                1,
                0,
                VoteStep::Precommit,
                B256::ZERO,
                signers[0].0,
                &signers[0].1,
            );
            assert!(collector.add_vote(vote).is_err());
        }
        let keys = || signers.iter().map(|(address, key)| (*address, key.verifying_key()));
        let mut set = collector.validators.clone();
        set.total_voting_power = 1;
        assert!(VoteCollector::new(set, keys(), 1, B256::ZERO, 0).is_err());
        let mut set = collector.validators.clone();
        set.validators[1].address = set.validators[0].address;
        assert!(VoteCollector::new(set, keys(), 1, B256::ZERO, 0).is_err());
        let mut set = collector.validators.clone();
        set.validators[0].voting_power = 0;
        set.total_voting_power = 3;
        assert!(VoteCollector::new(set, keys(), 1, B256::ZERO, 0).is_err());
        assert!(collector
            .verify_snapshot(&ConsensusSnapshot { certificates: vec![] }, u64::MAX)
            .is_ok());
    }

    #[test]
    fn four_validators_finalize_with_one_offline() {
        let (mut collector, validators) = network(4);
        let hash = B256::repeat_byte(7);
        for (index, (address, key)) in validators.iter().take(3).enumerate() {
            let certificate = collector
                .add_vote(SignedVote::sign(
                    &collector.domain,
                    9,
                    0,
                    VoteStep::Precommit,
                    hash,
                    *address,
                    key,
                ))
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
                .add_vote(SignedVote::sign(
                    &collector.domain,
                    2,
                    0,
                    VoteStep::Precommit,
                    hash,
                    *address,
                    key
                ))
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
                &collector.domain,
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
                &collector.domain,
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
            &collector.domain,
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
                    .add_vote(SignedVote::sign(
                        &producer.domain,
                        height,
                        0,
                        VoteStep::Precommit,
                        hash,
                        *address,
                        key,
                    ))
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
                .add_vote(SignedVote::sign(
                    &producer.domain,
                    2,
                    0,
                    VoteStep::Precommit,
                    hash,
                    *address,
                    key,
                ))
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
