//! Fault tolerance for DiLoCo training.
//!
//! Tracks worker heartbeats, detects stragglers, handles elastic membership
//! (workers joining/leaving mid-training), and triggers checkpoint recovery
//! when too many workers fail.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::worker::WorkerId;

// ---------------------------------------------------------------------------
// Health state
// ---------------------------------------------------------------------------

/// Health state of a monitored worker.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    /// Worker is within heartbeat timeout.
    Healthy,
    /// Worker is approaching timeout (last heartbeat within 50% of timeout).
    Suspect {
        /// Milliseconds since the last heartbeat.
        #[ts(type = "number")]
        last_seen_ms: u64,
    },
    /// Worker has exceeded heartbeat timeout.
    Dead,
}

// ---------------------------------------------------------------------------
// Worker health record
// ---------------------------------------------------------------------------

/// Health tracking record for a single worker.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkerHealth {
    /// Worker identifier.
    pub worker_id: WorkerId,
    /// Timestamp of the last received heartbeat (ms since epoch).
    #[ts(type = "number")]
    pub last_heartbeat_ms: u64,
    /// Current health state.
    pub state: HealthState,
    /// Number of consecutive missed heartbeat checks.
    #[ts(type = "number")]
    pub missed_heartbeats: u32,
}

// ---------------------------------------------------------------------------
// Fault events
// ---------------------------------------------------------------------------

/// Events emitted by [`HealthMonitor`] during health checks and heartbeats.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum FaultEvent {
    /// Worker is approaching heartbeat timeout.
    WorkerSuspect { worker_id: WorkerId },
    /// Worker has exceeded heartbeat timeout.
    WorkerDead { worker_id: WorkerId },
    /// A previously suspect/dead worker sent a heartbeat and recovered.
    WorkerRecovered { worker_id: WorkerId },
    /// Majority of workers are dead — checkpoint recovery should be triggered.
    RecoveryNeeded { dead_workers: Vec<WorkerId> },
}

// ---------------------------------------------------------------------------
// Health monitor
// ---------------------------------------------------------------------------

/// Monitors worker heartbeats and detects failures.
///
/// # Health logic
///
/// Given a configured `timeout_secs`:
/// - `elapsed < timeout` → **Healthy**
/// - `timeout <= elapsed < timeout * 2` → **Suspect**
/// - `elapsed >= timeout * 2` → **Dead**
///
/// When 50% or more of registered workers are **Dead**, a [`FaultEvent::RecoveryNeeded`]
/// event is emitted.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HealthMonitor {
    /// Heartbeat timeout in seconds.
    #[ts(type = "number")]
    timeout_secs: u64,
    /// Per-worker health records.
    workers: HashMap<WorkerId, WorkerHealth>,
}

impl HealthMonitor {
    /// Create a new health monitor with the given heartbeat timeout (seconds).
    pub fn new(timeout_secs: u64) -> Self {
        Self { timeout_secs, workers: HashMap::new() }
    }

    /// Register a new worker for health monitoring.
    pub fn register(&mut self, worker_id: WorkerId, now_ms: u64) {
        self.workers.insert(
            worker_id.clone(),
            WorkerHealth {
                worker_id,
                last_heartbeat_ms: now_ms,
                state: HealthState::Healthy,
                missed_heartbeats: 0,
            },
        );
    }

    /// Record a heartbeat from a worker.
    ///
    /// Returns events:
    /// - [`FaultEvent::WorkerRecovered`] if the worker was previously Suspect or Dead.
    pub fn heartbeat(&mut self, worker_id: &WorkerId, now_ms: u64) -> Vec<FaultEvent> {
        let Some(health) = self.workers.get_mut(worker_id) else {
            return Vec::new();
        };

        let previous_state = health.state;
        health.last_heartbeat_ms = now_ms;
        health.state = HealthState::Healthy;
        health.missed_heartbeats = 0;

        match previous_state {
            HealthState::Suspect { .. } | HealthState::Dead => {
                vec![FaultEvent::WorkerRecovered { worker_id: worker_id.clone() }]
            }
            HealthState::Healthy => Vec::new(),
        }
    }

    /// Check all workers and emit state transitions.
    ///
    /// Returns events:
    /// - [`FaultEvent::WorkerSuspect`] on Healthy → Suspect transition.
    /// - [`FaultEvent::WorkerDead`] on Suspect → Dead transition.
    /// - [`FaultEvent::RecoveryNeeded`] if 50%+ of workers are Dead after evaluation.
    pub fn check(&mut self, now_ms: u64) -> Vec<FaultEvent> {
        let timeout_ms = self.timeout_secs * 1000;
        let mut events = Vec::new();

        for health in self.workers.values_mut() {
            let elapsed = now_ms.saturating_sub(health.last_heartbeat_ms);
            let new_state = Self::classify(elapsed, timeout_ms);

            let transition = match (health.state, new_state) {
                (HealthState::Healthy, HealthState::Suspect { .. }) => {
                    health.missed_heartbeats += 1;
                    Some(FaultEvent::WorkerSuspect { worker_id: health.worker_id.clone() })
                }
                (HealthState::Healthy, HealthState::Dead) => {
                    health.missed_heartbeats += 1;
                    // Healthy → Dead in one check (clock jumped past suspect window).
                    Some(FaultEvent::WorkerDead { worker_id: health.worker_id.clone() })
                }
                (HealthState::Suspect { .. }, HealthState::Dead) => {
                    health.missed_heartbeats += 1;
                    Some(FaultEvent::WorkerDead { worker_id: health.worker_id.clone() })
                }
                // Suspect → still Suspect (update last_seen_ms) — no event.
                (HealthState::Suspect { .. }, HealthState::Suspect { .. }) => {
                    health.missed_heartbeats += 1;
                    None
                }
                // Re-state-changes that don't produce a transition event.
                (HealthState::Suspect { .. }, HealthState::Healthy) => None,
                (HealthState::Dead, HealthState::Dead) => {
                    health.missed_heartbeats += 1;
                    None
                }
                // Dead recovering via check is not valid — use heartbeat().
                // Healthy → Healthy, no transition.
                _ => None,
            };

            health.state = new_state;
            if let Some(event) = transition {
                events.push(event);
            }
        }

        // Check if recovery is needed (majority dead).
        let dead_count = self.dead_workers().len();
        let total = self.total_count();
        if total > 0 && dead_count * 2 >= total {
            events.push(FaultEvent::RecoveryNeeded { dead_workers: self.dead_workers() });
        }

        events
    }

    /// Remove a worker from monitoring.
    pub fn remove(&mut self, worker_id: &WorkerId) {
        self.workers.remove(worker_id);
    }

    /// Count of workers currently in [`HealthState::Healthy`].
    pub fn healthy_count(&self) -> usize {
        self.workers.values().filter(|h| h.state == HealthState::Healthy).count()
    }

    /// Total number of registered workers.
    pub fn total_count(&self) -> usize {
        self.workers.len()
    }

    /// Get the health record for a specific worker.
    pub fn get_health(&self, worker_id: &WorkerId) -> Option<&WorkerHealth> {
        self.workers.get(worker_id)
    }

    /// List IDs of all dead workers.
    pub fn dead_workers(&self) -> Vec<WorkerId> {
        self.workers
            .values()
            .filter(|h| h.state == HealthState::Dead)
            .map(|h| h.worker_id.clone())
            .collect()
    }

    /// Classify elapsed time into a health state.
    fn classify(elapsed_ms: u64, timeout_ms: u64) -> HealthState {
        if elapsed_ms >= timeout_ms * 2 {
            HealthState::Dead
        } else if elapsed_ms >= timeout_ms {
            HealthState::Suspect { last_seen_ms: elapsed_ms }
        } else {
            HealthState::Healthy
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a monitor with 10-second timeout.
    fn test_monitor() -> HealthMonitor {
        HealthMonitor::new(10)
    }

    /// Timeout in milliseconds (10 seconds).
    const TIMEOUT_MS: u64 = 10_000;

    fn wid(name: &str) -> WorkerId {
        WorkerId(name.to_string())
    }

    // ── Register + check healthy ───────────────────────────────────────

    #[test]
    fn register_then_check_healthy() {
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        let events = m.check(1000);
        assert!(events.is_empty());
        assert_eq!(m.get_health(&w).unwrap().state, HealthState::Healthy);
    }

    #[test]
    fn register_multiple_workers_healthy() {
        let mut m = test_monitor();
        m.register(wid("w1"), 0);
        m.register(wid("w2"), 0);
        m.register(wid("w3"), 0);
        let events = m.check(5000);
        assert!(events.is_empty());
        assert_eq!(m.healthy_count(), 3);
        assert_eq!(m.total_count(), 3);
    }

    // ── Heartbeat updates ──────────────────────────────────────────────

    #[test]
    fn heartbeat_updates_last_seen() {
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        let events = m.heartbeat(&w, 5000);
        assert!(events.is_empty());
        assert_eq!(m.get_health(&w).unwrap().last_heartbeat_ms, 5000);
    }

    #[test]
    fn heartbeat_resets_missed_count() {
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        // Transition to suspect to increment missed_heartbeats.
        let _ = m.check(TIMEOUT_MS);
        assert!(m.get_health(&w).unwrap().missed_heartbeats > 0);
        // Heartbeat resets.
        let _ = m.heartbeat(&w, TIMEOUT_MS + 100);
        assert_eq!(m.get_health(&w).unwrap().missed_heartbeats, 0);
    }

    // ── Transition to Suspect ──────────────────────────────────────────

    #[test]
    fn transition_to_suspect() {
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        let events = m.check(TIMEOUT_MS); // exactly at timeout boundary
        assert!(matches!(events[0], FaultEvent::WorkerSuspect { .. }));
        assert!(matches!(m.get_health(&w).unwrap().state, HealthState::Suspect { .. }));
    }

    #[test]
    fn suspect_at_150_percent_timeout() {
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        let events = m.check(TIMEOUT_MS + 5000); // 15s of 10s timeout
        assert!(matches!(events[0], FaultEvent::WorkerSuspect { .. }));
    }

    // ── Transition to Dead ─────────────────────────────────────────────

    #[test]
    fn transition_to_dead_via_suspect() {
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        // First check → suspect.
        let _ = m.check(TIMEOUT_MS);
        // Second check → dead (now_ms = 2*timeout).
        let events = m.check(TIMEOUT_MS * 2);
        let dead_events: Vec<_> =
            events.iter().filter(|e| matches!(e, FaultEvent::WorkerDead { .. })).collect();
        assert_eq!(dead_events.len(), 1);
        assert_eq!(m.get_health(&w).unwrap().state, HealthState::Dead);
    }

    #[test]
    fn transition_healthy_to_dead_directly() {
        // If the clock jumps past both windows, go straight to Dead.
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        let events = m.check(TIMEOUT_MS * 2 + 1);
        assert!(matches!(events[0], FaultEvent::WorkerDead { .. }));
        assert_eq!(m.get_health(&w).unwrap().state, HealthState::Dead);
    }

    // ── Recovery from Suspect ──────────────────────────────────────────

    #[test]
    fn recovery_from_suspect() {
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        let _ = m.check(TIMEOUT_MS); // → suspect
        let events = m.heartbeat(&w, TIMEOUT_MS + 100);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], FaultEvent::WorkerRecovered { .. }));
        assert_eq!(m.get_health(&w).unwrap().state, HealthState::Healthy);
    }

    // ── Recovery from Dead ─────────────────────────────────────────────

    #[test]
    fn recovery_from_dead() {
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        let _ = m.check(TIMEOUT_MS * 2 + 1); // → dead
        let events = m.heartbeat(&w, TIMEOUT_MS * 3);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], FaultEvent::WorkerRecovered { .. }));
        assert_eq!(m.get_health(&w).unwrap().state, HealthState::Healthy);
    }

    // ── RecoveryNeeded event ───────────────────────────────────────────

    #[test]
    fn recovery_needed_majority_dead() {
        let mut m = test_monitor();
        m.register(wid("w1"), 0);
        m.register(wid("w2"), 0);
        // 2 workers, both dead → 100% dead → recovery needed.
        let events = m.check(TIMEOUT_MS * 2 + 1);
        let recovery: Vec<_> =
            events.iter().filter(|e| matches!(e, FaultEvent::RecoveryNeeded { .. })).collect();
        assert_eq!(recovery.len(), 1);
    }

    #[test]
    fn recovery_needed_half_dead() {
        let mut m = test_monitor();
        m.register(wid("w1"), 0);
        m.register(wid("w2"), 0);
        m.register(wid("w3"), 0);
        m.register(wid("w4"), 0);
        // Kill w1 and w2 by not heartbeat-ing them, keep w3 and w4 alive.
        m.heartbeat(&wid("w3"), TIMEOUT_MS * 3);
        m.heartbeat(&wid("w4"), TIMEOUT_MS * 3);
        // Check at 2x timeout: w1, w2 die. 2/4 = 50% → recovery needed.
        let events = m.check(TIMEOUT_MS * 3 + 1);
        let recovery: Vec<_> =
            events.iter().filter(|e| matches!(e, FaultEvent::RecoveryNeeded { .. })).collect();
        assert_eq!(recovery.len(), 1);
        if let FaultEvent::RecoveryNeeded { dead_workers } = &recovery[0] {
            assert_eq!(dead_workers.len(), 2);
        }
    }

    #[test]
    fn no_recovery_needed_minority_dead() {
        let mut m = test_monitor();
        m.register(wid("w1"), 0);
        m.register(wid("w2"), 0);
        m.register(wid("w3"), 0);
        // Keep w2, w3 alive.
        m.heartbeat(&wid("w2"), TIMEOUT_MS * 3);
        m.heartbeat(&wid("w3"), TIMEOUT_MS * 3);
        // w1 dies (1/3 = 33% < 50%).
        let events = m.check(TIMEOUT_MS * 3 + 1);
        let recovery: Vec<_> =
            events.iter().filter(|e| matches!(e, FaultEvent::RecoveryNeeded { .. })).collect();
        assert!(recovery.is_empty());
    }

    // ── Remove worker ──────────────────────────────────────────────────

    #[test]
    fn remove_worker() {
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        assert_eq!(m.total_count(), 1);
        m.remove(&w);
        assert_eq!(m.total_count(), 0);
        assert!(m.get_health(&w).is_none());
    }

    #[test]
    fn remove_nonexistent_worker_is_noop() {
        let mut m = test_monitor();
        m.remove(&wid("ghost"));
        assert_eq!(m.total_count(), 0);
    }

    // ── Counts ─────────────────────────────────────────────────────────

    #[test]
    fn healthy_count_tracking() {
        let mut m = test_monitor();
        m.register(wid("w1"), 0);
        m.register(wid("w2"), 0);
        m.register(wid("w3"), 0);
        assert_eq!(m.healthy_count(), 3);
        // Kill w1.
        let _ = m.check(TIMEOUT_MS * 2 + 1);
        assert_eq!(m.healthy_count(), 0); // All dead now.
    }

    #[test]
    fn total_count() {
        let mut m = test_monitor();
        assert_eq!(m.total_count(), 0);
        m.register(wid("w1"), 0);
        assert_eq!(m.total_count(), 1);
        m.register(wid("w2"), 0);
        assert_eq!(m.total_count(), 2);
        m.remove(&wid("w1"));
        assert_eq!(m.total_count(), 1);
    }

    // ── dead_workers list ──────────────────────────────────────────────

    #[test]
    fn dead_workers_list() {
        let mut m = test_monitor();
        m.register(wid("w1"), 0);
        m.register(wid("w2"), 0);
        m.register(wid("w3"), 0);
        let _ = m.check(TIMEOUT_MS * 2 + 1);
        let dead = m.dead_workers();
        assert_eq!(dead.len(), 3);
    }

    #[test]
    fn dead_workers_empty_when_all_healthy() {
        let mut m = test_monitor();
        m.register(wid("w1"), 0);
        m.register(wid("w2"), 0);
        let _ = m.check(100);
        assert!(m.dead_workers().is_empty());
    }

    // ── Missed heartbeats increment ────────────────────────────────────

    #[test]
    fn missed_heartbeats_increment_on_suspect() {
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        let _ = m.check(TIMEOUT_MS);
        assert_eq!(m.get_health(&w).unwrap().missed_heartbeats, 1);
        // Still suspect on next check.
        let _ = m.check(TIMEOUT_MS + 1000);
        assert!(m.get_health(&w).unwrap().missed_heartbeats >= 2);
    }

    #[test]
    fn missed_heartbeats_increment_on_dead() {
        let mut m = test_monitor();
        let w = wid("w1");
        m.register(w.clone(), 0);
        let _ = m.check(TIMEOUT_MS * 2 + 1); // → dead
        assert!(m.get_health(&w).unwrap().missed_heartbeats >= 1);
        let _ = m.check(TIMEOUT_MS * 3); // still dead
        assert!(m.get_health(&w).unwrap().missed_heartbeats >= 2);
    }

    // ── Multiple workers, mixed states ─────────────────────────────────

    #[test]
    fn mixed_states() {
        let mut m = test_monitor();
        m.register(wid("healthy"), 0);
        m.register(wid("suspect"), 0);
        m.register(wid("dead"), 0);

        // Advance to suspect window for "suspect", past dead for "dead".
        // But "healthy" gets a heartbeat.
        let now = TIMEOUT_MS; // 10s
        m.heartbeat(&wid("healthy"), now);

        let _events = m.check(now + 1);
        // "suspect" transitions to Suspect. "dead" stays suspect too (only 10s).
        // "healthy" stays Healthy.
        assert_eq!(m.healthy_count(), 1);

        let suspect_count =
            m.workers.values().filter(|h| matches!(h.state, HealthState::Suspect { .. })).count();
        assert_eq!(suspect_count, 2);
    }

    // ── Edge case: single worker dies ──────────────────────────────────

    #[test]
    fn single_worker_dies_triggers_recovery() {
        let mut m = test_monitor();
        m.register(wid("lone"), 0);
        let events = m.check(TIMEOUT_MS * 2 + 1);
        // 1/1 = 100% dead → recovery needed.
        let recovery: Vec<_> =
            events.iter().filter(|e| matches!(e, FaultEvent::RecoveryNeeded { .. })).collect();
        assert_eq!(recovery.len(), 1);
    }

    #[test]
    fn single_worker_healthy_no_recovery() {
        let mut m = test_monitor();
        m.register(wid("lone"), 0);
        let events = m.check(100);
        assert!(events.is_empty());
    }

    // ── All workers healthy ────────────────────────────────────────────

    #[test]
    fn all_workers_healthy_no_events() {
        let mut m = test_monitor();
        for i in 0..5 {
            m.register(wid(&format!("w{i}")), 0);
        }
        let events = m.check(1000);
        assert!(events.is_empty());
        assert_eq!(m.healthy_count(), 5);
    }

    // ── Serde roundtrip ────────────────────────────────────────────────

    #[test]
    fn health_state_serde_roundtrip() {
        for state in
            [HealthState::Healthy, HealthState::Suspect { last_seen_ms: 5000 }, HealthState::Dead]
        {
            let json = serde_json::to_string(&state).unwrap();
            let back: HealthState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, back);
        }
    }

    #[test]
    fn worker_health_serde_roundtrip() {
        let wh = WorkerHealth {
            worker_id: wid("w1"),
            last_heartbeat_ms: 1000,
            state: HealthState::Suspect { last_seen_ms: 5000 },
            missed_heartbeats: 3,
        };
        let json = serde_json::to_string(&wh).unwrap();
        let back: WorkerHealth = serde_json::from_str(&json).unwrap();
        assert_eq!(wh.worker_id, back.worker_id);
        assert_eq!(wh.last_heartbeat_ms, back.last_heartbeat_ms);
        assert_eq!(wh.state, back.state);
        assert_eq!(wh.missed_heartbeats, back.missed_heartbeats);
    }

    #[test]
    fn fault_event_serde_roundtrip() {
        let events = vec![
            FaultEvent::WorkerSuspect { worker_id: wid("w1") },
            FaultEvent::WorkerDead { worker_id: wid("w2") },
            FaultEvent::WorkerRecovered { worker_id: wid("w3") },
            FaultEvent::RecoveryNeeded { dead_workers: vec![wid("w1"), wid("w2")] },
        ];
        let json = serde_json::to_string(&events).unwrap();
        let back: Vec<FaultEvent> = serde_json::from_str(&json).unwrap();
        assert_eq!(events, back);
    }

    // ── ts-rs export ───────────────────────────────────────────────────

    #[test]
    fn ts_export_health_state() {
        use ts_rs::Config;
        let name = HealthState::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_worker_health() {
        use ts_rs::Config;
        let name = WorkerHealth::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_fault_event() {
        use ts_rs::Config;
        let name = FaultEvent::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_health_monitor() {
        use ts_rs::Config;
        let name = HealthMonitor::name(&Config::new());
        assert!(!name.is_empty());
    }

    // ── HealthState snake_case serde ───────────────────────────────────

    #[test]
    fn health_state_snake_case() {
        let json = serde_json::to_string(&HealthState::Healthy).unwrap();
        assert!(json.contains("healthy"), "got: {json}");

        let suspect = HealthState::Suspect { last_seen_ms: 42 };
        let json = serde_json::to_string(&suspect).unwrap();
        assert!(json.contains("suspect"), "got: {json}");

        let json = serde_json::to_string(&HealthState::Dead).unwrap();
        assert!(json.contains("dead"), "got: {json}");
    }

    // ── Heartbeat for unregistered worker is no-op ─────────────────────

    #[test]
    fn heartbeat_unregistered_worker_noop() {
        let mut m = test_monitor();
        let events = m.heartbeat(&wid("ghost"), 1000);
        assert!(events.is_empty());
    }

    // ── Check with no workers ──────────────────────────────────────────

    #[test]
    fn check_empty_monitor() {
        let mut m = test_monitor();
        let events = m.check(99999);
        assert!(events.is_empty());
    }

    // ── HealthMonitor serde roundtrip ──────────────────────────────────

    #[test]
    fn health_monitor_serde_roundtrip() {
        let mut m = test_monitor();
        m.register(wid("w1"), 1000);
        m.register(wid("w2"), 2000);
        let _ = m.check(15000); // transition states

        let json = serde_json::to_string(&m).unwrap();
        let back: HealthMonitor = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total_count(), 2);
        assert_eq!(back.timeout_secs, 10);
    }
}
