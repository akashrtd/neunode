use std::collections::{HashMap, HashSet};

use libp2p::PeerId;

use crate::error::Result;

/// Verifies that a peer's libp2p PeerId corresponds to their claimed DID.
///
/// During the identify handshake, peers exchange their public keys. This module
/// maps PeerIds to verified DIDs and tracks which peers have completed
/// authentication. Unauthenticated peers are flagged for downstream filtering.
pub struct PeerAuth {
    /// Mapping from PeerId (string form) to verified DID.
    verified: HashMap<String, String>,
    /// Peers that have completed DID verification.
    authenticated: HashSet<String>,
}

impl PeerAuth {
    pub fn new() -> Self {
        Self { verified: HashMap::new(), authenticated: HashSet::new() }
    }
}

impl Default for PeerAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerAuth {

    /// Verify a peer by checking that their claimed DID matches the expected
    /// identity derived from their PeerId.
    ///
    /// In the neunode protocol, DIDs are of the form `did:neunode:<peer_id>`.
    /// A peer proves identity ownership by connecting with the libp2p keypair
    /// that matches the PeerId component of their DID.
    pub fn verify(&mut self, peer_id: &PeerId, claimed_did: &str) -> Result<bool> {
        let pid_str = peer_id.to_string();
        let expected_suffix = &pid_str;

        // DID format: did:neunode:<peer_id>
        if let Some(suffix) = claimed_did.strip_prefix("did:neunode:") {
            if suffix == expected_suffix {
                self.verified.insert(pid_str.clone(), claimed_did.to_string());
                self.authenticated.insert(pid_str);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Register a peer as authenticated with an explicit DID mapping.
    /// Used when DID format is non-standard or externally verified.
    pub fn register(&mut self, peer_id: &PeerId, did: &str) {
        let pid_str = peer_id.to_string();
        self.verified.insert(pid_str.clone(), did.to_string());
        self.authenticated.insert(pid_str);
    }

    /// Remove a peer's authentication state (e.g., on disconnect).
    pub fn remove(&mut self, peer_id: &PeerId) {
        let pid_str = peer_id.to_string();
        self.verified.remove(&pid_str);
        self.authenticated.remove(&pid_str);
    }

    /// Check if a peer has completed DID authentication.
    pub fn is_authenticated(&self, peer_id: &PeerId) -> bool {
        self.authenticated.contains(&peer_id.to_string())
    }

    /// Get the verified DID for a peer, if any.
    pub fn get_did(&self, peer_id: &PeerId) -> Option<&str> {
        self.verified.get(&peer_id.to_string()).map(|s| s.as_str())
    }

    /// Number of currently authenticated peers.
    pub fn authenticated_count(&self) -> usize {
        self.authenticated.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_peer_id() -> PeerId {
        libp2p::identity::Keypair::generate_ed25519().public().to_peer_id()
    }

    #[test]
    fn verify_matching_did() {
        let mut auth = PeerAuth::new();
        let peer_id = random_peer_id();
        let did = format!("did:neunode:{}", peer_id);
        assert!(auth.verify(&peer_id, &did).unwrap());
        assert!(auth.is_authenticated(&peer_id));
        assert_eq!(auth.get_did(&peer_id), Some(did.as_str()));
    }

    #[test]
    fn verify_mismatched_did() {
        let mut auth = PeerAuth::new();
        let peer_id = random_peer_id();
        let other_peer_id = random_peer_id();
        let wrong_did = format!("did:neunode:{}", other_peer_id);
        assert!(!auth.verify(&peer_id, &wrong_did).unwrap());
        assert!(!auth.is_authenticated(&peer_id));
    }

    #[test]
    fn verify_invalid_did_format() {
        let mut auth = PeerAuth::new();
        let peer_id = random_peer_id();
        assert!(!auth.verify(&peer_id, "did:other:abc123").unwrap());
        assert!(!auth.is_authenticated(&peer_id));
    }

    #[test]
    fn register_explicit() {
        let mut auth = PeerAuth::new();
        let peer_id = random_peer_id();
        auth.register(&peer_id, "did:neunode:custom");
        assert!(auth.is_authenticated(&peer_id));
        assert_eq!(auth.get_did(&peer_id), Some("did:neunode:custom"));
    }

    #[test]
    fn remove_peer() {
        let mut auth = PeerAuth::new();
        let peer_id = random_peer_id();
        let did = format!("did:neunode:{}", peer_id);
        auth.verify(&peer_id, &did).unwrap();
        assert!(auth.is_authenticated(&peer_id));
        auth.remove(&peer_id);
        assert!(!auth.is_authenticated(&peer_id));
        assert!(auth.get_did(&peer_id).is_none());
    }

    #[test]
    fn authenticated_count() {
        let mut auth = PeerAuth::new();
        assert_eq!(auth.authenticated_count(), 0);
        let p1 = random_peer_id();
        let p2 = random_peer_id();
        auth.verify(&p1, &format!("did:neunode:{}", p1)).unwrap();
        auth.verify(&p2, &format!("did:neunode:{}", p2)).unwrap();
        assert_eq!(auth.authenticated_count(), 2);
        auth.remove(&p1);
        assert_eq!(auth.authenticated_count(), 1);
    }

    #[test]
    fn verify_idempotent() {
        let mut auth = PeerAuth::new();
        let peer_id = random_peer_id();
        let did = format!("did:neunode:{}", peer_id);
        auth.verify(&peer_id, &did).unwrap();
        auth.verify(&peer_id, &did).unwrap();
        assert_eq!(auth.authenticated_count(), 1);
    }
}
