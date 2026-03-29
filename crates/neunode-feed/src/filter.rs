use neunode_core::kind::Kind;
use neunode_core::types::{Did, Timestamp};

use crate::event::FeedEvent;

#[derive(Debug, Clone, Default)]
pub struct FeedFilter {
    pub kinds: Option<Vec<Kind>>,
    pub authors: Option<Vec<Did>>,
    pub since: Option<Timestamp>,
    pub until: Option<Timestamp>,
    pub limit: Option<usize>,
    pub tags: Option<Vec<(String, String)>>,
}

impl FeedFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn kinds(mut self, kinds: Vec<Kind>) -> Self {
        self.kinds = Some(kinds);
        self
    }

    pub fn authors(mut self, authors: Vec<Did>) -> Self {
        self.authors = Some(authors);
        self
    }

    pub fn since(mut self, ts: Timestamp) -> Self {
        self.since = Some(ts);
        self
    }

    pub fn until(mut self, ts: Timestamp) -> Self {
        self.until = Some(ts);
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn tags(mut self, tags: Vec<(String, String)>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn matches(&self, event: &FeedEvent) -> bool {
        if let Some(kinds) = &self.kinds {
            if !kinds.contains(&event.kind) {
                return false;
            }
        }

        if let Some(authors) = &self.authors {
            if !authors.contains(&event.author) {
                return false;
            }
        }

        if let Some(since) = self.since {
            if event.timestamp < since {
                return false;
            }
        }

        if let Some(until) = self.until {
            if event.timestamp > until {
                return false;
            }
        }

        if let Some(tags) = &self.tags {
            for (key, value) in tags {
                let found = event.tags.iter().any(|t| t.key == *key && t.value == *value);
                if !found {
                    return false;
                }
            }
        }

        true
    }
}

pub fn apply_filter<'a>(filter: &FeedFilter, events: &'a [FeedEvent]) -> Vec<&'a FeedEvent> {
    let mut matched: Vec<&'a FeedEvent> = events.iter().filter(|e| filter.matches(e)).collect();

    if let Some(limit) = filter.limit {
        matched.truncate(limit);
    }

    matched
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventRef, EventTag};
    use neunode_core::types::{EventId, Hash256};

    fn test_did(name: &str) -> Did {
        Did(format!("did:neunode:{}", name))
    }

    fn make_event(kind: Kind, author: &str, ts: Timestamp, tags: Vec<(&str, &str)>) -> FeedEvent {
        FeedEvent {
            id: EventId(format!("evt_{}_{}", kind.as_u16(), ts)),
            kind,
            author: test_did(author),
            sequence: 0,
            timestamp: ts,
            prev_hash: Hash256("0".to_string()),
            content: format!("event at {}", ts),
            tags: tags
                .into_iter()
                .map(|(k, v)| EventTag { key: k.to_string(), value: v.to_string() })
                .collect(),
            refs: vec![EventRef {
                event_id: EventId("ref_0".to_string()),
                author: test_did("ref_author"),
            }],
            signature: None,
        }
    }

    fn sample_events() -> Vec<FeedEvent> {
        vec![
            make_event(Kind::BountyPost, "alice", 100, vec![("env", "prod")]),
            make_event(Kind::BountyClaim, "bob", 200, vec![("env", "test")]),
            make_event(Kind::Attest, "alice", 300, vec![("env", "prod")]),
            make_event(Kind::BountyPost, "charlie", 400, vec![]),
            make_event(Kind::Attest, "bob", 500, vec![("env", "prod"), ("priority", "high")]),
        ]
    }

    #[test]
    fn filter_by_kind() {
        let filter = FeedFilter::new().kinds(vec![Kind::BountyPost]);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 2);
        assert!(matched.iter().all(|e| e.kind == Kind::BountyPost));
    }

    #[test]
    fn filter_by_multiple_kinds() {
        let filter = FeedFilter::new().kinds(vec![Kind::BountyPost, Kind::BountyClaim]);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 3);
    }

    #[test]
    fn filter_by_author() {
        let filter = FeedFilter::new().authors(vec![test_did("alice")]);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 2);
        assert!(matched.iter().all(|e| e.author == test_did("alice")));
    }

    #[test]
    fn filter_by_multiple_authors() {
        let filter = FeedFilter::new().authors(vec![test_did("alice"), test_did("bob")]);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 4);
    }

    #[test]
    fn filter_by_time_range() {
        let filter = FeedFilter::new().since(150).until(450);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 3);
        assert!(matched.iter().all(|e| e.timestamp >= 150 && e.timestamp <= 450));
    }

    #[test]
    fn filter_since_only() {
        let filter = FeedFilter::new().since(300);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 3);
    }

    #[test]
    fn filter_until_only() {
        let filter = FeedFilter::new().until(200);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn filter_by_tags() {
        let filter = FeedFilter::new().tags(vec![("env".to_string(), "prod".to_string())]);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 3);
    }

    #[test]
    fn filter_by_multiple_tags() {
        let filter = FeedFilter::new().tags(vec![
            ("env".to_string(), "prod".to_string()),
            ("priority".to_string(), "high".to_string()),
        ]);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].timestamp, 500);
    }

    #[test]
    fn filter_tag_not_found() {
        let filter = FeedFilter::new().tags(vec![("nonexistent".to_string(), "value".to_string())]);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert!(matched.is_empty());
    }

    #[test]
    fn filter_with_limit() {
        let filter = FeedFilter::new().limit(2);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn filter_limit_larger_than_results() {
        let filter = FeedFilter::new().kinds(vec![Kind::BountyPost]).limit(10);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn filter_no_criteria_matches_all() {
        let filter = FeedFilter::new();
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 5);
    }

    #[test]
    fn filter_combination() {
        let filter =
            FeedFilter::new().kinds(vec![Kind::Attest]).authors(vec![test_did("bob")]).since(400);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].timestamp, 500);
    }

    #[test]
    fn filter_combination_no_results() {
        let filter = FeedFilter::new().kinds(vec![Kind::BountyPost]).authors(vec![test_did("bob")]);
        let events = sample_events();
        let matched = apply_filter(&filter, &events);
        assert!(matched.is_empty());
    }

    #[test]
    fn matches_method_directly() {
        let event = make_event(Kind::BountyPost, "alice", 100, vec![]);
        let filter = FeedFilter::new().kinds(vec![Kind::BountyPost]);
        assert!(filter.matches(&event));

        let filter2 = FeedFilter::new().kinds(vec![Kind::Attest]);
        assert!(!filter2.matches(&event));
    }

    #[test]
    fn filter_empty_events() {
        let filter = FeedFilter::new().kinds(vec![Kind::BountyPost]);
        let events: Vec<FeedEvent> = vec![];
        let matched = apply_filter(&filter, &events);
        assert!(matched.is_empty());
    }

    #[test]
    fn builder_pattern_chaining() {
        let filter = FeedFilter::new()
            .kinds(vec![Kind::BountyPost])
            .authors(vec![test_did("alice")])
            .since(0)
            .until(1000)
            .limit(10)
            .tags(vec![("env".to_string(), "prod".to_string())]);

        assert!(filter.kinds.is_some());
        assert!(filter.authors.is_some());
        assert!(filter.since.is_some());
        assert!(filter.until.is_some());
        assert!(filter.limit.is_some());
        assert!(filter.tags.is_some());
    }
}
