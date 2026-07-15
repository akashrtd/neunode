//! AMD SEV-SNP VCEK attestation verification.
//!
//! Verification is offline and deterministic: callers supply the report and
//! certificate chain. The chain is anchored to AMD's built-in production ARK/ASK,
//! VCEK extensions are bound to report chip/TCB values, the report signature is
//! verified, and relying-party measurement/challenge policy is applied last.

use der::Decode;
use sev::{
    certs::snp::{ca, Chain, Verifiable},
    firmware::guest::AttestationReport,
    Generation,
};
use x509_cert::{ext::Extension, Certificate as X509Certificate};

use crate::error::{Result, VerificationError};

const OID_BOOTLOADER: &str = "1.3.6.1.4.1.3704.1.3.1";
const OID_TEE: &str = "1.3.6.1.4.1.3704.1.3.2";
const OID_SNP: &str = "1.3.6.1.4.1.3704.1.3.3";
const OID_UCODE: &str = "1.3.6.1.4.1.3704.1.3.8";
const OID_FMC: &str = "1.3.6.1.4.1.3704.1.3.9";
const OID_HW_ID: &str = "1.3.6.1.4.1.3704.1.4";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmdGeneration {
    Milan,
    Genoa,
    Turin,
}

impl AmdGeneration {
    fn vendor_generation(self) -> Generation {
        match self {
            Self::Milan => Generation::Milan,
            Self::Genoa => Generation::Genoa,
            Self::Turin => Generation::Turin,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AmdTcb {
    pub bootloader: u8,
    pub tee: u8,
    pub snp: u8,
    pub microcode: u8,
    pub fmc: Option<u8>,
}

/// Strict relying-party policy for an AMD SEV-SNP workload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmdSnpPolicy {
    pub expected_measurement: [u8; 48],
    pub expected_report_data: [u8; 64],
    pub minimum_tcb: AmdTcb,
    pub allow_smt: bool,
    pub allow_migration: bool,
}

impl AmdSnpPolicy {
    pub fn strict(
        expected_measurement: [u8; 48],
        expected_report_data: [u8; 64],
        minimum_tcb: AmdTcb,
    ) -> Self {
        Self {
            expected_measurement,
            expected_report_data,
            minimum_tcb,
            allow_smt: false,
            allow_migration: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AmdSnpClaims {
    pub generation: AmdGeneration,
    pub measurement: Vec<u8>,
    pub report_data: Vec<u8>,
    pub chip_id: Vec<u8>,
    pub reported_tcb: AmdTcb,
    pub guest_svn: u32,
    pub vmpl: u32,
    pub verified_at_secs: u64,
}

/// Production verifier for AMD-signed VCEK reports.
pub struct AmdSnpVerifier {
    generation: AmdGeneration,
    trusted_ca: ca::Chain,
}

impl AmdSnpVerifier {
    /// Construct a verifier pinned to AMD's built-in production ARK and ASK.
    pub fn production_vcek(generation: AmdGeneration) -> Self {
        Self { generation, trusted_ca: generation.vendor_generation().into() }
    }

    pub fn verify(
        &self,
        report: &AttestationReport,
        chain: &Chain,
        policy: &AmdSnpPolicy,
        now_secs: u64,
    ) -> Result<AmdSnpClaims> {
        if chain.ca.ark != self.trusted_ca.ark || chain.ca.ask != self.trusted_ca.ask {
            return Err(tee_error(
                "SEV-SNP certificate chain is not anchored to the pinned AMD root",
            ));
        }

        verify_certificate_time(&chain.ca.ark, "ARK", now_secs)?;
        verify_certificate_time(&chain.ca.ask, "ASK", now_secs)?;
        verify_certificate_time(&chain.vek, "VCEK", now_secs)?;

        (chain, report).verify().map_err(|error| {
            tee_error(format!("SEV-SNP chain/report signature failed: {error}"))
        })?;
        verify_vcek_extensions(&chain.vek, report, self.generation)?;
        verify_policy(report, policy)?;

        Ok(AmdSnpClaims {
            generation: self.generation,
            measurement: report.measurement.to_vec(),
            report_data: report.report_data.to_vec(),
            chip_id: report.chip_id.to_vec(),
            reported_tcb: report_tcb(report),
            guest_svn: report.guest_svn,
            vmpl: report.vmpl,
            verified_at_secs: now_secs,
        })
    }
}

fn verify_certificate_time(
    certificate: &sev::certs::snp::Certificate,
    label: &str,
    now_secs: u64,
) -> Result<()> {
    let der = certificate
        .to_der()
        .map_err(|error| tee_error(format!("failed to encode {label} certificate: {error}")))?;
    let certificate = X509Certificate::from_der(&der)
        .map_err(|error| tee_error(format!("failed to parse {label} certificate: {error}")))?;
    let validity = certificate.tbs_certificate.validity;
    let not_before = validity.not_before.to_unix_duration().as_secs();
    let not_after = validity.not_after.to_unix_duration().as_secs();
    if now_secs < not_before {
        return Err(tee_error(format!(
            "{label} certificate is not valid until Unix time {not_before}"
        )));
    }
    if now_secs > not_after {
        return Err(tee_error(format!("{label} certificate expired at Unix time {not_after}")));
    }
    Ok(())
}

fn verify_vcek_extensions(
    vek: &sev::certs::snp::Certificate,
    report: &AttestationReport,
    generation: AmdGeneration,
) -> Result<()> {
    let der = vek
        .to_der()
        .map_err(|error| tee_error(format!("failed to encode VCEK certificate: {error}")))?;
    let certificate = X509Certificate::from_der(&der)
        .map_err(|error| tee_error(format!("failed to parse VCEK certificate: {error}")))?;

    require_integer_extension(&certificate, OID_BOOTLOADER, report.reported_tcb.bootloader)?;
    require_integer_extension(&certificate, OID_TEE, report.reported_tcb.tee)?;
    require_integer_extension(&certificate, OID_SNP, report.reported_tcb.snp)?;
    require_integer_extension(&certificate, OID_UCODE, report.reported_tcb.microcode)?;
    require_octet_extension(&certificate, OID_HW_ID, &report.chip_id)?;

    if generation == AmdGeneration::Turin {
        let fmc = report
            .reported_tcb
            .fmc
            .ok_or_else(|| tee_error("Turin report does not contain an FMC TCB value"))?;
        require_integer_extension(&certificate, OID_FMC, fmc)?;
    }
    Ok(())
}

fn verify_policy(report: &AttestationReport, policy: &AmdSnpPolicy) -> Result<()> {
    if report.measurement != policy.expected_measurement {
        return Err(tee_error(format!(
            "SEV-SNP measurement policy mismatch: expected {}, got {}",
            hex::encode(policy.expected_measurement),
            hex::encode(report.measurement)
        )));
    }
    if report.report_data != policy.expected_report_data {
        return Err(tee_error("SEV-SNP report data does not match the challenge binding"));
    }
    if report.policy.debug_allowed() {
        return Err(tee_error("SEV-SNP debug-enabled guests are not trusted"));
    }
    if report.policy.migrate_ma_allowed() && !policy.allow_migration {
        return Err(tee_error("SEV-SNP migration is not accepted by policy"));
    }
    if report.policy.smt_allowed() && !policy.allow_smt {
        return Err(tee_error("SEV-SNP SMT is not accepted by policy"));
    }

    let actual = report_tcb(report);
    ensure_tcb_at_least(actual, policy.minimum_tcb)
}

fn ensure_tcb_at_least(actual: AmdTcb, minimum: AmdTcb) -> Result<()> {
    let sufficient = actual.bootloader >= minimum.bootloader
        && actual.tee >= minimum.tee
        && actual.snp >= minimum.snp
        && actual.microcode >= minimum.microcode
        && match minimum.fmc {
            Some(minimum_fmc) => actual.fmc.is_some_and(|actual_fmc| actual_fmc >= minimum_fmc),
            None => true,
        };
    if sufficient {
        Ok(())
    } else {
        Err(tee_error(format!(
            "SEV-SNP reported TCB {actual:?} is below required minimum {minimum:?}"
        )))
    }
}

fn report_tcb(report: &AttestationReport) -> AmdTcb {
    AmdTcb {
        bootloader: report.reported_tcb.bootloader,
        tee: report.reported_tcb.tee,
        snp: report.reported_tcb.snp,
        microcode: report.reported_tcb.microcode,
        fmc: report.reported_tcb.fmc,
    }
}

fn extension<'a>(certificate: &'a X509Certificate, oid: &str) -> Result<&'a Extension> {
    certificate
        .tbs_certificate
        .extensions
        .as_ref()
        .and_then(|extensions| {
            extensions.iter().find(|extension| extension.extn_id.to_string() == oid)
        })
        .ok_or_else(|| tee_error(format!("VCEK is missing required extension {oid}")))
}

fn require_integer_extension(certificate: &X509Certificate, oid: &str, expected: u8) -> Result<()> {
    let value = extension(certificate, oid)?.extn_value.as_bytes();
    let actual = parse_der_u8(value)
        .ok_or_else(|| tee_error(format!("invalid VCEK integer extension {oid}")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(tee_error(format!(
            "VCEK extension {oid} mismatch: certificate {actual}, report {expected}"
        )))
    }
}

fn require_octet_extension(
    certificate: &X509Certificate,
    oid: &str,
    expected: &[u8],
) -> Result<()> {
    let value = extension(certificate, oid)?.extn_value.as_bytes();
    let actual = parse_der_octets(value)
        .ok_or_else(|| tee_error(format!("invalid VCEK octet extension {oid}")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(tee_error(format!("VCEK extension {oid} does not match the report")))
    }
}

fn parse_der_u8(value: &[u8]) -> Option<u8> {
    match value {
        [0x02, 0x01, byte] => Some(*byte),
        [0x02, 0x02, 0x00, byte] if *byte >= 0x80 => Some(*byte),
        _ => None,
    }
}

fn parse_der_octets(value: &[u8]) -> Option<&[u8]> {
    match value {
        [0x04, length, bytes @ ..] if usize::from(*length) == bytes.len() => Some(bytes),
        // `x509-cert` unwraps the extension's outer OCTET STRING. AMD encodes
        // HW_ID directly as its 64-byte content rather than a nested value.
        bytes if bytes.len() == 64 => Some(bytes),
        _ => None,
    }
}

fn tee_error(reason: impl Into<String>) -> VerificationError {
    VerificationError::TeeAttestationFailed(reason.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sev::{
        certs::snp::{builtin::milan, Certificate},
        parser::ByteParser,
    };

    fn milan_fixture() -> (AttestationReport, Chain) {
        // AMD's fixture is a 1,184-byte report encoded as 2,368 hex characters.
        let report_fixture = include_str!("tee_amd_report_milan.hex").trim();
        assert_eq!(report_fixture.len(), 2_368);
        let report_bytes = hex::decode(report_fixture).unwrap();
        let report = AttestationReport::from_bytes(report_bytes.as_slice()).unwrap();
        let vek_bytes = hex::decode(include_str!("tee_amd_vcek_milan.hex").trim()).unwrap();
        let chain = Chain {
            ca: ca::Chain { ark: milan::ark().unwrap(), ask: milan::ask().unwrap() },
            vek: Certificate::from_der(&vek_bytes).unwrap(),
        };
        (report, chain)
    }

    fn fixture_policy(report: &AttestationReport) -> AmdSnpPolicy {
        AmdSnpPolicy {
            expected_measurement: report.measurement,
            expected_report_data: report.report_data,
            minimum_tcb: report_tcb(report),
            allow_smt: report.policy.smt_allowed(),
            allow_migration: report.policy.migrate_ma_allowed(),
        }
    }

    #[test]
    fn verifies_vendor_milan_report_end_to_end() {
        let (report, chain) = milan_fixture();
        let policy = fixture_policy(&report);
        let verifier = AmdSnpVerifier::production_vcek(AmdGeneration::Milan);

        let claims = verifier.verify(&report, &chain, &policy, 1_740_000_000).unwrap();

        assert_eq!(claims.measurement, report.measurement);
        assert_eq!(claims.report_data, report.report_data);
        assert_eq!(claims.chip_id, report.chip_id);
        assert_eq!(claims.verified_at_secs, 1_740_000_000);
    }

    #[test]
    fn rejects_tampered_vendor_report() {
        let (mut report, chain) = milan_fixture();
        let policy = fixture_policy(&report);
        report.measurement[0] ^= 1;
        let verifier = AmdSnpVerifier::production_vcek(AmdGeneration::Milan);

        let error = verifier.verify(&report, &chain, &policy, 1_740_000_000).unwrap_err();

        assert!(error.to_string().contains("signature failed"));
    }

    #[test]
    fn rejects_expired_vcek_at_caller_time() {
        let (report, chain) = milan_fixture();
        let policy = fixture_policy(&report);
        let verifier = AmdSnpVerifier::production_vcek(AmdGeneration::Milan);

        let error = verifier.verify(&report, &chain, &policy, 2_000_000_000).unwrap_err();

        assert!(error.to_string().contains("certificate expired"));
    }

    #[test]
    fn parses_strict_der_u8_values() {
        assert_eq!(parse_der_u8(&[0x02, 0x01, 0x7f]), Some(0x7f));
        assert_eq!(parse_der_u8(&[0x02, 0x02, 0x00, 0x80]), Some(0x80));
        assert_eq!(parse_der_u8(&[0x02, 0x02, 0x00, 0x7f]), None);
        assert_eq!(parse_der_u8(&[0x02, 0x01]), None);
    }

    #[test]
    fn parses_strict_der_octets() {
        assert_eq!(parse_der_octets(&[0x04, 0x03, 1, 2, 3]), Some(&[1, 2, 3][..]));
        assert_eq!(parse_der_octets(&[7; 64]), Some(&[7; 64][..]));
        assert_eq!(parse_der_octets(&[0x04, 0x04, 1, 2, 3]), None);
        assert_eq!(parse_der_octets(&[0x05, 0x03, 1, 2, 3]), None);
    }

    #[test]
    fn tcb_comparison_is_component_wise() {
        let minimum = AmdTcb { bootloader: 2, tee: 0, snp: 8, microcode: 115, fmc: None };
        assert!(ensure_tcb_at_least(minimum, minimum).is_ok());
        let downgraded = AmdTcb { microcode: 114, ..minimum };
        assert!(ensure_tcb_at_least(downgraded, minimum).is_err());
    }

    #[test]
    fn strict_policy_disallows_smt_and_migration() {
        let policy = AmdSnpPolicy::strict(
            [1; 48],
            [2; 64],
            AmdTcb { bootloader: 1, tee: 1, snp: 1, microcode: 1, fmc: None },
        );
        assert!(!policy.allow_smt);
        assert!(!policy.allow_migration);
    }
}
