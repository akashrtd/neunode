use neunode_core::kind::{Kind, KindCategory};

pub fn topic_for_kind(kind: &Kind) -> &'static str {
    kind.gossipsub_topic()
}

pub fn all_topics() -> Vec<&'static str> {
    vec![
        "neunode/system",
        "neunode/bounty",
        "neunode/training",
        "neunode/attestation",
        "neunode/inference",
        "neunode/governance",
        "neunode/custom",
    ]
}

pub fn topic_for_category(cat: KindCategory) -> &'static str {
    match cat {
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

pub fn parse_topic(topic: &str) -> Option<KindCategory> {
    match topic {
        "neunode/system" => Some(KindCategory::System),
        "neunode/bounty" => Some(KindCategory::Bounty),
        "neunode/training" => Some(KindCategory::Training),
        "neunode/attestation" => Some(KindCategory::Attestation),
        "neunode/inference" => Some(KindCategory::Inference),
        "neunode/governance" => Some(KindCategory::Governance),
        "neunode/custom" => Some(KindCategory::Custom),
        _ => None,
    }
}

pub fn is_valid_topic(topic: &str) -> bool {
    parse_topic(topic).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_for_kind_system() {
        assert_eq!(topic_for_kind(&Kind::AgentMetadata), "neunode/system");
        assert_eq!(topic_for_kind(&Kind::CapabilityUpdate), "neunode/system");
        assert_eq!(topic_for_kind(&Kind::Lifecycle), "neunode/system");
    }

    #[test]
    fn topic_for_kind_bounty() {
        assert_eq!(topic_for_kind(&Kind::BountyPost), "neunode/bounty");
        assert_eq!(topic_for_kind(&Kind::BountyClaim), "neunode/bounty");
        assert_eq!(topic_for_kind(&Kind::EscrowDeposit), "neunode/bounty");
    }

    #[test]
    fn topic_for_kind_training() {
        assert_eq!(topic_for_kind(&Kind::JobSubmit), "neunode/training");
        assert_eq!(topic_for_kind(&Kind::Checkpoint), "neunode/training");
        assert_eq!(topic_for_kind(&Kind::GradientUpdate), "neunode/training");
    }

    #[test]
    fn topic_for_kind_attestation() {
        assert_eq!(topic_for_kind(&Kind::Attest), "neunode/attestation");
        assert_eq!(topic_for_kind(&Kind::CounterAttest), "neunode/attestation");
        assert_eq!(topic_for_kind(&Kind::VerificationResult), "neunode/attestation");
    }

    #[test]
    fn topic_for_kind_inference() {
        assert_eq!(topic_for_kind(&Kind::ModelAnnounce), "neunode/inference");
        assert_eq!(topic_for_kind(&Kind::ServeOffer), "neunode/inference");
        assert_eq!(topic_for_kind(&Kind::BenchmarkClaim), "neunode/inference");
    }

    #[test]
    fn topic_for_kind_governance() {
        assert_eq!(topic_for_kind(&Kind::Proposal), "neunode/governance");
        assert_eq!(topic_for_kind(&Kind::Vote), "neunode/governance");
        assert_eq!(topic_for_kind(&Kind::ParameterChange), "neunode/governance");
    }

    #[test]
    fn all_topics_count_and_contents() {
        let topics = all_topics();
        assert_eq!(topics.len(), 7);
        assert!(topics.contains(&"neunode/system"));
        assert!(topics.contains(&"neunode/bounty"));
        assert!(topics.contains(&"neunode/training"));
        assert!(topics.contains(&"neunode/attestation"));
        assert!(topics.contains(&"neunode/inference"));
        assert!(topics.contains(&"neunode/governance"));
        assert!(topics.contains(&"neunode/custom"));
    }

    #[test]
    fn topic_for_category_matches_kind() {
        let categories = [
            KindCategory::System,
            KindCategory::Bounty,
            KindCategory::Training,
            KindCategory::Attestation,
            KindCategory::Inference,
            KindCategory::Governance,
            KindCategory::Custom,
        ];
        for cat in &categories {
            let topic = topic_for_category(*cat);
            assert!(topic.starts_with("neunode/"));
        }
    }

    #[test]
    fn parse_roundtrip() {
        for topic in all_topics() {
            let cat = parse_topic(topic).expect("should parse");
            assert_eq!(topic_for_category(cat), topic);
        }
    }

    #[test]
    fn parse_invalid_topic() {
        assert_eq!(parse_topic("neunode/invalid"), None);
        assert_eq!(parse_topic(""), None);
        assert_eq!(parse_topic("other/topic"), None);
    }

    #[test]
    fn is_valid_topic_true() {
        for topic in all_topics() {
            assert!(is_valid_topic(topic));
        }
    }

    #[test]
    fn is_valid_topic_false() {
        assert!(!is_valid_topic("neunode/invalid"));
        assert!(!is_valid_topic(""));
        assert!(!is_valid_topic("random"));
    }

    #[test]
    fn unknown_category_not_in_all_topics() {
        let topics = all_topics();
        assert!(!topics.contains(&"neunode/unknown"));
    }

    #[test]
    fn unknown_category_has_topic() {
        assert_eq!(topic_for_category(KindCategory::Unknown), "neunode/unknown");
    }
}
