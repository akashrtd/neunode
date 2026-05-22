use std::collections::HashMap;

/// Per-DID sliding window rate limiter for feed events.
///
/// Tracks how many events each DID has published within a time window
/// and rejects events that exceed the configured limit.
pub struct RateLimiter {
    max_events: usize,
    window_secs: u64,
    buckets: HashMap<String, Vec<u64>>,
}

impl RateLimiter {
    pub fn new(max_events: usize, window_secs: u64) -> Self {
        Self { max_events, window_secs, buckets: HashMap::new() }
    }

    /// Check if a DID is allowed to publish. Records the attempt and returns true if allowed.
    pub fn allow(&mut self, did: &str, now_secs: u64) -> bool {
        let window_start = now_secs.saturating_sub(self.window_secs);
        let timestamps = self.buckets.entry(did.to_string()).or_default();

        // Remove expired entries
        timestamps.retain(|&t| t > window_start);

        if timestamps.len() >= self.max_events {
            return false;
        }

        timestamps.push(now_secs);
        true
    }

    /// Check if a DID would be allowed without recording the attempt.
    pub fn would_allow(&mut self, did: &str, now_secs: u64) -> bool {
        let window_start = now_secs.saturating_sub(self.window_secs);
        let timestamps = self.buckets.entry(did.to_string()).or_default();
        timestamps.retain(|&t| t > window_start);
        timestamps.len() < self.max_events
    }

    pub fn max_events(&self) -> usize {
        self.max_events
    }

    pub fn window_secs(&self) -> u64 {
        self.window_secs
    }

    /// Remove tracking data for DIDs with no recent activity.
    pub fn prune(&mut self, now_secs: u64) {
        let window_start = now_secs.saturating_sub(self.window_secs);
        self.buckets.retain(|_, timestamps| {
            timestamps.retain(|&t| t > window_start);
            !timestamps.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_within_limit() {
        let mut limiter = RateLimiter::new(3, 60);
        assert!(limiter.allow("did:neunode:alice", 100));
        assert!(limiter.allow("did:neunode:alice", 101));
        assert!(limiter.allow("did:neunode:alice", 102));
    }

    #[test]
    fn reject_over_limit() {
        let mut limiter = RateLimiter::new(2, 60);
        assert!(limiter.allow("did:neunode:alice", 100));
        assert!(limiter.allow("did:neunode:alice", 101));
        assert!(!limiter.allow("did:neunode:alice", 102));
    }

    #[test]
    fn different_dids_independent() {
        let mut limiter = RateLimiter::new(1, 60);
        assert!(limiter.allow("did:neunode:alice", 100));
        assert!(limiter.allow("did:neunode:bob", 100));
        assert!(!limiter.allow("did:neunode:alice", 101));
        assert!(!limiter.allow("did:neunode:bob", 101));
    }

    #[test]
    fn window_expiry_allows_again() {
        let mut limiter = RateLimiter::new(2, 60);
        assert!(limiter.allow("did:neunode:alice", 100));
        assert!(limiter.allow("did:neunode:alice", 110));
        // Over limit
        assert!(!limiter.allow("did:neunode:alice", 115));
        // Window expires (100 and 110 are now outside window 170-60=110)
        assert!(limiter.allow("did:neunode:alice", 170));
    }

    #[test]
    fn would_allow_no_side_effect() {
        let mut limiter = RateLimiter::new(1, 60);
        assert!(limiter.would_allow("did:neunode:alice", 100));
        assert!(limiter.allow("did:neunode:alice", 100));
        assert!(!limiter.would_allow("did:neunode:alice", 101));
    }

    #[test]
    fn prune_removes_stale_entries() {
        let mut limiter = RateLimiter::new(5, 60);
        limiter.allow("did:neunode:alice", 100);
        limiter.allow("did:neunode:bob", 200);
        assert_eq!(limiter.buckets.len(), 2);

        limiter.prune(300);
        // Alice's entry at t=100 is outside window [240, 300], pruned
        // Bob's entry at t=200 is outside window [240, 300], pruned
        assert!(limiter.buckets.is_empty());
    }

    #[test]
    fn accessors() {
        let limiter = RateLimiter::new(10, 120);
        assert_eq!(limiter.max_events(), 10);
        assert_eq!(limiter.window_secs(), 120);
    }
}
