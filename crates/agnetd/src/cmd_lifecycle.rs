use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::cli::{GlobalArgs, LifecycleCommands};
use crate::output::OutputWriter;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Lifecycle state definitions (Doc 16)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AgentState {
    Created,
    Active,
    Hibernating,
    Idle,
    Zombie,
    Dead,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Created => write!(f, "CREATED"),
            AgentState::Active => write!(f, "ACTIVE"),
            AgentState::Hibernating => write!(f, "HIBERNATING"),
            AgentState::Idle => write!(f, "IDLE"),
            AgentState::Zombie => write!(f, "ZOMBIE"),
            AgentState::Dead => write!(f, "DEAD"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LifecycleRecord {
    pub did: String,
    pub state: AgentState,
    pub last_activity: u64,
    pub activated_at: Option<u64>,
    pub hibernated_at: Option<u64>,
    pub tombstoned_at: Option<u64>,
    pub derived_from: Option<String>,
}

// ---------------------------------------------------------------------------
// Storage helpers (use identity CF with lifecycle key prefix)
// ---------------------------------------------------------------------------

const LIFECYCLE_PREFIX: &str = "lifecycle:";

fn lifecycle_key(did: &str) -> String {
    format!("{LIFECYCLE_PREFIX}{did}")
}

fn get_record(db: &neunode_storage::db::NeunodeDb, did: &str) -> Result<Option<LifecycleRecord>> {
    let key = lifecycle_key(did);
    let store = neunode_storage::identity_store::IdentityStore::new(db);
    Ok(store.get(&key)?)
}

fn put_record(db: &neunode_storage::db::NeunodeDb, record: &LifecycleRecord) -> Result<()> {
    let key = lifecycle_key(&record.did);
    let store = neunode_storage::identity_store::IdentityStore::new(db);
    store.put(&key, record)?;
    Ok(())
}

fn all_records(db: &neunode_storage::db::NeunodeDb) -> Result<Vec<LifecycleRecord>> {
    let entries = db.prefix_scan(neunode_storage::cf::CF_IDENTITY, &[])?;
    let mut records = Vec::new();
    for (key, value) in entries {
        let Ok(key) = bincode::deserialize::<String>(&key) else {
            continue;
        };
        if key.starts_with(LIFECYCLE_PREFIX) {
            let record = bincode::deserialize::<LifecycleRecord>(&value)
                .map_err(|e| anyhow::anyhow!("corrupt lifecycle record {key}: {e}"))?;
            records.push(record);
        }
    }
    Ok(records)
}

fn now_ts() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

// Thresholds (Doc 16)
const IDLE_THRESHOLD_SECS: u64 = 7 * 86400;
const ZOMBIE_THRESHOLD_SECS: u64 = 30 * 86400;
const DEATH_THRESHOLD_SECS: u64 = 90 * 86400;
const ZOMBIE_WARNING_SECS: u64 = 25 * 86400;

// ---------------------------------------------------------------------------
// Command handler
// ---------------------------------------------------------------------------

pub fn execute(cmd: &LifecycleCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        LifecycleCommands::Status => show_status(&writer, state),
        LifecycleCommands::Activate => activate(&writer, state),
        LifecycleCommands::Hibernate => hibernate(&writer, state),
        LifecycleCommands::Reactivate => reactivate(&writer, state),
        LifecycleCommands::List => list_agents(&writer, state),
        LifecycleCommands::Reap => reap(&writer, state),
    }
}

fn show_status(writer: &OutputWriter, state: &AppState) -> Result<()> {
    let did = state.require_did()?;

    match get_record(&state.db, &did.0)? {
        Some(record) => {
            let elapsed = now_ts().saturating_sub(record.last_activity);
            writer.write_value("DID", &did.0);
            writer.write_value("State", &record.state.to_string());
            writer.write_value("Last Activity", &record.last_activity.to_string());
            writer.write_value("Elapsed", &format!("{elapsed}s"));

            if let Some(t) = record.activated_at {
                writer.write_value("Activated At", &t.to_string());
            }

            // Show warnings
            if record.state == AgentState::Active && elapsed > ZOMBIE_WARNING_SECS {
                let days_to_zombie = (ZOMBIE_THRESHOLD_SECS.saturating_sub(elapsed)) / 86400;
                writer.write_warning(&format!(
                    "Zombie warning: {days_to_zombie} days until zombification"
                ));
            }
        }
        None => {
            writer
                .write_status("No lifecycle record. Run `agnetd lifecycle activate` to register.");
        }
    }
    Ok(())
}

fn activate(writer: &OutputWriter, state: &AppState) -> Result<()> {
    let did = state.require_did()?;
    let now = now_ts();

    match get_record(&state.db, &did.0)? {
        Some(mut record) => {
            if record.state == AgentState::Active {
                anyhow::bail!("agent is already ACTIVE");
            }
            if record.state == AgentState::Dead {
                anyhow::bail!("agent is DEAD (tombstoned, irreversible)");
            }
            record.state = AgentState::Active;
            record.last_activity = now;
            put_record(&state.db, &record)?;
            writer.write_status(&format!("Agent {} reactivated", did.0));
        }
        None => {
            let record = LifecycleRecord {
                did: did.0.clone(),
                state: AgentState::Active,
                last_activity: now,
                activated_at: Some(now),
                hibernated_at: None,
                tombstoned_at: None,
                derived_from: None,
            };
            put_record(&state.db, &record)?;
            writer.write_status(&format!("Agent {} activated (immunity period: 14 days)", did.0));
        }
    }
    Ok(())
}

fn hibernate(writer: &OutputWriter, state: &AppState) -> Result<()> {
    let did = state.require_did()?;
    let now = now_ts();

    let mut record = get_record(&state.db, &did.0)?
        .ok_or_else(|| anyhow::anyhow!("no lifecycle record -- run `agnetd lifecycle activate`"))?;

    if record.state != AgentState::Active {
        anyhow::bail!("can only hibernate from ACTIVE state, current: {}", record.state);
    }

    record.state = AgentState::Hibernating;
    record.hibernated_at = Some(now);
    put_record(&state.db, &record)?;

    writer.write_status(&format!("Agent {} hibernating (state preserved)", did.0));
    Ok(())
}

fn reactivate(writer: &OutputWriter, state: &AppState) -> Result<()> {
    let did = state.require_did()?;
    let now = now_ts();

    let mut record = get_record(&state.db, &did.0)?
        .ok_or_else(|| anyhow::anyhow!("no lifecycle record -- run `agnetd lifecycle activate`"))?;

    if record.state != AgentState::Hibernating {
        anyhow::bail!("can only reactivate from HIBERNATING, current: {}", record.state);
    }

    record.state = AgentState::Active;
    record.last_activity = now;
    record.hibernated_at = None;
    put_record(&state.db, &record)?;

    writer.write_status(&format!("Agent {} reactivated from hibernation", did.0));
    Ok(())
}

fn list_agents(writer: &OutputWriter, state: &AppState) -> Result<()> {
    // Scan identity CF for lifecycle: prefix
    let records = all_records(state.db())?;
    if records.is_empty() {
        writer.write_status("No agents registered");
        return Ok(());
    }

    let headers = ["DID", "State", "Last Activity"];
    let rows: Vec<Vec<String>> = records
        .iter()
        .map(|record| {
            let did_short = if record.did.len() > 24 {
                format!("{}...", &record.did[..24])
            } else {
                record.did.clone()
            };
            vec![did_short, record.state.to_string(), record.last_activity.to_string()]
        })
        .collect();

    writer.write_table(&headers, &rows);
    Ok(())
}

fn reap(writer: &OutputWriter, state: &AppState) -> Result<()> {
    let db = state.db();
    let now = now_ts();

    let records = all_records(db)?;
    let mut transitions: Vec<(String, String, String)> = Vec::new();

    for mut record in records {
        let elapsed = now.saturating_sub(record.last_activity);
        let old_state = record.state.clone();

        match record.state {
            AgentState::Active | AgentState::Idle => {
                if elapsed >= ZOMBIE_THRESHOLD_SECS {
                    record.state = AgentState::Zombie;
                } else if elapsed >= IDLE_THRESHOLD_SECS && record.state == AgentState::Active {
                    record.state = AgentState::Idle;
                }
            }
            AgentState::Zombie => {
                if elapsed >= DEATH_THRESHOLD_SECS {
                    record.state = AgentState::Dead;
                    record.tombstoned_at = Some(now);
                }
            }
            AgentState::Hibernating => {
                // Check max hibernation (180 days)
                if let Some(hib_at) = record.hibernated_at {
                    if now.saturating_sub(hib_at) >= 180 * 86400 {
                        record.state = AgentState::Dead;
                        record.tombstoned_at = Some(now);
                    }
                }
            }
            AgentState::Created | AgentState::Dead => {}
        }

        if record.state != old_state {
            transitions.push((record.did.clone(), old_state.to_string(), record.state.to_string()));
            put_record(db, &record)?;
        }
    }

    if transitions.is_empty() {
        writer.write_status("No state transitions needed");
    } else {
        let headers = ["Agent DID", "From", "To"];
        let rows: Vec<Vec<String>> = transitions
            .iter()
            .map(|(did, from, to)| {
                let did_short =
                    if did.len() > 24 { format!("{}...", &did[..24]) } else { did.clone() };
                vec![did_short, from.clone(), to.clone()]
            })
            .collect();
        writer.write_table(&headers, &rows);
        writer.write_status(&format!("Processed {} state transitions", transitions.len()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{human_writer, test_state};

    #[test]
    fn encoded_lifecycle_keys_are_listed_and_reaped() {
        let state = test_state();
        let did = state.require_did().unwrap().0.clone();
        let record = LifecycleRecord {
            did: did.clone(),
            state: AgentState::Active,
            last_activity: now_ts().saturating_sub(IDLE_THRESHOLD_SECS + 1),
            activated_at: Some(1),
            hibernated_at: None,
            tombstoned_at: None,
            derived_from: None,
        };
        put_record(state.db(), &record).unwrap();

        let records = all_records(state.db()).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].did, did);
        list_agents(&human_writer(), &state).unwrap();
        reap(&human_writer(), &state).unwrap();

        let updated = get_record(state.db(), &did).unwrap().unwrap();
        assert_eq!(updated.state, AgentState::Idle);
    }
}
