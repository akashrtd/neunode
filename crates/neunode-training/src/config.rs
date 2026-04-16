use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Result, TrainingError};

/// Configuration for DiLoCo distributed training.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "bindings/training_config.ts")]
pub struct TrainingConfig {
    /// Number of local SGD steps before syncing with coordinator.
    #[ts(type = "number")]
    pub local_steps: u32,
    /// Learning rate for inner optimizer (AdamW).
    pub inner_lr: f64,
    /// Learning rate for outer optimizer (Nesterov SGD).
    pub outer_lr: f64,
    /// Outer momentum (Nesterov).
    pub outer_momentum: f64,
    /// Batch size per worker.
    #[ts(type = "number")]
    pub batch_size: u32,
    /// Gradient quantization bit width (8 for int8).
    #[ts(type = "number")]
    pub quantization_bits: u32,
    /// Maximum workers in a training group.
    #[ts(type = "number")]
    pub max_workers: u32,
    /// Heartbeat timeout in seconds before evicting a worker.
    #[ts(type = "number")]
    pub heartbeat_timeout_secs: u64,
    /// Checkpoint interval in outer steps.
    #[ts(type = "number")]
    pub checkpoint_interval: u32,
    /// Enable async training mode (no synchronization barrier).
    pub async_mode: bool,
    /// Minimum workers required before aggregation triggers.
    #[ts(type = "number")]
    pub min_workers: u32,
    /// Maximum staleness (outer steps behind) before dropping a gradient.
    /// 0 = unlimited (accept any staleness).
    #[ts(type = "number")]
    pub max_staleness: u32,
    /// Seconds to wait for stragglers after min_workers have reported.
    #[ts(type = "number")]
    pub grace_period_secs: u64,
    /// Seconds before forcing aggregation if min_workers have reported.
    #[ts(type = "number")]
    pub collection_timeout_secs: u64,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            local_steps: 500,
            inner_lr: 4e-4,
            outer_lr: 0.7,
            outer_momentum: 0.9,
            batch_size: 512,
            quantization_bits: 8,
            max_workers: 16,
            heartbeat_timeout_secs: 6,
            checkpoint_interval: 100,
            async_mode: false,
            min_workers: 2,
            max_staleness: 0,
            grace_period_secs: 10,
            collection_timeout_secs: 30,
        }
    }
}

impl TrainingConfig {
    /// Validates that all config fields have sensible values.
    pub fn validate(&self) -> Result<()> {
        if self.local_steps == 0 {
            return Err(TrainingError::ConfigInvalid("local_steps must be > 0".to_string()));
        }
        if self.inner_lr <= 0.0 {
            return Err(TrainingError::ConfigInvalid("inner_lr must be > 0".to_string()));
        }
        if self.outer_lr <= 0.0 {
            return Err(TrainingError::ConfigInvalid("outer_lr must be > 0".to_string()));
        }
        if self.batch_size == 0 {
            return Err(TrainingError::ConfigInvalid("batch_size must be > 0".to_string()));
        }
        if self.quantization_bits == 0 || self.quantization_bits > 16 {
            return Err(TrainingError::ConfigInvalid(
                "quantization_bits must be in [1, 16]".to_string(),
            ));
        }
        if self.max_workers == 0 {
            return Err(TrainingError::ConfigInvalid("max_workers must be >= 1".to_string()));
        }
        if self.heartbeat_timeout_secs == 0 {
            return Err(TrainingError::ConfigInvalid(
                "heartbeat_timeout_secs must be >= 1".to_string(),
            ));
        }
        if self.checkpoint_interval == 0 {
            return Err(TrainingError::ConfigInvalid(
                "checkpoint_interval must be >= 1".to_string(),
            ));
        }
        if self.min_workers == 0 {
            return Err(TrainingError::ConfigInvalid("min_workers must be >= 1".to_string()));
        }
        if self.min_workers > self.max_workers {
            return Err(TrainingError::ConfigInvalid(
                "min_workers must be <= max_workers".to_string(),
            ));
        }
        if self.grace_period_secs == 0 {
            return Err(TrainingError::ConfigInvalid("grace_period_secs must be >= 1".to_string()));
        }
        if self.collection_timeout_secs == 0 {
            return Err(TrainingError::ConfigInvalid(
                "collection_timeout_secs must be >= 1".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]
    use super::*;

    #[test]
    fn default_values() {
        let cfg = TrainingConfig::default();
        assert_eq!(cfg.local_steps, 500);
        assert!((cfg.inner_lr - 4e-4).abs() < f64::EPSILON);
        assert!((cfg.outer_lr - 0.7).abs() < f64::EPSILON);
        assert!((cfg.outer_momentum - 0.9).abs() < f64::EPSILON);
        assert_eq!(cfg.batch_size, 512);
        assert_eq!(cfg.quantization_bits, 8);
        assert_eq!(cfg.max_workers, 16);
        assert_eq!(cfg.heartbeat_timeout_secs, 6);
        assert_eq!(cfg.checkpoint_interval, 100);
        assert!(!cfg.async_mode);
        assert_eq!(cfg.min_workers, 2);
        assert_eq!(cfg.max_staleness, 0);
        assert_eq!(cfg.grace_period_secs, 10);
        assert_eq!(cfg.collection_timeout_secs, 30);
    }

    #[test]
    fn validate_defaults_passes() {
        let cfg = TrainingConfig::default();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn validate_zero_local_steps_fails() {
        let mut cfg = TrainingConfig::default();
        cfg.local_steps = 0;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, TrainingError::ConfigInvalid(_)));
        assert!(err.to_string().contains("local_steps"));
    }

    #[test]
    fn validate_negative_inner_lr_fails() {
        let mut cfg = TrainingConfig::default();
        cfg.inner_lr = -0.001;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, TrainingError::ConfigInvalid(_)));
        assert!(err.to_string().contains("inner_lr"));
    }

    #[test]
    fn validate_negative_outer_lr_fails() {
        let mut cfg = TrainingConfig::default();
        cfg.outer_lr = -1.0;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, TrainingError::ConfigInvalid(_)));
        assert!(err.to_string().contains("outer_lr"));
    }

    #[test]
    fn validate_zero_batch_size_fails() {
        let mut cfg = TrainingConfig::default();
        cfg.batch_size = 0;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, TrainingError::ConfigInvalid(_)));
        assert!(err.to_string().contains("batch_size"));
    }

    #[test]
    fn validate_quantization_bits_zero_fails() {
        let mut cfg = TrainingConfig::default();
        cfg.quantization_bits = 0;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, TrainingError::ConfigInvalid(_)));
        assert!(err.to_string().contains("quantization_bits"));
    }

    #[test]
    fn validate_quantization_bits_above_16_fails() {
        let mut cfg = TrainingConfig::default();
        cfg.quantization_bits = 17;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, TrainingError::ConfigInvalid(_)));
        assert!(err.to_string().contains("quantization_bits"));
    }

    #[test]
    fn validate_zero_max_workers_fails() {
        let mut cfg = TrainingConfig::default();
        cfg.max_workers = 0;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, TrainingError::ConfigInvalid(_)));
        assert!(err.to_string().contains("max_workers"));
    }

    #[test]
    fn validate_zero_heartbeat_timeout_fails() {
        let mut cfg = TrainingConfig::default();
        cfg.heartbeat_timeout_secs = 0;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, TrainingError::ConfigInvalid(_)));
        assert!(err.to_string().contains("heartbeat_timeout_secs"));
    }

    #[test]
    fn validate_zero_checkpoint_interval_fails() {
        let mut cfg = TrainingConfig::default();
        cfg.checkpoint_interval = 0;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, TrainingError::ConfigInvalid(_)));
        assert!(err.to_string().contains("checkpoint_interval"));
    }

    #[test]
    fn validate_boundary_values_pass() {
        let cfg = TrainingConfig {
            local_steps: 1,
            inner_lr: f64::MIN_POSITIVE,
            outer_lr: f64::MIN_POSITIVE,
            outer_momentum: 0.0,
            batch_size: 1,
            quantization_bits: 1,
            max_workers: 1,
            heartbeat_timeout_secs: 1,
            checkpoint_interval: 1,
            async_mode: true,
            min_workers: 1,
            max_staleness: 5,
            grace_period_secs: 1,
            collection_timeout_secs: 1,
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = TrainingConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: TrainingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.local_steps, deserialized.local_steps);
        assert_eq!(cfg.batch_size, deserialized.batch_size);
        assert_eq!(cfg.quantization_bits, deserialized.quantization_bits);
        assert_eq!(cfg.max_workers, deserialized.max_workers);
        assert_eq!(cfg.heartbeat_timeout_secs, deserialized.heartbeat_timeout_secs);
        assert_eq!(cfg.checkpoint_interval, deserialized.checkpoint_interval);
    }

    #[test]
    fn ts_export() {
        use ts_rs::Config;
        let cfg = Config::new();
        let name = TrainingConfig::name(&cfg);
        assert!(!name.is_empty());
    }
}
