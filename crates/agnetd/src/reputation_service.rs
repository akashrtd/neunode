use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use neunode_core::TokenAmount;
use neunode_reputation::attestation::Attestation;
use neunode_reputation::score::FactorInputs;
use neunode_storage::{
    cf, codec,
    db::NeunodeDb,
    token_store::{TokenBalance, TokenStore},
};

/// Local ledger evidence only. Unknown inputs remain zero; no fabricated measurements.
pub fn inputs(db: &NeunodeDb, did: &str) -> Result<FactorInputs> {
    let staked =
        TokenStore::new(db).get_all_balances(did)?.iter().try_fold(0u128, |sum, balance| {
            sum.checked_add(balance.staked).ok_or_else(|| anyhow::anyhow!("stake total overflow"))
        })?;
    let total = db
        .prefix_scan(cf::CF_TOKENS, &[])?
        .iter()
        .filter(|(key, _)| key.len() == 17)
        .try_fold(0u128, |sum, (_, bytes)| {
            let balance: TokenBalance = codec::deserialize(bytes)?;
            sum.checked_add(balance.staked)
                .ok_or_else(|| anyhow::anyhow!("network stake total overflow"))
        })?;
    let attestations = attestations_for(db, did)?;
    let average = if attestations.is_empty() {
        0.0
    } else {
        attestations.iter().map(|attestation| attestation.score).sum::<f64>()
            / attestations.len() as f64
    };
    let events = neunode_storage::feed_store::FeedStore::new(db).get_all(did)?;
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    let verified: Vec<_> = events
        .iter()
        .filter_map(|row| crate::feed_wire::stored_to_event(row).ok())
        .filter(|event| {
            db.get_raw(cf::CF_FEED_INDEX, event.id.0.as_bytes())
                .ok()
                .flatten()
                .and_then(|wire| crate::feed_wire::verify_authenticated_event(&wire).ok())
                .is_some_and(|verified| verified == *event && verified.timestamp <= now)
        })
        .collect();
    let days: BTreeSet<_> = verified.iter().map(|event| event.timestamp / 86400).collect();
    let earliest = verified.iter().map(|event| event.timestamp).min();
    let age = earliest.map(|timestamp| now.saturating_sub(timestamp) / 86400).unwrap_or(0);
    let recent = verified
        .iter()
        .filter(|event| event.timestamp <= now && event.timestamp >= now.saturating_sub(86400))
        .count();
    let bounties = neunode_storage::bounty_store::BountyStore::new(db).list_all()?;
    let completed = bounties
        .iter()
        .filter(|bounty| bounty.provider_did.as_deref() == Some(did) && bounty.state == "Paid")
        .count();
    let failed = bounties
        .iter()
        .filter(|bounty| bounty.provider_did.as_deref() == Some(did) && bounty.state == "Rejected")
        .count();
    Ok(FactorInputs {
        staked_amount: TokenAmount(staked),
        total_staked: TokenAmount(total),
        attestation_count: attestations.len().try_into()?,
        avg_attestation_score: average,
        events_per_day: recent as f64,
        days_active: days.len().try_into()?,
        tasks_completed: completed.try_into()?,
        tasks_failed: failed.try_into()?,
        days_since_creation: age.try_into()?,
    })
}

pub fn attestations_for(db: &NeunodeDb, did: &str) -> Result<Vec<Attestation>> {
    let identities = neunode_storage::identity_store::IdentityStore::new(db);
    let mut latest = BTreeMap::<String, Attestation>::new();
    for (_, bytes) in db.prefix_scan(cf::CF_REPUTATION, &[])? {
        let Ok(attestation) = codec::deserialize::<Attestation>(&bytes) else { continue };
        if attestation.target.0 != did || attestation.validate().is_err() {
            continue;
        }
        let Some(document) = identities.get::<String>(&attestation.attester.0)? else { continue };
        let Ok(document) = neunode_identity::document::DidDocument::from_json(&document) else {
            continue;
        };
        let Ok(key) = document.ed25519_verifying_key() else { continue };
        if !attestation.verify(&key) {
            continue;
        }
        let entry =
            latest.entry(attestation.attester.0.clone()).or_insert_with(|| attestation.clone());
        if attestation.timestamp >= entry.timestamp {
            *entry = attestation;
        }
    }
    Ok(latest.into_values().collect())
}

pub fn candidate(
    db: &NeunodeDb,
    did: String,
    capabilities: Vec<String>,
) -> Result<neunode_discovery::AgentCandidate> {
    let inputs = inputs(db, &did)?;
    let score = neunode_reputation::score::ReputationScore::compute_default(&inputs);
    Ok(neunode_discovery::AgentCandidate {
        did,
        capabilities,
        reputation_score: score.total / 20.0,
        stake_amount: inputs.staked_amount.0.try_into()?,
        availability_score: 0.0,
        latency_ms: u32::MAX,
        cost_per_unit: f64::MAX,
        is_online: false,
    })
}

pub fn leaderboard(db: &NeunodeDb) -> Result<Vec<(String, f64)>> {
    let mut agents = BTreeSet::new();
    for (_, bytes) in db.prefix_scan(cf::CF_REPUTATION, &[])? {
        if let Ok(attestation) = codec::deserialize::<Attestation>(&bytes) {
            agents.insert(attestation.target.0);
        }
    }
    for (key, bytes) in db.prefix_scan(cf::CF_IDENTITY, &[])? {
        if let (Ok(did), Ok(document)) =
            (codec::deserialize::<String>(&key), codec::deserialize::<String>(&bytes))
        {
            if neunode_identity::document::DidDocument::from_json(&document)
                .is_ok_and(|doc| doc.id == did)
            {
                agents.insert(did);
            }
        }
    }
    let mut ranked = agents
        .into_iter()
        .map(|did| {
            let score =
                neunode_reputation::score::ReputationScore::compute_default(&inputs(db, &did)?);
            Ok((did, score.total))
        })
        .collect::<Result<Vec<_>>>()?;
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Ok(ranked)
}
