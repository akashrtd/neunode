use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::Result;
#[cfg(not(feature = "tee-sim"))]
use crate::error::VerificationError;

/// Type of trusted execution environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum TeeType {
    IntelTdx,
    AmdSev,
    NvidiaCcn,
    AppleSecureEnclave,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

/// Raw TEE attestation quote.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct TeeQuote {
    pub tee_type: TeeType,
    pub measurement_hash: String,
    pub signer_public_key: Vec<u8>,
    #[ts(type = "number")]
    pub timestamp_ms: u64,
    pub nonce: Vec<u8>,
    pub raw_quote: Vec<u8>,
}

/// Verified TEE attestation result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct TeeAttestation {
    pub quote: TeeQuote,
    pub verified: bool,
    #[ts(type = "number")]
    pub verification_timestamp_ms: u64,
    pub verifier_id: String,
}

/// Verifies TEE attestation quotes.
///
/// Production builds fail closed until a vendor-backed verifier is configured.
/// The `tee-sim` feature is intended only for explicit development and testing.
pub struct TeeVerifier;

impl TeeVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Verifies a TEE attestation quote.
    ///
    /// With the opt-in `tee-sim` feature, this performs a simulated
    /// verification checking measurement hash and nonce match. Without `tee-sim`,
    /// this generic interface returns an error; production callers must use the
    /// vendor-backed Intel TDX or AMD SEV-SNP verifier.
    #[cfg(feature = "tee-sim")]
    pub fn verify_quote(
        &self,
        quote: &TeeQuote,
        expected_measurement: &str,
        challenge_nonce: &[u8],
    ) -> Result<TeeAttestation> {
        let measurement_ok = quote.measurement_hash == expected_measurement;
        let nonce_ok = quote.nonce == challenge_nonce;
        let verified = measurement_ok && nonce_ok;

        Ok(TeeAttestation {
            quote: quote.clone(),
            verified,
            verification_timestamp_ms: now_ms(),
            verifier_id: "tee_simulator_untrusted".to_string(),
        })
    }

    /// Generic production interface — fails closed so callers cannot accidentally
    /// substitute it for the vendor-backed Intel TDX or AMD SEV-SNP verifiers.
    #[cfg(not(feature = "tee-sim"))]
    pub fn verify_quote(
        &self,
        _quote: &TeeQuote,
        _expected_measurement: &str,
        _challenge_nonce: &[u8],
    ) -> Result<TeeAttestation> {
        Err(VerificationError::TeeAttestationFailed(
            "generic TEE verification is unavailable; select Intel TDX or AMD SEV-SNP verification"
                .to_string(),
        ))
    }

    pub fn is_quote_fresh(&self, quote: &TeeQuote, max_age_secs: u64) -> bool {
        let now = now_ms();
        let quote_age_ms = now.saturating_sub(quote.timestamp_ms);
        quote_age_ms <= max_age_secs * 1000
    }
}

impl Default for TeeVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_quote(timestamp_ms: u64) -> TeeQuote {
        TeeQuote {
            tee_type: TeeType::IntelTdx,
            measurement_hash: "abc123".to_string(),
            signer_public_key: vec![1, 2, 3, 4],
            timestamp_ms,
            nonce: vec![0xAA, 0xBB],
            raw_quote: vec![0xDE, 0xAD, 0xBE, 0xEF],
        }
    }

    #[test]
    #[cfg(feature = "tee-sim")]
    fn verify_quote_match() {
        let verifier = TeeVerifier::new();
        let quote = sample_quote(now_ms());
        let attestation = verifier.verify_quote(&quote, "abc123", &[0xAA, 0xBB]).unwrap();
        assert!(attestation.verified);
        assert_eq!(attestation.verifier_id, "tee_simulator_untrusted");
    }

    #[test]
    #[cfg(feature = "tee-sim")]
    fn verify_quote_mismatch_measurement() {
        let verifier = TeeVerifier::new();
        let quote = sample_quote(now_ms());
        let attestation = verifier.verify_quote(&quote, "wrong_hash", &[0xAA, 0xBB]).unwrap();
        assert!(!attestation.verified);
    }

    #[test]
    #[cfg(feature = "tee-sim")]
    fn verify_quote_mismatch_nonce() {
        let verifier = TeeVerifier::new();
        let quote = sample_quote(now_ms());
        let attestation = verifier.verify_quote(&quote, "abc123", &[0xFF, 0xFF]).unwrap();
        assert!(!attestation.verified);
    }

    #[test]
    fn is_quote_fresh_recent() {
        let verifier = TeeVerifier::new();
        let quote = sample_quote(now_ms() - 5000); // 5s ago
        assert!(verifier.is_quote_fresh(&quote, 10));
    }

    #[test]
    fn is_quote_fresh_expired() {
        let verifier = TeeVerifier::new();
        let quote = sample_quote(now_ms() - 20000); // 20s ago
        assert!(!verifier.is_quote_fresh(&quote, 10));
    }

    #[test]
    fn tee_type_serde_roundtrip() {
        let types = vec![
            TeeType::IntelTdx,
            TeeType::AmdSev,
            TeeType::NvidiaCcn,
            TeeType::AppleSecureEnclave,
        ];
        for tt in types {
            let json = serde_json::to_string(&tt).unwrap();
            let back: TeeType = serde_json::from_str(&json).unwrap();
            assert_eq!(tt, back);
        }
    }

    #[test]
    fn tee_type_snake_case() {
        let json = serde_json::to_string(&TeeType::IntelTdx).unwrap();
        assert!(json.contains("intel_tdx"));
        let json = serde_json::to_string(&TeeType::NvidiaCcn).unwrap();
        assert!(json.contains("nvidia_ccn"));
        let json = serde_json::to_string(&TeeType::AppleSecureEnclave).unwrap();
        assert!(json.contains("apple_secure_enclave"));
    }

    #[test]
    fn quote_serde_roundtrip() {
        let quote = sample_quote(1700000000000);
        let json = serde_json::to_string(&quote).unwrap();
        let back: TeeQuote = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tee_type, TeeType::IntelTdx);
        assert_eq!(back.measurement_hash, "abc123");
        assert_eq!(back.nonce, vec![0xAA, 0xBB]);
    }

    #[test]
    #[cfg(feature = "tee-sim")]
    fn attestation_serde_roundtrip() {
        let verifier = TeeVerifier::new();
        let quote = sample_quote(1700000000000);
        let attestation = verifier.verify_quote(&quote, "abc123", &[0xAA, 0xBB]).unwrap();
        let json = serde_json::to_string(&attestation).unwrap();
        let back: TeeAttestation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.verified, attestation.verified);
        assert_eq!(back.quote.measurement_hash, "abc123");
    }

    #[test]
    #[cfg(feature = "tee-sim")]
    fn different_tee_types() {
        let verifier = TeeVerifier::new();
        for tt in
            [TeeType::IntelTdx, TeeType::AmdSev, TeeType::NvidiaCcn, TeeType::AppleSecureEnclave]
        {
            let quote = TeeQuote { tee_type: tt, ..sample_quote(now_ms()) };
            let att = verifier.verify_quote(&quote, "abc123", &[0xAA, 0xBB]).unwrap();
            assert!(att.verified);
        }
    }

    #[test]
    #[cfg(feature = "tee-sim")]
    fn raw_quote_preserved() {
        let verifier = TeeVerifier::new();
        let quote = sample_quote(now_ms());
        let att = verifier.verify_quote(&quote, "abc123", &[0xAA, 0xBB]).unwrap();
        assert_eq!(att.quote.raw_quote, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    #[cfg(feature = "tee-sim")]
    fn signer_public_key_preserved() {
        let verifier = TeeVerifier::new();
        let quote = sample_quote(now_ms());
        let att = verifier.verify_quote(&quote, "abc123", &[0xAA, 0xBB]).unwrap();
        assert_eq!(att.quote.signer_public_key, vec![1, 2, 3, 4]);
    }

    #[test]
    #[cfg(feature = "tee-sim")]
    fn nonce_mismatch() {
        let verifier = TeeVerifier::new();
        let mut quote = sample_quote(now_ms());
        quote.nonce = vec![0x11, 0x22];
        let att = verifier.verify_quote(&quote, "abc123", &[0xAA, 0xBB]).unwrap();
        assert!(!att.verified);
        assert_eq!(att.quote.measurement_hash, "abc123");
    }

    #[test]
    #[cfg(not(feature = "tee-sim"))]
    fn production_build_fails_closed() {
        let verifier = TeeVerifier::new();
        let quote = sample_quote(now_ms());
        let error = verifier.verify_quote(&quote, "abc123", &[0xAA, 0xBB]).unwrap_err();
        assert!(error.to_string().contains("production TEE verification not yet implemented"));
    }

    #[test]
    fn ts_type_declarations() {
        let cfg = ts_rs::Config::default();
        let _ = <TeeType as ts_rs::TS>::decl(&cfg);
        let _ = <TeeQuote as ts_rs::TS>::decl(&cfg);
        let _ = <TeeAttestation as ts_rs::TS>::decl(&cfg);
    }
}
