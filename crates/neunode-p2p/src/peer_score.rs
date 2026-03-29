use std::collections::HashMap;
use std::time::Instant;

use libp2p::PeerId;
use neunode_core::constants::p2p;
use serde::{Deserialize, Serialize};

const DEFAULT_P1_WEIGHT: f64 = 1.0;
const DEFAULT_P2_WEIGHT: f64 = 1.0;
const DEFAULT_P3_WEIGHT: f64 = 0.5;
const DEFAULT_P4_WEIGHT: f64 = -10.0;
const DEFAULT_P7_WEIGHT: f64 = -5.0;

const DEFAULT_P1_CAP: f64 = 10.0;
const DEFAULT_P2_CAP: f64 = 10.0;
const DEFAULT_P3_CAP: f64 = 10.0;
const DEFAULT_P4_CAP: f64 = 0.0;
const DEFAULT_P5_CAP: f64 = 10.0;
const DEFAULT_P6_CAP: f64 = 10.0;
const DEFAULT_P7_CAP: f64 = 0.0;

const GRAYLIST_THRESHOLD: f64 = -100.0;
const PUBLISH_THRESHOLD: f64 = -1000.0;
const GOSSIP_THRESHOLD: f64 = -500.0;

const DECAY_INTERVAL_SECS: f64 = 1.0;
const DECAY_TO_ZERO: f64 = 0.01;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PeerScoreParams {
    pub p1_weight: f64,
    pub p2_weight: f64,
    pub p3_weight: f64,
    pub p4_weight: f64,
    pub p5_weight: f64,
    pub p6_weight: f64,
    pub p7_weight: f64,
    pub p1_cap: f64,
    pub p2_cap: f64,
    pub p3_cap: f64,
    pub p4_cap: f64,
    pub p5_cap: f64,
    pub p6_cap: f64,
    pub p7_cap: f64,
    pub graylist_threshold: f64,
    pub publish_threshold: f64,
    pub gossip_threshold: f64,
    pub decay_interval_secs: f64,
    pub decay_to_zero: f64,
    pub retain_score_secs: f64,
}

impl Default for PeerScoreParams {
    fn default() -> Self {
        Self {
            p1_weight: DEFAULT_P1_WEIGHT,
            p2_weight: DEFAULT_P2_WEIGHT,
            p3_weight: DEFAULT_P3_WEIGHT,
            p4_weight: DEFAULT_P4_WEIGHT,
            p5_weight: p2p::PEER_SCORE_P5_WEIGHT,
            p6_weight: p2p::PEER_SCORE_P6_WEIGHT,
            p7_weight: DEFAULT_P7_WEIGHT,
            p1_cap: DEFAULT_P1_CAP,
            p2_cap: DEFAULT_P2_CAP,
            p3_cap: DEFAULT_P3_CAP,
            p4_cap: DEFAULT_P4_CAP,
            p5_cap: DEFAULT_P5_CAP,
            p6_cap: DEFAULT_P6_CAP,
            p7_cap: DEFAULT_P7_CAP,
            graylist_threshold: GRAYLIST_THRESHOLD,
            publish_threshold: PUBLISH_THRESHOLD,
            gossip_threshold: GOSSIP_THRESHOLD,
            decay_interval_secs: DECAY_INTERVAL_SECS,
            decay_to_zero: DECAY_TO_ZERO,
            retain_score_secs: 3600.0,
        }
    }
}

impl PeerScoreParams {
    pub fn threshold_score(&self, kind: ThresholdKind) -> f64 {
        match kind {
            ThresholdKind::Graylist => self.graylist_threshold,
            ThresholdKind::Publish => self.publish_threshold,
            ThresholdKind::Gossip => self.gossip_threshold,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdKind {
    Graylist,
    Publish,
    Gossip,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoreEvent {
    MessageDelivered,
    MessageInvalid { penalty: f64 },
    PeerConnected,
    PeerDisconnected,
    AppScoreUpdate { delta: f64 },
    BehavioralPenalty { penalty: f64 },
    TimeInMeshTick,
}

#[derive(Debug, Clone)]
struct PeerState {
    p1_time_in_mesh: f64,
    p2_first_message_deliveries: f64,
    p3_mesh_message_deliveries: f64,
    p4_invalid_messages: f64,
    p5_app_score: f64,
    p6_ip_colocation: f64,
    p7_behavioral_penalty: f64,
    last_update: Instant,
    connected: bool,
}

impl PeerState {
    fn new() -> Self {
        Self {
            p1_time_in_mesh: 0.0,
            p2_first_message_deliveries: 0.0,
            p3_mesh_message_deliveries: 0.0,
            p4_invalid_messages: 0.0,
            p5_app_score: 0.0,
            p6_ip_colocation: 0.0,
            p7_behavioral_penalty: 0.0,
            last_update: Instant::now(),
            connected: false,
        }
    }

    fn compute_score(&self, params: &PeerScoreParams) -> f64 {
        let p1 = self.p1_time_in_mesh.min(params.p1_cap) * params.p1_weight;
        let p2 = self.p2_first_message_deliveries.min(params.p2_cap) * params.p2_weight;
        let p3 = self.p3_mesh_message_deliveries.min(params.p3_cap) * params.p3_weight;
        let p4 = self.p4_invalid_messages * params.p4_weight;
        let p5 = self.p5_app_score.min(params.p5_cap) * params.p5_weight;
        let p6 = self.p6_ip_colocation * params.p6_weight;
        let p7 = self.p7_behavioral_penalty * params.p7_weight;
        p1 + p2 + p3 + p4 + p5 + p6 + p7
    }

    fn apply_decay(&mut self, params: &PeerScoreParams) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        if elapsed < params.decay_interval_secs {
            return;
        }
        let decay_factor = params.decay_to_zero.powf(elapsed / params.decay_interval_secs);
        self.p1_time_in_mesh *= decay_factor;
        self.p2_first_message_deliveries *= decay_factor;
        self.p3_mesh_message_deliveries *= decay_factor;
        self.p4_invalid_messages *= decay_factor;
        self.p5_app_score *= decay_factor;
        self.p6_ip_colocation *= decay_factor;
        self.p7_behavioral_penalty *= decay_factor;
        self.last_update = now;
    }
}

#[derive(Debug)]
pub struct PeerScore {
    params: PeerScoreParams,
    peers: HashMap<String, PeerState>,
}

impl PeerScore {
    pub fn new(params: PeerScoreParams) -> Self {
        Self { params, peers: HashMap::new() }
    }

    pub fn update_score(&mut self, peer_id: &PeerId, event: ScoreEvent) -> f64 {
        let key = peer_id.to_string();
        let state = self.peers.entry(key).or_insert_with(PeerState::new);
        state.apply_decay(&self.params);

        match event {
            ScoreEvent::MessageDelivered => {
                state.p2_first_message_deliveries += 1.0;
                state.p3_mesh_message_deliveries += 1.0;
            }
            ScoreEvent::MessageInvalid { penalty } => {
                state.p4_invalid_messages += penalty;
            }
            ScoreEvent::PeerConnected => {
                state.connected = true;
                state.p1_time_in_mesh += 1.0;
            }
            ScoreEvent::PeerDisconnected => {
                state.connected = false;
            }
            ScoreEvent::AppScoreUpdate { delta } => {
                state.p5_app_score += delta;
            }
            ScoreEvent::BehavioralPenalty { penalty } => {
                state.p7_behavioral_penalty += penalty;
            }
            ScoreEvent::TimeInMeshTick => {
                if state.connected {
                    state.p1_time_in_mesh += 1.0;
                }
            }
        }

        state.compute_score(&self.params)
    }

    pub fn get_score(&mut self, peer_id: &PeerId) -> f64 {
        let key = peer_id.to_string();
        if let Some(state) = self.peers.get_mut(&key) {
            state.apply_decay(&self.params);
            state.compute_score(&self.params)
        } else {
            0.0
        }
    }

    pub fn is_graylisted(&mut self, peer_id: &PeerId) -> bool {
        self.get_score(peer_id) < self.params.graylist_threshold
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peers.remove(&peer_id.to_string());
    }

    pub fn params(&self) -> &PeerScoreParams {
        &self.params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_peer_id() -> PeerId {
        libp2p::identity::Keypair::generate_ed25519().public().to_peer_id()
    }

    #[test]
    fn default_params_sanity() {
        let params = PeerScoreParams::default();
        assert!(params.p1_weight > 0.0);
        assert!(params.p2_weight > 0.0);
        assert!(params.p3_weight > 0.0);
        assert!(params.p4_weight < 0.0);
        assert!(params.p5_weight > 0.0);
        assert!(params.p6_weight > 0.0);
        assert!(params.p7_weight < 0.0);
        assert!(params.p1_cap > 0.0);
        assert!(params.graylist_threshold < 0.0);
    }

    #[test]
    fn initial_score_is_zero() {
        let mut scorer = PeerScore::new(PeerScoreParams::default());
        let peer = random_peer_id();
        let score = scorer.get_score(&peer);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn message_delivered_increases_score() {
        let mut scorer = PeerScore::new(PeerScoreParams::default());
        let peer = random_peer_id();
        let before = scorer.get_score(&peer);
        let after = scorer.update_score(&peer, ScoreEvent::MessageDelivered);
        assert!(after > before);
    }

    #[test]
    fn message_invalid_decreases_score() {
        let mut scorer = PeerScore::new(PeerScoreParams::default());
        let peer = random_peer_id();
        scorer.update_score(&peer, ScoreEvent::PeerConnected);
        let before = scorer.get_score(&peer);
        let after = scorer.update_score(&peer, ScoreEvent::MessageInvalid { penalty: 5.0 });
        assert!(after < before);
    }

    #[test]
    fn peer_connected_increases_score() {
        let mut scorer = PeerScore::new(PeerScoreParams::default());
        let peer = random_peer_id();
        let before = scorer.get_score(&peer);
        let after = scorer.update_score(&peer, ScoreEvent::PeerConnected);
        assert!(after > before);
    }

    #[test]
    fn peer_disconnected_stops_mesh_time_growth() {
        let mut scorer = PeerScore::new(PeerScoreParams::default());
        let peer = random_peer_id();
        scorer.update_score(&peer, ScoreEvent::PeerConnected);
        scorer.update_score(&peer, ScoreEvent::TimeInMeshTick);
        scorer.update_score(&peer, ScoreEvent::TimeInMeshTick);
        let connected_mesh_score = scorer.get_score(&peer);
        scorer.update_score(&peer, ScoreEvent::PeerDisconnected);
        let score_after_disconnect = scorer.get_score(&peer);
        let score_after_idle_tick = scorer.update_score(&peer, ScoreEvent::TimeInMeshTick);
        assert!(score_after_idle_tick <= score_after_disconnect);
        assert!(connected_mesh_score > 0.0);
    }

    #[test]
    fn app_score_update() {
        let mut scorer = PeerScore::new(PeerScoreParams::default());
        let peer = random_peer_id();
        let before = scorer.get_score(&peer);
        let after = scorer.update_score(&peer, ScoreEvent::AppScoreUpdate { delta: 5.0 });
        assert!(after > before);
    }

    #[test]
    fn behavioral_penalty_decreases_score() {
        let mut scorer = PeerScore::new(PeerScoreParams::default());
        let peer = random_peer_id();
        scorer.update_score(&peer, ScoreEvent::PeerConnected);
        let before = scorer.get_score(&peer);
        let after = scorer.update_score(&peer, ScoreEvent::BehavioralPenalty { penalty: 3.0 });
        assert!(after < before);
    }

    #[test]
    fn graylist_threshold_works() {
        let params = PeerScoreParams::default();
        let mut scorer = PeerScore::new(params);
        let peer = random_peer_id();
        assert!(!scorer.is_graylisted(&peer));
        for _ in 0..100 {
            scorer.update_score(&peer, ScoreEvent::MessageInvalid { penalty: 10.0 });
        }
        assert!(scorer.is_graylisted(&peer));
    }

    #[test]
    fn peer_count_tracks_entries() {
        let mut scorer = PeerScore::new(PeerScoreParams::default());
        assert_eq!(scorer.peer_count(), 0);
        let p1 = random_peer_id();
        let p2 = random_peer_id();
        scorer.update_score(&p1, ScoreEvent::PeerConnected);
        assert_eq!(scorer.peer_count(), 1);
        scorer.update_score(&p2, ScoreEvent::PeerConnected);
        assert_eq!(scorer.peer_count(), 2);
    }

    #[test]
    fn remove_peer() {
        let mut scorer = PeerScore::new(PeerScoreParams::default());
        let peer = random_peer_id();
        scorer.update_score(&peer, ScoreEvent::PeerConnected);
        assert_eq!(scorer.peer_count(), 1);
        scorer.remove_peer(&peer);
        assert_eq!(scorer.peer_count(), 0);
        assert_eq!(scorer.get_score(&peer), 0.0);
    }

    #[test]
    fn score_capped_by_p1_cap() {
        let params = PeerScoreParams { p1_cap: 5.0, p1_weight: 10.0, ..Default::default() };
        let mut scorer = PeerScore::new(params);
        let peer = random_peer_id();
        for _ in 0..100 {
            scorer.update_score(&peer, ScoreEvent::TimeInMeshTick);
        }
        let score = scorer.get_score(&peer);
        let max_from_p1 = 5.0 * 10.0;
        assert!(score <= max_from_p1 + 0.001);
    }

    #[test]
    fn threshold_score_kinds() {
        let params = PeerScoreParams::default();
        assert!(params.threshold_score(ThresholdKind::Graylist) < 0.0);
        assert!(params.threshold_score(ThresholdKind::Publish) < 0.0);
        assert!(params.threshold_score(ThresholdKind::Gossip) < 0.0);
    }

    #[test]
    fn multiple_peers_independent() {
        let mut scorer = PeerScore::new(PeerScoreParams::default());
        let p1 = random_peer_id();
        let p2 = random_peer_id();
        scorer.update_score(&p1, ScoreEvent::PeerConnected);
        scorer.update_score(&p2, ScoreEvent::MessageInvalid { penalty: 10.0 });
        let s1 = scorer.get_score(&p1);
        let s2 = scorer.get_score(&p2);
        assert!(s1 > s2);
    }

    #[test]
    fn peer_score_params_serde_roundtrip() {
        let params = PeerScoreParams::default();
        let json = serde_json::to_string(&params).unwrap();
        let back: PeerScoreParams = serde_json::from_str(&json).unwrap();
        assert_eq!(params, back);
    }

    #[test]
    fn time_in_mesh_only_when_connected() {
        let mut scorer = PeerScore::new(PeerScoreParams::default());
        let peer = random_peer_id();
        let score_disconnected = scorer.update_score(&peer, ScoreEvent::TimeInMeshTick);
        scorer.update_score(&peer, ScoreEvent::PeerConnected);
        let score_connected = scorer.update_score(&peer, ScoreEvent::TimeInMeshTick);
        assert!(score_connected > score_disconnected);
    }
}
