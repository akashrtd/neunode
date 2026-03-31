use neunode_core::types::{Hash256, Timestamp};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum VerificationLayer {
    Layer1,
    Layer2,
    Layer3,
    Layer4,
}

impl VerificationLayer {
    pub fn index(&self) -> usize {
        match self {
            Self::Layer1 => 0,
            Self::Layer2 => 1,
            Self::Layer3 => 2,
            Self::Layer4 => 3,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Layer1 => "automated",
            Self::Layer2 => "ai_review",
            Self::Layer3 => "peer_review",
            Self::Layer4 => "arbitration",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct VerificationResult {
    pub layer: VerificationLayer,
    pub passed: bool,
    pub confidence: f64,
    pub evidence_hash: Hash256,
    pub timestamp: Timestamp,
}

impl VerificationResult {
    pub fn new(
        layer: VerificationLayer,
        passed: bool,
        confidence: f64,
        evidence_hash: Hash256,
        timestamp: Timestamp,
    ) -> Self {
        Self { layer, passed, confidence, evidence_hash, timestamp }
    }
}

/// Simulated artifact verification for Phase 1.
/// In production, this will use Gauntlet, RepOps, peer review, and ZK proofs.
pub fn verify_artifact(artifact_hash: &Hash256, requirements: &str) -> VerificationResult {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let non_empty = !artifact_hash.0.is_empty();
    let has_requirements = !requirements.is_empty();

    let passed = non_empty && has_requirements;
    let confidence = if passed { 0.85 } else { 0.0 };

    VerificationResult::new(
        VerificationLayer::Layer1,
        passed,
        confidence,
        artifact_hash.clone(),
        now,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub struct VerificationPipeline {
    pub layers: Vec<VerificationLayer>,
}

impl Default for VerificationPipeline {
    fn default() -> Self {
        Self {
            layers: vec![
                VerificationLayer::Layer1,
                VerificationLayer::Layer2,
                VerificationLayer::Layer3,
            ],
        }
    }
}

impl VerificationPipeline {
    pub fn new(layers: Vec<VerificationLayer>) -> Self {
        let mut sorted = layers;
        sorted.sort_by_key(|l| l.index());
        Self { layers: sorted }
    }

    pub fn run(&self, artifact_hash: &Hash256, requirements: &str) -> Vec<VerificationResult> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.layers
            .iter()
            .enumerate()
            .map(|(i, layer)| {
                let passed = match layer {
                    VerificationLayer::Layer1 => {
                        !artifact_hash.0.is_empty() && !requirements.is_empty()
                    }
                    VerificationLayer::Layer2 => true,
                    VerificationLayer::Layer3 => i < 3,
                    VerificationLayer::Layer4 => true,
                };
                let confidence = match layer {
                    VerificationLayer::Layer1 => {
                        if passed {
                            0.85
                        } else {
                            0.0
                        }
                    }
                    VerificationLayer::Layer2 => 0.90,
                    VerificationLayer::Layer3 => 0.95,
                    VerificationLayer::Layer4 => 1.0,
                };
                VerificationResult::new(*layer, passed, confidence, artifact_hash.clone(), now)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hash() -> Hash256 {
        Hash256("abc123def456".to_string())
    }

    #[test]
    fn verification_layer_index() {
        assert_eq!(VerificationLayer::Layer1.index(), 0);
        assert_eq!(VerificationLayer::Layer2.index(), 1);
        assert_eq!(VerificationLayer::Layer3.index(), 2);
        assert_eq!(VerificationLayer::Layer4.index(), 3);
    }

    #[test]
    fn verification_layer_name() {
        assert_eq!(VerificationLayer::Layer1.name(), "automated");
        assert_eq!(VerificationLayer::Layer2.name(), "ai_review");
        assert_eq!(VerificationLayer::Layer3.name(), "peer_review");
        assert_eq!(VerificationLayer::Layer4.name(), "arbitration");
    }

    #[test]
    fn verification_layer_serde_roundtrip() {
        for layer in [
            VerificationLayer::Layer1,
            VerificationLayer::Layer2,
            VerificationLayer::Layer3,
            VerificationLayer::Layer4,
        ] {
            let json = serde_json::to_string(&layer).unwrap();
            let back: VerificationLayer = serde_json::from_str(&json).unwrap();
            assert_eq!(layer, back);
        }
    }

    #[test]
    fn verification_result_new() {
        let result =
            VerificationResult::new(VerificationLayer::Layer1, true, 0.9, test_hash(), 1000);
        assert_eq!(result.layer, VerificationLayer::Layer1);
        assert!(result.passed);
        assert!((result.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(result.evidence_hash, test_hash());
        assert_eq!(result.timestamp, 1000);
    }

    #[test]
    fn verification_result_serde_roundtrip() {
        let result =
            VerificationResult::new(VerificationLayer::Layer2, false, 0.5, test_hash(), 42);
        let json = serde_json::to_string(&result).unwrap();
        let back: VerificationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, back);
    }

    #[test]
    fn verify_artifact_valid() {
        let result = verify_artifact(&test_hash(), "accuracy > 95%");
        assert!(result.passed);
        assert!((result.confidence - 0.85).abs() < f64::EPSILON);
        assert_eq!(result.layer, VerificationLayer::Layer1);
    }

    #[test]
    fn verify_artifact_empty_hash() {
        let result = verify_artifact(&Hash256(String::new()), "accuracy > 95%");
        assert!(!result.passed);
        assert!((result.confidence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn verify_artifact_empty_requirements() {
        let result = verify_artifact(&test_hash(), "");
        assert!(!result.passed);
    }

    #[test]
    fn pipeline_default_layers() {
        let pipeline = VerificationPipeline::default();
        assert_eq!(pipeline.layers.len(), 3);
        assert_eq!(pipeline.layers[0], VerificationLayer::Layer1);
        assert_eq!(pipeline.layers[1], VerificationLayer::Layer2);
        assert_eq!(pipeline.layers[2], VerificationLayer::Layer3);
    }

    #[test]
    fn pipeline_run_default() {
        let pipeline = VerificationPipeline::default();
        let results = pipeline.run(&test_hash(), "accuracy > 95%");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].layer, VerificationLayer::Layer1);
        assert_eq!(results[1].layer, VerificationLayer::Layer2);
        assert_eq!(results[2].layer, VerificationLayer::Layer3);
    }

    #[test]
    fn pipeline_run_all_pass() {
        let pipeline = VerificationPipeline::default();
        let results = pipeline.run(&test_hash(), "accuracy > 95%");
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn pipeline_run_confidence_increases() {
        let pipeline = VerificationPipeline::default();
        let results = pipeline.run(&test_hash(), "accuracy > 95%");
        for i in 1..results.len() {
            assert!(results[i].confidence >= results[i - 1].confidence);
        }
    }

    #[test]
    fn pipeline_custom_layers_sorted() {
        let pipeline =
            VerificationPipeline::new(vec![VerificationLayer::Layer3, VerificationLayer::Layer1]);
        assert_eq!(pipeline.layers[0], VerificationLayer::Layer1);
        assert_eq!(pipeline.layers[1], VerificationLayer::Layer3);
    }

    #[test]
    fn pipeline_all_four_layers() {
        let pipeline = VerificationPipeline::new(vec![
            VerificationLayer::Layer4,
            VerificationLayer::Layer1,
            VerificationLayer::Layer2,
            VerificationLayer::Layer3,
        ]);
        let results = pipeline.run(&test_hash(), "accuracy > 95%");
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].layer, VerificationLayer::Layer1);
        assert_eq!(results[3].layer, VerificationLayer::Layer4);
    }

    #[test]
    fn pipeline_serde_roundtrip() {
        let pipeline = VerificationPipeline::default();
        let json = serde_json::to_string(&pipeline).unwrap();
        let back: VerificationPipeline = serde_json::from_str(&json).unwrap();
        assert_eq!(pipeline, back);
    }

    #[test]
    fn verify_artifact_timestamp_set() {
        let result = verify_artifact(&test_hash(), "requirements");
        assert!(result.timestamp > 0);
    }
}
