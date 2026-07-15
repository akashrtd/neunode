//! AMD SEV-SNP VCEK attestation verification.
//!
//! Verification is offline and deterministic: callers supply the report and
//! certificate chain. The chain is anchored to AMD's built-in production ARK/ASK,
//! VCEK extensions are bound to report chip/TCB values, the report signature is
//! verified, and relying-party measurement/challenge policy is applied last.

use der::Decode;
use openssl::{
    asn1::Asn1Time,
    x509::{CrlStatus, X509Crl, X509},
};
use sev::{
    certs::snp::{ca, Certificate, Chain, Verifiable},
    firmware::guest::AttestationReport,
    parser::ByteParser,
    Generation,
};
use sha2::{Digest, Sha384};
use x509_cert::{ext::Extension, Certificate as X509Certificate};

use crate::error::{Result, VerificationError};

const OID_BOOTLOADER: &str = "1.3.6.1.4.1.3704.1.3.1";
const OID_TEE: &str = "1.3.6.1.4.1.3704.1.3.2";
const OID_SNP: &str = "1.3.6.1.4.1.3704.1.3.3";
const OID_UCODE: &str = "1.3.6.1.4.1.3704.1.3.8";
const OID_FMC: &str = "1.3.6.1.4.1.3704.1.3.9";
const OID_HW_ID: &str = "1.3.6.1.4.1.3704.1.4";
const OID_PRODUCT_NAME: &str = "1.3.6.1.4.1.3704.1.2";
const OID_CSP_ID: &str = "1.3.6.1.4.1.3704.1.5";

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
    endorsement: AmdEndorsement,
}

enum AmdEndorsement {
    Vcek,
    Vlek { expected_product_name: String, expected_csp_id: String, crl_der: Vec<u8> },
}

impl AmdSnpVerifier {
    /// Construct a verifier pinned to AMD's built-in production ARK and ASK.
    pub fn production_vcek(generation: AmdGeneration) -> Self {
        Self {
            generation,
            trusted_ca: generation.vendor_generation().into(),
            endorsement: AmdEndorsement::Vcek,
        }
    }

    /// Construct a VLEK verifier with an explicitly pinned AMD VLEK root.
    ///
    /// AMD VLEK uses a distinct ARK → ASVK → VLEK hierarchy. The caller must
    /// provision the expected ARK SHA-384 digest independently from the evidence,
    /// the complete current AMD CRL, and exact CSP/product identities.
    pub fn pinned_vlek(
        generation: AmdGeneration,
        ark_der: &[u8],
        asvk_der: &[u8],
        crl_der: &[u8],
        expected_ark_sha384: [u8; 48],
        expected_product_name: String,
        expected_csp_id: String,
    ) -> Result<Self> {
        if generation == AmdGeneration::Turin {
            return Err(tee_error(
                "AMD VLEK certificate specification does not yet define Turin trust semantics",
            ));
        }
        if expected_product_name.is_empty() || expected_csp_id.is_empty() {
            return Err(tee_error("VLEK product name and CSP identity cannot be empty"));
        }
        let actual_digest: [u8; 48] = Sha384::digest(ark_der).into();
        if actual_digest != expected_ark_sha384 {
            return Err(tee_error(format!(
                "VLEK ARK pin mismatch: expected {}, got {}",
                hex::encode(expected_ark_sha384),
                hex::encode(actual_digest)
            )));
        }
        X509Crl::from_der(crl_der)
            .map_err(|error| tee_error(format!("invalid AMD VLEK CRL: {error}")))?;
        let trusted_ca = ca::Chain {
            ark: Certificate::from_der(ark_der)
                .map_err(|error| tee_error(format!("invalid AMD VLEK ARK: {error}")))?,
            ask: Certificate::from_der(asvk_der)
                .map_err(|error| tee_error(format!("invalid AMD ASVK: {error}")))?,
        };
        Ok(Self {
            generation,
            trusted_ca,
            endorsement: AmdEndorsement::Vlek {
                expected_product_name,
                expected_csp_id,
                crl_der: crl_der.to_vec(),
            },
        })
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
        match &self.endorsement {
            AmdEndorsement::Vcek => verify_vcek_extensions(&chain.vek, report, self.generation)?,
            AmdEndorsement::Vlek { expected_product_name, expected_csp_id, crl_der } => {
                verify_vlek_extensions(&chain.vek, report, expected_product_name, expected_csp_id)?;
                verify_vlek_crl(crl_der, &chain.ca.ark, &chain.ca.ask, &chain.vek, now_secs)?;
            }
        }
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

    /// Decode a raw SNP report and DER certificate chain, then run the complete
    /// production verification path. Evidence decoding failures are fail-closed.
    pub fn verify_der(
        &self,
        raw_report: &[u8],
        ark_der: &[u8],
        ask_der: &[u8],
        vek_der: &[u8],
        policy: &AmdSnpPolicy,
        now_secs: u64,
    ) -> Result<AmdSnpClaims> {
        let report = AttestationReport::from_bytes(raw_report)
            .map_err(|error| tee_error(format!("invalid SEV-SNP report: {error}")))?;
        let chain = Chain {
            ca: ca::Chain {
                ark: Certificate::from_der(ark_der)
                    .map_err(|error| tee_error(format!("invalid AMD ARK certificate: {error}")))?,
                ask: Certificate::from_der(ask_der)
                    .map_err(|error| tee_error(format!("invalid AMD ASK certificate: {error}")))?,
            },
            vek: Certificate::from_der(vek_der)
                .map_err(|error| tee_error(format!("invalid AMD VEK certificate: {error}")))?,
        };
        self.verify(&report, &chain, policy, now_secs)
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

fn verify_vlek_extensions(
    vek: &Certificate,
    report: &AttestationReport,
    expected_product_name: &str,
    expected_csp_id: &str,
) -> Result<()> {
    let der = vek
        .to_der()
        .map_err(|error| tee_error(format!("failed to encode VLEK certificate: {error}")))?;
    let certificate = X509Certificate::from_der(&der)
        .map_err(|error| tee_error(format!("failed to parse VLEK certificate: {error}")))?;

    require_integer_extension(&certificate, OID_BOOTLOADER, report.reported_tcb.bootloader)?;
    require_integer_extension(&certificate, OID_TEE, report.reported_tcb.tee)?;
    require_integer_extension(&certificate, OID_SNP, report.reported_tcb.snp)?;
    require_integer_extension(&certificate, OID_UCODE, report.reported_tcb.microcode)?;
    require_ia5_extension(&certificate, OID_PRODUCT_NAME, expected_product_name)?;
    require_ia5_extension(&certificate, OID_CSP_ID, expected_csp_id)?;
    if extension_optional(&certificate, OID_HW_ID).is_some() {
        return Err(tee_error("VLEK certificate unexpectedly contains a chip HW_ID"));
    }
    Ok(())
}

fn verify_vlek_crl(
    crl_der: &[u8],
    ark: &Certificate,
    asvk: &Certificate,
    vlek: &Certificate,
    now_secs: u64,
) -> Result<()> {
    let crl = X509Crl::from_der(crl_der)
        .map_err(|error| tee_error(format!("invalid AMD VLEK CRL: {error}")))?;
    let ark = openssl_certificate(ark, "VLEK ARK")?;
    let asvk = openssl_certificate(asvk, "ASVK")?;
    let vlek = openssl_certificate(vlek, "VLEK")?;
    let ark_key = ark
        .public_key()
        .map_err(|error| tee_error(format!("failed to read VLEK ARK public key: {error}")))?;
    if !crl
        .verify(&ark_key)
        .map_err(|error| tee_error(format!("failed to verify AMD VLEK CRL: {error}")))?
    {
        return Err(tee_error("AMD VLEK CRL signature is invalid"));
    }

    let now = Asn1Time::from_unix(
        i64::try_from(now_secs).map_err(|_| tee_error("verification time exceeds i64"))?,
    )
    .map_err(|error| tee_error(format!("invalid VLEK verification time: {error}")))?;
    if crl
        .last_update()
        .compare(&now)
        .map_err(|error| tee_error(format!("failed to compare VLEK CRL time: {error}")))?
        .is_gt()
    {
        return Err(tee_error("AMD VLEK CRL is not valid yet"));
    }
    let next_update =
        crl.next_update().ok_or_else(|| tee_error("AMD VLEK CRL does not contain nextUpdate"))?;
    if next_update
        .compare(&now)
        .map_err(|error| tee_error(format!("failed to compare VLEK CRL expiry: {error}")))?
        .is_lt()
    {
        return Err(tee_error("AMD VLEK CRL has expired"));
    }
    for (label, certificate) in [("ASVK", &asvk), ("VLEK", &vlek)] {
        if !matches!(crl.get_by_cert(certificate), CrlStatus::NotRevoked) {
            return Err(tee_error(format!("AMD {label} certificate is revoked")));
        }
    }
    Ok(())
}

fn openssl_certificate(certificate: &Certificate, label: &str) -> Result<X509> {
    let der = certificate
        .to_der()
        .map_err(|error| tee_error(format!("failed to encode {label}: {error}")))?;
    X509::from_der(&der).map_err(|error| tee_error(format!("failed to parse {label}: {error}")))
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
    extension_optional(certificate, oid)
        .ok_or_else(|| tee_error(format!("VCEK is missing required extension {oid}")))
}

fn extension_optional<'a>(certificate: &'a X509Certificate, oid: &str) -> Option<&'a Extension> {
    certificate.tbs_certificate.extensions.as_ref().and_then(|extensions| {
        extensions.iter().find(|extension| extension.extn_id.to_string() == oid)
    })
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

fn require_ia5_extension(certificate: &X509Certificate, oid: &str, expected: &str) -> Result<()> {
    let value = extension(certificate, oid)?.extn_value.as_bytes();
    let actual = parse_der_ia5(value)
        .ok_or_else(|| tee_error(format!("invalid VLEK IA5String extension {oid}")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(tee_error(format!(
            "VLEK extension {oid} mismatch: expected '{expected}', got '{actual}'"
        )))
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

fn parse_der_ia5(value: &[u8]) -> Option<&str> {
    let [0x16, length, bytes @ ..] = value else {
        return None;
    };
    if usize::from(*length) != bytes.len() || !bytes.is_ascii() {
        return None;
    }
    std::str::from_utf8(bytes).ok()
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

    fn milan_vlek_ca() -> (Vec<u8>, Vec<u8>, Vec<u8>, [u8; 48]) {
        let certificates =
            X509::stack_from_pem(include_bytes!("tee_amd_vlek_milan_chain.pem")).unwrap();
        assert_eq!(certificates.len(), 2);
        let asvk_der = certificates[0].to_der().unwrap();
        let ark_der = certificates[1].to_der().unwrap();
        let crl_der = hex::decode(include_str!("tee_amd_vlek_milan_crl.hex").trim()).unwrap();
        let pin = Sha384::digest(&ark_der).into();
        (ark_der, asvk_der, crl_der, pin)
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
    fn decodes_and_verifies_raw_vendor_milan_evidence() {
        let (report, chain) = milan_fixture();
        let policy = fixture_policy(&report);
        let raw_report = report.to_bytes().unwrap();
        let verifier = AmdSnpVerifier::production_vcek(AmdGeneration::Milan);

        let claims = verifier
            .verify_der(
                raw_report.as_ref(),
                &chain.ca.ark.to_der().unwrap(),
                &chain.ca.ask.to_der().unwrap(),
                &chain.vek.to_der().unwrap(),
                &policy,
                1_740_000_000,
            )
            .unwrap();

        assert_eq!(claims.measurement, report.measurement);
    }

    #[test]
    fn accepts_pinned_amd_milan_vlek_ca_and_crl() {
        let (ark_der, asvk_der, crl_der, pin) = milan_vlek_ca();

        let verifier = AmdSnpVerifier::pinned_vlek(
            AmdGeneration::Milan,
            &ark_der,
            &asvk_der,
            &crl_der,
            pin,
            "Milan-B0".to_string(),
            "cloud.example".to_string(),
        )
        .unwrap();

        assert!(matches!(verifier.endorsement, AmdEndorsement::Vlek { .. }));
    }

    #[test]
    fn rejects_unpinned_vlek_root() {
        let (ark_der, asvk_der, crl_der, mut pin) = milan_vlek_ca();
        pin[0] ^= 1;

        let error = AmdSnpVerifier::pinned_vlek(
            AmdGeneration::Milan,
            &ark_der,
            &asvk_der,
            &crl_der,
            pin,
            "Milan-B0".to_string(),
            "cloud.example".to_string(),
        )
        .err()
        .unwrap();

        assert!(error.to_string().contains("ARK pin mismatch"));
    }

    #[test]
    fn validates_signed_and_current_vlek_crl() {
        let (ark_der, asvk_der, crl_der, _) = milan_vlek_ca();
        let ark = Certificate::from_der(&ark_der).unwrap();
        let asvk = Certificate::from_der(&asvk_der).unwrap();

        verify_vlek_crl(&crl_der, &ark, &asvk, &asvk, 1_783_000_000).unwrap();
        let error = verify_vlek_crl(&crl_der, &ark, &asvk, &asvk, 1_800_000_000).unwrap_err();
        assert!(error.to_string().contains("CRL has expired"));
    }

    #[test]
    fn rejects_tampered_vlek_crl_signature() {
        let (ark_der, asvk_der, mut crl_der, _) = milan_vlek_ca();
        let ark = Certificate::from_der(&ark_der).unwrap();
        let asvk = Certificate::from_der(&asvk_der).unwrap();
        *crl_der.last_mut().unwrap() ^= 1;

        let error = verify_vlek_crl(&crl_der, &ark, &asvk, &asvk, 1_783_000_000).unwrap_err();

        assert!(error.to_string().contains("CRL signature is invalid"));
    }

    #[test]
    fn rejects_turin_vlek_until_amd_defines_its_certificate_profile() {
        let (ark_der, asvk_der, crl_der, pin) = milan_vlek_ca();

        let error = AmdSnpVerifier::pinned_vlek(
            AmdGeneration::Turin,
            &ark_der,
            &asvk_der,
            &crl_der,
            pin,
            "Turin-A0".to_string(),
            "cloud.example".to_string(),
        )
        .err()
        .unwrap();

        assert!(error.to_string().contains("does not yet define Turin"));
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
    fn parses_strict_der_ia5_strings() {
        assert_eq!(parse_der_ia5(&[0x16, 0x05, b'M', b'i', b'l', b'a', b'n']), Some("Milan"));
        assert_eq!(parse_der_ia5(&[0x16, 0x04, b'M', b'i', b'l', b'a', b'n']), None);
        assert_eq!(parse_der_ia5(&[0x0c, 0x05, b'M', b'i', b'l', b'a', b'n']), None);
        assert_eq!(parse_der_ia5(&[0x16, 0x01, 0xff]), None);
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
