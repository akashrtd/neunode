//! Intel TDX quote verification backed by the DCAP Quote Verification Library.
//!
//! Cryptographic quote and collateral verification is delegated to `dcap-qvl`.
//! This module applies Neunode's relying-party policy only after that validation
//! succeeds. Collateral acquisition is deliberately outside this verifier so a
//! caller can pin, cache, audit, and reproduce the exact evidence used.

use std::collections::BTreeSet;

use dcap_qvl::{quote::Report, verify::QuoteVerifier, QuoteCollateralV3};

use crate::error::{Result, VerificationError};

const MR_TD_LEN: usize = 48;
const REPORT_DATA_LEN: usize = 64;
const MAX_TDX_QUOTE_SIZE: usize = 1024 * 1024;

/// Relying-party policy applied after Intel DCAP verification succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntelTdxPolicy {
    /// Exact expected TDX initial measurement (`MR_TD`).
    pub expected_mr_td: [u8; MR_TD_LEN],
    /// Exact challenge-bound report data. Callers should domain-separate and hash
    /// their nonce plus the workload public key into this 64-byte value.
    pub expected_report_data: [u8; REPORT_DATA_LEN],
    /// Explicitly accepted Intel TCB result strings. Secure default: `UpToDate` only.
    pub accepted_tcb_statuses: BTreeSet<String>,
}

impl IntelTdxPolicy {
    pub fn strict(
        expected_mr_td: [u8; MR_TD_LEN],
        expected_report_data: [u8; REPORT_DATA_LEN],
    ) -> Self {
        Self {
            expected_mr_td,
            expected_report_data,
            accepted_tcb_statuses: BTreeSet::from(["UpToDate".to_string()]),
        }
    }
}

/// Claims returned only after both DCAP cryptographic verification and policy checks pass.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IntelTdxClaims {
    pub mr_td: Vec<u8>,
    pub report_data: Vec<u8>,
    pub tcb_status: String,
    pub advisory_ids: Vec<String>,
    pub verified_at_secs: u64,
}

/// Offline Intel TDX verifier using Intel's production root certificate.
pub struct IntelTdxVerifier {
    inner: QuoteVerifier,
}

impl IntelTdxVerifier {
    pub fn production() -> Self {
        Self { inner: QuoteVerifier::new_prod() }
    }

    /// Verify a raw TDX quote and its complete DCAP collateral at a caller-supplied time.
    ///
    /// Supplying time explicitly makes verification reproducible and prevents hidden
    /// wall-clock dependencies. The caller must use a trusted current time in production.
    pub fn verify(
        &self,
        raw_quote: &[u8],
        collateral: &QuoteCollateralV3,
        policy: &IntelTdxPolicy,
        now_secs: u64,
    ) -> Result<IntelTdxClaims> {
        if raw_quote.is_empty() {
            return Err(tee_error("Intel TDX quote is empty"));
        }
        if raw_quote.len() > MAX_TDX_QUOTE_SIZE {
            return Err(tee_error(format!(
                "Intel TDX quote exceeds the {MAX_TDX_QUOTE_SIZE}-byte safety limit"
            )));
        }
        if policy.accepted_tcb_statuses.is_empty() {
            return Err(tee_error("accepted TCB status policy cannot be empty"));
        }

        let verified = self
            .inner
            .verify(raw_quote, collateral, now_secs)
            .map_err(|error| tee_error(format!("Intel DCAP verification failed: {error:#}")))?;
        let report = match &verified.report {
            Report::TD10(report) => report,
            Report::TD15(report) => &report.base,
            Report::SgxEnclave(_) => {
                return Err(tee_error("Intel SGX quote supplied to the TDX verifier"));
            }
        };

        if report.mr_td != policy.expected_mr_td {
            return Err(tee_error(format!(
                "TDX MR_TD policy mismatch: expected {}, got {}",
                hex::encode(policy.expected_mr_td),
                hex::encode(report.mr_td)
            )));
        }
        if report.report_data != policy.expected_report_data {
            return Err(tee_error("TDX report data does not match the challenge binding"));
        }
        if !policy.accepted_tcb_statuses.contains(&verified.status) {
            return Err(tee_error(format!(
                "TDX TCB status '{}' is not accepted by policy",
                verified.status
            )));
        }

        Ok(IntelTdxClaims {
            mr_td: report.mr_td.to_vec(),
            report_data: report.report_data.to_vec(),
            tcb_status: verified.status,
            advisory_ids: verified.advisory_ids,
            verified_at_secs: now_secs,
        })
    }
}

impl Default for IntelTdxVerifier {
    fn default() -> Self {
        Self::production()
    }
}

fn tee_error(reason: impl Into<String>) -> VerificationError {
    VerificationError::TeeAttestationFailed(reason.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcap_qvl::quote::Quote;

    const FIXTURE_TIME: u64 = 1_751_000_000;

    fn tdx_fixture() -> (Vec<u8>, QuoteCollateralV3, IntelTdxPolicy) {
        let quote_hex = include_str!("tee_intel_tdx_quote.hex").trim();
        let raw_quote = hex::decode(quote_hex.strip_prefix("0x").unwrap_or(quote_hex)).unwrap();
        assert_eq!(raw_quote.len(), 5_006);
        let collateral =
            serde_json::from_str(include_str!("tee_intel_tdx_collateral.json")).unwrap();
        let quote = Quote::parse(&raw_quote).unwrap();
        let report = match quote.report {
            Report::TD10(report) => report,
            Report::TD15(report) => report.base,
            Report::SgxEnclave(_) => panic!("TDX fixture decoded as SGX"),
        };
        let policy = IntelTdxPolicy::strict(report.mr_td, report.report_data);
        (raw_quote, collateral, policy)
    }

    #[test]
    fn verifies_vendor_tdx_quote_end_to_end() {
        let (raw_quote, collateral, policy) = tdx_fixture();
        let verifier = IntelTdxVerifier::production();

        let claims = verifier.verify(&raw_quote, &collateral, &policy, FIXTURE_TIME).unwrap();

        assert_eq!(claims.mr_td, policy.expected_mr_td);
        assert_eq!(claims.report_data, policy.expected_report_data);
        assert_eq!(claims.tcb_status, "UpToDate");
        assert!(claims.advisory_ids.is_empty());
        assert_eq!(claims.verified_at_secs, FIXTURE_TIME);
    }

    #[test]
    fn rejects_tampered_vendor_tdx_quote() {
        let (mut raw_quote, collateral, policy) = tdx_fixture();
        raw_quote[128] ^= 1;
        let verifier = IntelTdxVerifier::production();

        let error = verifier.verify(&raw_quote, &collateral, &policy, FIXTURE_TIME).unwrap_err();

        assert!(error.to_string().contains("DCAP verification failed"));
    }

    #[test]
    fn rejects_vendor_tdx_quote_with_wrong_measurement_policy() {
        let (raw_quote, collateral, mut policy) = tdx_fixture();
        policy.expected_mr_td[0] ^= 1;
        let verifier = IntelTdxVerifier::production();

        let error = verifier.verify(&raw_quote, &collateral, &policy, FIXTURE_TIME).unwrap_err();

        assert!(error.to_string().contains("MR_TD policy mismatch"));
    }

    #[test]
    fn rejects_vendor_tdx_quote_with_expired_collateral() {
        let (raw_quote, collateral, policy) = tdx_fixture();
        let verifier = IntelTdxVerifier::production();

        let error = verifier.verify(&raw_quote, &collateral, &policy, 2_000_000_000).unwrap_err();

        assert!(error.to_string().contains("DCAP verification failed"));
    }

    #[test]
    fn strict_policy_accepts_only_up_to_date() {
        let policy = IntelTdxPolicy::strict([1; MR_TD_LEN], [2; REPORT_DATA_LEN]);
        assert_eq!(policy.accepted_tcb_statuses.len(), 1);
        assert!(policy.accepted_tcb_statuses.contains("UpToDate"));
    }

    #[test]
    fn empty_quote_fails_before_collateral_processing() {
        let verifier = IntelTdxVerifier::production();
        let collateral = QuoteCollateralV3 {
            pck_crl_issuer_chain: String::new(),
            root_ca_crl: Vec::new(),
            pck_crl: Vec::new(),
            tcb_info_issuer_chain: String::new(),
            tcb_info: String::new(),
            tcb_info_signature: Vec::new(),
            qe_identity_issuer_chain: String::new(),
            qe_identity: String::new(),
            qe_identity_signature: Vec::new(),
            pck_certificate_chain: None,
        };
        let policy = IntelTdxPolicy::strict([0; MR_TD_LEN], [0; REPORT_DATA_LEN]);

        let error = verifier.verify(&[], &collateral, &policy, 1_700_000_000).unwrap_err();
        assert!(error.to_string().contains("quote is empty"));
    }

    #[test]
    fn empty_status_policy_fails_closed() {
        let verifier = IntelTdxVerifier::production();
        let collateral = QuoteCollateralV3 {
            pck_crl_issuer_chain: String::new(),
            root_ca_crl: Vec::new(),
            pck_crl: Vec::new(),
            tcb_info_issuer_chain: String::new(),
            tcb_info: String::new(),
            tcb_info_signature: Vec::new(),
            qe_identity_issuer_chain: String::new(),
            qe_identity: String::new(),
            qe_identity_signature: Vec::new(),
            pck_certificate_chain: None,
        };
        let policy = IntelTdxPolicy {
            expected_mr_td: [0; MR_TD_LEN],
            expected_report_data: [0; REPORT_DATA_LEN],
            accepted_tcb_statuses: BTreeSet::new(),
        };

        let error = verifier.verify(&[1], &collateral, &policy, 1_700_000_000).unwrap_err();
        assert!(error.to_string().contains("cannot be empty"));
    }

    #[test]
    fn oversized_quote_fails_before_parsing() {
        let verifier = IntelTdxVerifier::production();
        let collateral = QuoteCollateralV3 {
            pck_crl_issuer_chain: String::new(),
            root_ca_crl: Vec::new(),
            pck_crl: Vec::new(),
            tcb_info_issuer_chain: String::new(),
            tcb_info: String::new(),
            tcb_info_signature: Vec::new(),
            qe_identity_issuer_chain: String::new(),
            qe_identity: String::new(),
            qe_identity_signature: Vec::new(),
            pck_certificate_chain: None,
        };
        let policy = IntelTdxPolicy::strict([0; MR_TD_LEN], [0; REPORT_DATA_LEN]);

        let error = verifier
            .verify(&vec![0; MAX_TDX_QUOTE_SIZE + 1], &collateral, &policy, 1_700_000_000)
            .unwrap_err();
        assert!(error.to_string().contains("safety limit"));
    }
}
