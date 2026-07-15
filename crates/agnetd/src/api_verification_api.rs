use axum::response::IntoResponse;
use axum::Json;
use neunode_verification::tee_amd::{AmdGeneration, AmdSnpPolicy, AmdSnpVerifier, AmdTcb};
use neunode_verification::tee_intel::{IntelTdxPolicy, IntelTdxVerifier};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::error::ApiError;
use crate::api::types;

#[derive(Debug, Deserialize, ToSchema)]
pub struct IntelTdxVerifyRequest {
    /// Raw TDX quote encoded as hexadecimal.
    pub quote_hex: String,
    /// Complete Intel DCAP QuoteCollateralV3 JSON.
    pub collateral_json: String,
    pub mr_td: String,
    pub report_data: String,
    pub now_secs: Option<u64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AmdPolicyRequest {
    pub measurement: String,
    pub report_data: String,
    pub minimum_tcb: AmdTcbRequest,
    #[serde(default)]
    pub allow_smt: bool,
    #[serde(default)]
    pub allow_migration: bool,
    pub now_secs: Option<u64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AmdTcbRequest {
    pub bootloader: u8,
    pub tee: u8,
    pub snp: u8,
    pub microcode: u8,
    pub fmc: Option<u8>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AmdGenerationRequest {
    Milan,
    Genoa,
    Turin,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AmdSnpVerifyRequest {
    pub report_hex: String,
    pub ark_hex: String,
    pub ask_hex: String,
    pub vek_hex: String,
    pub generation: AmdGenerationRequest,
    #[serde(flatten)]
    pub policy: AmdPolicyRequest,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AmdVlekVerifyRequest {
    pub report_hex: String,
    pub ark_hex: String,
    pub asvk_hex: String,
    pub vlek_hex: String,
    pub crl_hex: String,
    pub ark_sha384: String,
    pub product_name: String,
    pub csp_id: String,
    pub generation: AmdGenerationRequest,
    #[serde(flatten)]
    pub policy: AmdPolicyRequest,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IntelTdxVerifyResponse {
    pub verified: bool,
    pub tee_type: String,
    pub claims: IntelTdxClaimsResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IntelTdxClaimsResponse {
    pub mr_td: Vec<u8>,
    pub report_data: Vec<u8>,
    pub tcb_status: String,
    pub advisory_ids: Vec<String>,
    pub verified_at_secs: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AmdSnpVerifyResponse {
    pub verified: bool,
    pub tee_type: String,
    pub claims: AmdSnpClaimsResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AmdVlekVerifyResponse {
    pub verified: bool,
    pub tee_type: String,
    pub claims: AmdSnpClaimsResponse,
    pub product_name: String,
    pub csp_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AmdSnpClaimsResponse {
    pub generation: String,
    pub measurement: Vec<u8>,
    pub report_data: Vec<u8>,
    pub chip_id: Vec<u8>,
    pub reported_tcb: AmdTcbClaimsResponse,
    pub guest_svn: u32,
    pub vmpl: u32,
    pub verified_at_secs: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AmdTcbClaimsResponse {
    pub bootloader: u8,
    pub tee: u8,
    pub snp: u8,
    pub microcode: u8,
    pub fmc: Option<u8>,
}

#[utoipa::path(
    post,
    path = "/api/v1/verification/tee/intel-tdx",
    request_body = IntelTdxVerifyRequest,
    responses((status = 200, body = IntelTdxVerifyResponse), (status = 400, description = "Invalid evidence or failed verification")),
    tag = "verification",
)]
pub async fn verify_intel_tdx(
    Json(body): Json<IntelTdxVerifyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let quote = decode_hex(&body.quote_hex, "quote")?;
    let policy = IntelTdxPolicy::strict(
        decode_fixed_hex(&body.mr_td, "mr_td")?,
        decode_fixed_hex(&body.report_data, "report_data")?,
    );
    let claims = IntelTdxVerifier::production()
        .verify_json(&quote, body.collateral_json.as_bytes(), &policy, trusted_time(body.now_secs)?)
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(types::ok(IntelTdxVerifyResponse {
        verified: true,
        tee_type: "intel_tdx".to_string(),
        claims: intel_claims(claims),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/verification/tee/amd-snp",
    request_body = AmdSnpVerifyRequest,
    responses((status = 200, body = AmdSnpVerifyResponse), (status = 400, description = "Invalid evidence or failed verification")),
    tag = "verification",
)]
pub async fn verify_amd_snp(
    Json(body): Json<AmdSnpVerifyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let generation = amd_generation(body.generation);
    if generation == AmdGeneration::Turin && body.policy.minimum_tcb.fmc.is_none() {
        return Err(ApiError::BadRequest("minimum_tcb.fmc is required for AMD Turin".into()));
    }
    let policy = amd_policy(&body.policy)?;
    let claims = AmdSnpVerifier::production_vcek(generation)
        .verify_der(
            &decode_hex(&body.report_hex, "report")?,
            &decode_hex(&body.ark_hex, "ark")?,
            &decode_hex(&body.ask_hex, "ask")?,
            &decode_hex(&body.vek_hex, "vek")?,
            &policy,
            trusted_time(body.policy.now_secs)?,
        )
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(types::ok(AmdSnpVerifyResponse {
        verified: true,
        tee_type: "amd_sev_snp".to_string(),
        claims: amd_claims(claims),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/verification/tee/amd-vlek",
    request_body = AmdVlekVerifyRequest,
    responses((status = 200, body = AmdVlekVerifyResponse), (status = 400, description = "Invalid evidence or failed verification")),
    tag = "verification",
)]
pub async fn verify_amd_vlek(
    Json(body): Json<AmdVlekVerifyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.product_name.trim().is_empty() || body.csp_id.trim().is_empty() {
        return Err(ApiError::BadRequest("product_name and csp_id cannot be empty".into()));
    }
    let ark = decode_hex(&body.ark_hex, "ark")?;
    let asvk = decode_hex(&body.asvk_hex, "asvk")?;
    let crl = decode_hex(&body.crl_hex, "crl")?;
    let verifier = AmdSnpVerifier::pinned_vlek(
        amd_generation(body.generation),
        &ark,
        &asvk,
        &crl,
        decode_fixed_hex(&body.ark_sha384, "ark_sha384")?,
        body.product_name.clone(),
        body.csp_id.clone(),
    )
    .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let claims = verifier
        .verify_der(
            &decode_hex(&body.report_hex, "report")?,
            &ark,
            &asvk,
            &decode_hex(&body.vlek_hex, "vlek")?,
            &amd_policy(&body.policy)?,
            trusted_time(body.policy.now_secs)?,
        )
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(types::ok(AmdVlekVerifyResponse {
        verified: true,
        tee_type: "amd_sev_snp_vlek".to_string(),
        claims: amd_claims(claims),
        product_name: body.product_name,
        csp_id: body.csp_id,
    }))
}

fn decode_hex(value: &str, label: &str) -> Result<Vec<u8>, ApiError> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} cannot be empty")));
    }
    hex::decode(value)
        .map_err(|error| ApiError::BadRequest(format!("invalid {label} hex: {error}")))
}

fn decode_fixed_hex<const N: usize>(value: &str, label: &str) -> Result<[u8; N], ApiError> {
    let decoded = decode_hex(value, label)?;
    let length = decoded.len();
    decoded.try_into().map_err(|_| {
        ApiError::BadRequest(format!("{label} must be exactly {N} bytes, got {length}"))
    })
}

fn amd_policy(body: &AmdPolicyRequest) -> Result<AmdSnpPolicy, ApiError> {
    let mut policy = AmdSnpPolicy::strict(
        decode_fixed_hex(&body.measurement, "measurement")?,
        decode_fixed_hex(&body.report_data, "report_data")?,
        AmdTcb {
            bootloader: body.minimum_tcb.bootloader,
            tee: body.minimum_tcb.tee,
            snp: body.minimum_tcb.snp,
            microcode: body.minimum_tcb.microcode,
            fmc: body.minimum_tcb.fmc,
        },
    );
    policy.allow_smt = body.allow_smt;
    policy.allow_migration = body.allow_migration;
    Ok(policy)
}

fn amd_generation(value: AmdGenerationRequest) -> AmdGeneration {
    match value {
        AmdGenerationRequest::Milan => AmdGeneration::Milan,
        AmdGenerationRequest::Genoa => AmdGeneration::Genoa,
        AmdGenerationRequest::Turin => AmdGeneration::Turin,
    }
}

fn intel_claims(claims: neunode_verification::tee_intel::IntelTdxClaims) -> IntelTdxClaimsResponse {
    IntelTdxClaimsResponse {
        mr_td: claims.mr_td,
        report_data: claims.report_data,
        tcb_status: claims.tcb_status,
        advisory_ids: claims.advisory_ids,
        verified_at_secs: claims.verified_at_secs,
    }
}

fn amd_claims(claims: neunode_verification::tee_amd::AmdSnpClaims) -> AmdSnpClaimsResponse {
    AmdSnpClaimsResponse {
        generation: match claims.generation {
            AmdGeneration::Milan => "milan",
            AmdGeneration::Genoa => "genoa",
            AmdGeneration::Turin => "turin",
        }
        .to_string(),
        measurement: claims.measurement,
        report_data: claims.report_data,
        chip_id: claims.chip_id,
        reported_tcb: AmdTcbClaimsResponse {
            bootloader: claims.reported_tcb.bootloader,
            tee: claims.reported_tcb.tee,
            snp: claims.reported_tcb.snp,
            microcode: claims.reported_tcb.microcode,
            fmc: claims.reported_tcb.fmc,
        },
        guest_svn: claims.guest_svn,
        vmpl: claims.vmpl,
        verified_at_secs: claims.verified_at_secs,
    }
}

fn trusted_time(explicit: Option<u64>) -> Result<u64, ApiError> {
    explicit.map_or_else(
        || {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .map_err(|error| ApiError::Internal(format!("system clock is invalid: {error}")))
        },
        Ok,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_hex_is_exact_and_accepts_prefix() {
        assert_eq!(decode_fixed_hex::<2>("0x0102", "value").unwrap(), [1, 2]);
        assert!(decode_fixed_hex::<2>("01", "value").unwrap_err().message().contains("2 bytes"));
        assert!(decode_fixed_hex::<2>("zzzz", "value").is_err());
    }

    #[test]
    fn amd_policy_defaults_remain_fail_closed() {
        let request: AmdPolicyRequest = serde_json::from_value(serde_json::json!({
            "measurement": "11".repeat(48),
            "report_data": "22".repeat(64),
            "minimum_tcb": { "bootloader": 1, "tee": 2, "snp": 3, "microcode": 4 }
        }))
        .unwrap();
        let policy = amd_policy(&request).unwrap();
        assert!(!policy.allow_smt);
        assert!(!policy.allow_migration);
    }
}
