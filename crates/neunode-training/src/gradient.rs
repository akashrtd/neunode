use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Result, TrainingError};

/// Wire format for gradient exchange in DiLoCo training.
///
/// Used when serializing pseudo-gradients for `Kind::GradientUpdate` (2010)
/// feed events distributed over the P2P mesh.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum GradientWireFormat {
    /// Full 32-bit float precision (low-bandwidth debugging).
    F32,
    /// Int8 quantized: `q = clamp(round(f / scale), -128, 127)`.
    /// Production default — 4× compression over F32.
    Int8 { scale: f32 },
}

/// A gradient message ready for wire transmission.
///
/// Contains encoded pseudo-gradients along with metadata identifying the
/// source worker, training job, and outer step. The checksum enables
/// integrity verification after P2P transport.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GradientMessage {
    /// DID or identifier of the sending worker.
    pub worker_id: String,
    /// Training job identifier this gradient belongs to.
    pub job_id: String,
    /// Outer step number this pseudo-gradient targets.
    #[ts(type = "number")]
    pub outer_step: u64,
    /// Encoding format (F32 or Int8 with scale factor).
    pub format: GradientWireFormat,
    /// Serialized gradient data (raw f32 bytes or quantized i8 bytes).
    pub payload: Vec<u8>,
    /// XOR-fold checksum of payload as hex string (64 chars).
    pub checksum: String,
}

/// Compute a 32-byte XOR-fold checksum and return as lowercase hex.
///
/// XOR-folds the payload bytes into a 32-byte accumulator (cycling),
/// then hex-encodes the result.
fn xor_fold_checksum(data: &[u8]) -> String {
    let mut acc = [0u8; 32];
    for (i, &byte) in data.iter().enumerate() {
        acc[i % 32] ^= byte;
    }
    hex::encode(acc)
}

impl GradientMessage {
    /// Encode full-precision f32 gradients into a wire message.
    ///
    /// Each gradient value is stored as 4 native-endian bytes.
    /// Payload length = `gradients.len() * 4`.
    pub fn encode_f32(
        worker_id: &str,
        job_id: &str,
        outer_step: u64,
        gradients: &[f32],
    ) -> Result<Self> {
        if gradients.is_empty() {
            return Err(TrainingError::AggregationFailed(
                "cannot encode empty gradients".to_string(),
            ));
        }
        let mut payload = Vec::with_capacity(gradients.len() * 4);
        for &g in gradients {
            payload.extend_from_slice(&g.to_ne_bytes());
        }
        let checksum = xor_fold_checksum(&payload);
        Ok(Self {
            worker_id: worker_id.to_string(),
            job_id: job_id.to_string(),
            outer_step,
            format: GradientWireFormat::F32,
            payload,
            checksum,
        })
    }

    /// Encode int8-quantized gradients into a wire message.
    ///
    /// Quantization: `q_i = clamp(round(f_i / scale), -128, 127)` stored as
    /// a single signed byte. Payload length = `gradients.len()`.
    /// Dequantized: `f_approx = q_i * scale`.
    pub fn encode_int8(
        worker_id: &str,
        job_id: &str,
        outer_step: u64,
        gradients: &[f32],
        scale: f32,
    ) -> Result<Self> {
        if gradients.is_empty() {
            return Err(TrainingError::AggregationFailed(
                "cannot encode empty gradients".to_string(),
            ));
        }
        if scale <= 0.0 || !scale.is_finite() {
            return Err(TrainingError::ConfigInvalid(
                "scale must be positive and finite".to_string(),
            ));
        }
        let payload: Vec<u8> = gradients
            .iter()
            .map(|&g| {
                let q = (g / scale).round().clamp(-128.0, 127.0) as i8;
                q as u8
            })
            .collect();
        let checksum = xor_fold_checksum(&payload);
        Ok(Self {
            worker_id: worker_id.to_string(),
            job_id: job_id.to_string(),
            outer_step,
            format: GradientWireFormat::Int8 { scale },
            payload,
            checksum,
        })
    }

    /// Decode payload back to f32 gradient values.
    ///
    /// For F32 format: interprets payload as native-endian f32 bytes.
    /// For Int8 format: dequantizes each byte as `q * scale`.
    ///
    /// Returns `GradientMismatch` if payload length is not a multiple of 4
    /// (F32) or if the payload is empty.
    pub fn decode(&self) -> Result<Vec<f32>> {
        if self.payload.is_empty() {
            return Err(TrainingError::GradientMismatch);
        }
        match self.format {
            GradientWireFormat::F32 => {
                if !self.payload.len().is_multiple_of(4) {
                    return Err(TrainingError::GradientMismatch);
                }
                let count = self.payload.len() / 4;
                let mut out = Vec::with_capacity(count);
                for chunk in self.payload.chunks_exact(4) {
                    let bytes: [u8; 4] =
                        chunk.try_into().expect("chunks_exact(4) always yields 4 bytes");
                    out.push(f32::from_ne_bytes(bytes));
                }
                Ok(out)
            }
            GradientWireFormat::Int8 { scale } => {
                Ok(self.payload.iter().map(|&b| (b as i8 as f32) * scale).collect())
            }
        }
    }

    /// Verify payload integrity against the stored checksum.
    ///
    /// Recomputes the XOR-fold checksum and compares to the stored value.
    pub fn verify_checksum(&self) -> bool {
        let computed = xor_fold_checksum(&self.payload);
        self.checksum == computed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Encode F32 tests ──────────────────────────────────────────────

    #[test]
    fn encode_f32_basic() {
        let grads = vec![1.0, -2.5, 3.14];
        let msg = GradientMessage::encode_f32("w1", "job1", 5, &grads).unwrap();
        assert_eq!(msg.worker_id, "w1");
        assert_eq!(msg.job_id, "job1");
        assert_eq!(msg.outer_step, 5);
        assert_eq!(msg.format, GradientWireFormat::F32);
        assert_eq!(msg.payload.len(), 12); // 3 × 4 bytes
    }

    #[test]
    fn encode_f32_roundtrip() {
        let grads = vec![0.1, -0.2, 0.3, -0.4, 0.5];
        let msg = GradientMessage::encode_f32("w1", "job1", 0, &grads).unwrap();
        let decoded = msg.decode().unwrap();
        assert_eq!(decoded, grads);
    }

    #[test]
    fn encode_f32_empty_gradients_error() {
        let result = GradientMessage::encode_f32("w1", "job1", 0, &[]);
        assert!(result.is_err());
        match result.unwrap_err() {
            TrainingError::AggregationFailed(msg) => {
                assert!(msg.contains("empty"));
            }
            other => panic!("expected AggregationFailed, got {other}"),
        }
    }

    // ── Encode Int8 tests ─────────────────────────────────────────────

    #[test]
    fn encode_int8_basic() {
        let grads = vec![0.1, -0.2, 0.3];
        let msg = GradientMessage::encode_int8("w2", "job2", 3, &grads, 0.01).unwrap();
        assert_eq!(msg.worker_id, "w2");
        assert_eq!(msg.job_id, "job2");
        assert_eq!(msg.outer_step, 3);
        assert!(matches!(msg.format, GradientWireFormat::Int8 { scale: 0.01 }));
        assert_eq!(msg.payload.len(), 3); // 1 byte per value
    }

    #[test]
    fn encode_int8_roundtrip() {
        let grads = vec![0.05, -0.03, 0.07, -0.01];
        let scale = 0.01f32;
        let msg = GradientMessage::encode_int8("w1", "job1", 1, &grads, scale).unwrap();
        let decoded = msg.decode().unwrap();
        // Int8 quantization has precision loss — check approximate equality.
        for (orig, dec) in grads.iter().zip(decoded.iter()) {
            let diff = (orig - dec).abs();
            assert!(diff <= scale, "diff {diff} > scale {scale}");
        }
    }

    #[test]
    fn encode_int8_clamps_large_values() {
        // Values larger than 127 * scale should clamp to 127.
        let scale = 1.0f32;
        let grads = vec![200.0, -300.0];
        let msg = GradientMessage::encode_int8("w1", "job1", 0, &grads, scale).unwrap();
        let decoded = msg.decode().unwrap();
        assert!((decoded[0] - 127.0).abs() < f32::EPSILON);
        assert!((decoded[1] - (-128.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn encode_int8_empty_gradients_error() {
        let result = GradientMessage::encode_int8("w1", "job1", 0, &[], 0.01);
        assert!(result.is_err());
    }

    #[test]
    fn encode_int8_zero_scale_error() {
        let result = GradientMessage::encode_int8("w1", "job1", 0, &[1.0], 0.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            TrainingError::ConfigInvalid(msg) => {
                assert!(msg.contains("scale"));
            }
            other => panic!("expected ConfigInvalid, got {other}"),
        }
    }

    #[test]
    fn encode_int8_negative_scale_error() {
        let result = GradientMessage::encode_int8("w1", "job1", 0, &[1.0], -0.5);
        assert!(result.is_err());
    }

    #[test]
    fn encode_int8_nan_scale_error() {
        let result = GradientMessage::encode_int8("w1", "job1", 0, &[1.0], f32::NAN);
        assert!(result.is_err());
    }

    // ── Checksum tests ────────────────────────────────────────────────

    #[test]
    fn checksum_verifies_f32() {
        let msg = GradientMessage::encode_f32("w1", "job1", 0, &[1.0, 2.0, 3.0]).unwrap();
        assert!(msg.verify_checksum());
    }

    #[test]
    fn checksum_verifies_int8() {
        let msg = GradientMessage::encode_int8("w1", "job1", 0, &[0.1, -0.2, 0.3], 0.01).unwrap();
        assert!(msg.verify_checksum());
    }

    #[test]
    fn checksum_tamper_detection() {
        let mut msg = GradientMessage::encode_f32("w1", "job1", 0, &[1.0, 2.0]).unwrap();
        assert!(msg.verify_checksum());
        // Tamper with payload.
        msg.payload[0] ^= 0xFF;
        assert!(!msg.verify_checksum());
    }

    #[test]
    fn checksum_tamper_checksum_string() {
        let mut msg = GradientMessage::encode_f32("w1", "job1", 0, &[1.0]).unwrap();
        let original_checksum = msg.checksum.clone();
        msg.checksum = "0".repeat(original_checksum.len());
        assert!(!msg.verify_checksum());
    }

    #[test]
    fn checksum_length_is_64_hex_chars() {
        let msg = GradientMessage::encode_f32("w1", "job1", 0, &[1.0]).unwrap();
        assert_eq!(msg.checksum.len(), 64);
        assert!(msg.checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ── Decode error tests ────────────────────────────────────────────

    #[test]
    fn decode_empty_payload_error() {
        let msg = GradientMessage {
            worker_id: "w1".to_string(),
            job_id: "j1".to_string(),
            outer_step: 0,
            format: GradientWireFormat::F32,
            payload: vec![],
            checksum: xor_fold_checksum(&[]),
        };
        let result = msg.decode();
        assert!(result.is_err());
        match result.unwrap_err() {
            TrainingError::GradientMismatch => {}
            other => panic!("expected GradientMismatch, got {other}"),
        }
    }

    #[test]
    fn decode_f32_bad_length_error() {
        let msg = GradientMessage {
            worker_id: "w1".to_string(),
            job_id: "j1".to_string(),
            outer_step: 0,
            format: GradientWireFormat::F32,
            payload: vec![0x00, 0x01, 0x02], // 3 bytes, not multiple of 4
            checksum: "fake".to_string(),
        };
        let result = msg.decode();
        assert!(result.is_err());
    }

    // ── Serde roundtrip tests ─────────────────────────────────────────

    #[test]
    fn serde_roundtrip_f32_message() {
        let msg = GradientMessage::encode_f32("worker-7", "job-42", 99, &[0.1, -0.2]).unwrap();
        let json = serde_json::to_string(&msg).unwrap();
        let back: GradientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.worker_id, "worker-7");
        assert_eq!(back.job_id, "job-42");
        assert_eq!(back.outer_step, 99);
        assert_eq!(back.format, GradientWireFormat::F32);
        assert_eq!(back.payload, msg.payload);
        assert_eq!(back.checksum, msg.checksum);
        // Decoded gradients match.
        assert_eq!(back.decode().unwrap(), msg.decode().unwrap());
    }

    #[test]
    fn serde_roundtrip_int8_message() {
        let msg =
            GradientMessage::encode_int8("worker-3", "job-10", 7, &[0.05, -0.03], 0.01).unwrap();
        let json = serde_json::to_string(&msg).unwrap();
        let back: GradientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.worker_id, "worker-3");
        assert_eq!(back.job_id, "job-10");
        assert_eq!(back.outer_step, 7);
        assert!(matches!(back.format, GradientWireFormat::Int8 { scale: 0.01 }));
    }

    #[test]
    fn serde_wire_format_f32() {
        let fmt = GradientWireFormat::F32;
        let json = serde_json::to_string(&fmt).unwrap();
        assert!(json.contains("f32") || json.contains("F32"));
        let back: GradientWireFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(fmt, back);
    }

    #[test]
    fn serde_wire_format_int8() {
        let fmt = GradientWireFormat::Int8 { scale: 0.05 };
        let json = serde_json::to_string(&fmt).unwrap();
        let back: GradientWireFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(fmt, back);
    }

    // ── Format variant tests ──────────────────────────────────────────

    #[test]
    fn format_equality_f32() {
        assert_eq!(GradientWireFormat::F32, GradientWireFormat::F32);
    }

    #[test]
    fn format_equality_int8() {
        assert_eq!(
            GradientWireFormat::Int8 { scale: 0.1 },
            GradientWireFormat::Int8 { scale: 0.1 }
        );
    }

    #[test]
    fn format_inequality() {
        assert_ne!(GradientWireFormat::F32, GradientWireFormat::Int8 { scale: 1.0 });
        assert_ne!(
            GradientWireFormat::Int8 { scale: 0.1 },
            GradientWireFormat::Int8 { scale: 0.2 }
        );
    }

    // ── Field preservation tests ──────────────────────────────────────

    #[test]
    fn outer_step_preserved() {
        for step in [0, 1, 100, u64::MAX] {
            let msg = GradientMessage::encode_f32("w", "j", step, &[1.0]).unwrap();
            assert_eq!(msg.outer_step, step);
            let decoded = msg.decode().unwrap();
            assert_eq!(decoded, vec![1.0]);
        }
    }

    #[test]
    fn worker_id_preserved() {
        let msg = GradientMessage::encode_f32("did:neunode:abc123", "j1", 0, &[0.5]).unwrap();
        assert_eq!(msg.worker_id, "did:neunode:abc123");
    }

    #[test]
    fn job_id_preserved() {
        let msg = GradientMessage::encode_f32("w1", "train-llama-3b-step-42", 0, &[0.5]).unwrap();
        assert_eq!(msg.job_id, "train-llama-3b-step-42");
    }

    // ── Large gradient encoding ───────────────────────────────────────

    #[test]
    fn large_f32_gradient() {
        let grads: Vec<f32> = (0..10_000).map(|i| i as f32 * 0.001).collect();
        let msg = GradientMessage::encode_f32("w1", "big", 0, &grads).unwrap();
        assert_eq!(msg.payload.len(), 40_000);
        assert!(msg.verify_checksum());
        let decoded = msg.decode().unwrap();
        assert_eq!(decoded.len(), 10_000);
        for (i, &v) in decoded.iter().enumerate() {
            assert!((v - i as f32 * 0.001).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn large_int8_gradient() {
        let grads: Vec<f32> = (0..10_000).map(|i| i as f32 * 0.001 - 5.0).collect();
        let scale = 0.01f32;
        let msg = GradientMessage::encode_int8("w1", "big", 0, &grads, scale).unwrap();
        assert_eq!(msg.payload.len(), 10_000);
        assert!(msg.verify_checksum());
        let decoded = msg.decode().unwrap();
        assert_eq!(decoded.len(), 10_000);
    }

    // ── ts-rs export tests ────────────────────────────────────────────

    #[test]
    fn ts_export_gradient_wire_format() {
        use ts_rs::Config;
        let name = GradientWireFormat::name(&Config::new());
        assert!(!name.is_empty());
    }

    #[test]
    fn ts_export_gradient_message() {
        use ts_rs::Config;
        let name = GradientMessage::name(&Config::new());
        assert!(!name.is_empty());
    }

    // ── Snake case serde ──────────────────────────────────────────────

    #[test]
    fn wire_format_snake_case() {
        let json = serde_json::to_string(&GradientWireFormat::F32).unwrap();
        assert!(json.contains("f32"), "got: {json}");
        assert!(!json.contains("F32"), "should be snake_case: {json}");

        let json2 = serde_json::to_string(&GradientWireFormat::Int8 { scale: 0.1 }).unwrap();
        assert!(json2.contains("int8"), "got: {json2}");
        assert!(!json2.contains("Int8"), "should be snake_case: {json2}");
    }

    // ── Zero gradients ────────────────────────────────────────────────

    #[test]
    fn zero_gradients_f32() {
        let grads = vec![0.0f32; 100];
        let msg = GradientMessage::encode_f32("w1", "j1", 0, &grads).unwrap();
        let decoded = msg.decode().unwrap();
        assert!(decoded.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn zero_gradients_int8() {
        let grads = vec![0.0f32; 100];
        let msg = GradientMessage::encode_int8("w1", "j1", 0, &grads, 1.0).unwrap();
        let decoded = msg.decode().unwrap();
        assert!(decoded.iter().all(|&v| v == 0.0));
    }
}
