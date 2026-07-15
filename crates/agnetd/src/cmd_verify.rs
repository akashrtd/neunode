use anyhow::Result;
use neunode_verification::bisection::BisectionSolver;
use neunode_verification::gauntlet::{Gauntlet, GauntletConfig, GauntletTest};
use neunode_verification::repops::{DeterministicExecutor, RepOpsResult};
use neunode_verification::spot_check::{SpotCheckConfig, SpotChecker};
use neunode_verification::tee_amd::{AmdGeneration, AmdSnpPolicy, AmdSnpVerifier, AmdTcb};
use neunode_verification::tee_intel::{IntelTdxPolicy, IntelTdxVerifier};

use crate::cli::{
    AmdGenerationArg, AmdVerificationPolicyArgs, GlobalArgs, TeeVerifyCommands, VerifyCommands,
};
use crate::output::OutputWriter;
use crate::state::AppState;

pub fn execute(cmd: &VerifyCommands, args: &GlobalArgs, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(args.output);
    match cmd {
        VerifyCommands::Gauntlet { test_name, input_hash, expected_hash } => {
            run_gauntlet(test_name, input_hash, expected_hash, &writer, state)
        }
        VerifyCommands::SpotCheck { original, recomputed } => {
            run_spot_check(original, recomputed, &writer, state)
        }
        VerifyCommands::Repops { hashes_a, hashes_b } => {
            run_repops(hashes_a, hashes_b, &writer, state)
        }
        VerifyCommands::Bisection { claimant, challenger } => {
            run_bisection(claimant, challenger, &writer, state)
        }
        VerifyCommands::Tee { command } => verify_tee(command, &writer, state),
        VerifyCommands::Status => show_status(&writer, state),
    }
}

// --- Subcommand handlers ---

fn run_gauntlet(
    test_name: &str,
    input_hash: &str,
    expected_hash: &str,
    writer: &OutputWriter,
    _state: &AppState,
) -> Result<()> {
    if test_name.is_empty() {
        anyhow::bail!("test name cannot be empty");
    }
    if input_hash.is_empty() {
        anyhow::bail!("input hash cannot be empty");
    }
    if expected_hash.is_empty() {
        anyhow::bail!("expected hash cannot be empty");
    }

    let gauntlet = Gauntlet::new(GauntletConfig::default());
    let test = GauntletTest {
        name: test_name.to_string(),
        input_hash: input_hash.to_string(),
        expected_output_hash: expected_hash.to_string(),
        injected: false,
        difficulty: 5,
    };

    let result = gauntlet.verify(&test, input_hash)?;

    let out = serde_json::json!({
        "test_name": test_name,
        "passed": result.passed,
        "layer": format!("{:?}", result.layer),
        "confidence": result.confidence,
        "evidence_hash": result.evidence_hash,
        "verifier_id": result.verifier_id,
        "timestamp_ms": result.timestamp_ms,
        "details": result.details,
    });

    writer.write_json(&out);
    if result.passed {
        writer.write_status(&format!("Gauntlet test '{test_name}' passed"));
    }
    Ok(())
}

fn run_spot_check(
    original_path: &str,
    recomputed_path: &str,
    writer: &OutputWriter,
    _state: &AppState,
) -> Result<()> {
    if original_path.is_empty() {
        anyhow::bail!("original file path cannot be empty");
    }
    if recomputed_path.is_empty() {
        anyhow::bail!("recomputed file path cannot be empty");
    }

    let original_bytes = std::fs::read(original_path)
        .map_err(|e| anyhow::anyhow!("failed to read original file '{}': {e}", original_path))?;
    let recomputed_bytes = std::fs::read(recomputed_path).map_err(|e| {
        anyhow::anyhow!("failed to read recomputed file '{}': {e}", recomputed_path)
    })?;

    let checker = SpotChecker::new(SpotCheckConfig::default());
    let result = checker.verify_output(&original_bytes, &recomputed_bytes);

    let out = serde_json::json!({
        "original_hash": result.original_hash,
        "recomputed_hash": result.recomputed_hash,
        "match": result.match_result,
        "retries_used": result.retries_used,
        "elapsed_ms": result.elapsed_ms,
    });

    writer.write_json(&out);
    if result.match_result {
        writer.write_status("Spot check passed: hashes match");
    } else {
        writer.write_error("Spot check failed: hash mismatch");
    }
    Ok(())
}

fn run_repops(
    hashes_a_str: &str,
    hashes_b_str: &str,
    writer: &OutputWriter,
    _state: &AppState,
) -> Result<()> {
    if hashes_a_str.is_empty() {
        anyhow::bail!("hashes_a cannot be empty");
    }
    if hashes_b_str.is_empty() {
        anyhow::bail!("hashes_b cannot be empty");
    }

    let hashes_a = parse_comma_hashes(hashes_a_str)?;
    let hashes_b = parse_comma_hashes(hashes_b_str)?;

    let result_a = build_repops_result(&hashes_a);
    let result_b = build_repops_result(&hashes_b);

    let matches = DeterministicExecutor::compare(&result_a, &result_b);

    let out = serde_json::json!({
        "matches": matches,
        "hashes_a_count": hashes_a.len(),
        "hashes_b_count": hashes_b.len(),
        "output_hash_a": result_a.output_hash,
        "output_hash_b": result_b.output_hash,
        "intermediate_count_a": result_a.intermediate_hashes.len(),
        "intermediate_count_b": result_b.intermediate_hashes.len(),
    });

    writer.write_json(&out);
    if matches {
        writer.write_status("RepOps comparison: executions match");
    } else {
        writer.write_error("RepOps comparison: executions differ");
    }
    Ok(())
}

fn run_bisection(
    claimant_str: &str,
    challenger_str: &str,
    writer: &OutputWriter,
    _state: &AppState,
) -> Result<()> {
    if claimant_str.is_empty() {
        anyhow::bail!("claimant hashes cannot be empty");
    }
    if challenger_str.is_empty() {
        anyhow::bail!("challenger hashes cannot be empty");
    }

    let claimant = parse_comma_hashes(claimant_str)?;
    let challenger = parse_comma_hashes(challenger_str)?;

    let solver = BisectionSolver::new();
    let result = solver.solve(&claimant, &challenger, |idx| {
        claimant.get(idx as usize) == challenger.get(idx as usize)
    });

    let out = serde_json::json!({
        "found": result.found,
        "disagreeing_op_index": result.disagreeing_op_index,
        "steps_taken": result.steps_taken,
        "total_ops": result.total_ops,
        "claimant_hash": result.claimant_hash,
        "challenger_hash": result.challenger_hash,
    });

    writer.write_json(&out);
    if result.found {
        let idx = result.disagreeing_op_index.unwrap_or(0);
        writer.write_status(&format!("Bisection: first disagreement at op {idx}"));
    } else if result.total_ops > 0 {
        writer.write_status("Bisection: all operations match");
    } else {
        writer.write_warning("Bisection: no operations to compare");
    }
    Ok(())
}

fn verify_tee(command: &TeeVerifyCommands, writer: &OutputWriter, _state: &AppState) -> Result<()> {
    match command {
        TeeVerifyCommands::Intel { quote, collateral, mr_td, report_data, now_secs } => {
            let raw_quote = read_evidence(quote, "Intel TDX quote")?;
            let collateral_json = read_evidence(collateral, "Intel DCAP collateral")?;
            let policy = IntelTdxPolicy::strict(
                decode_fixed_hex(mr_td, "MR_TD")?,
                decode_fixed_hex(report_data, "REPORT_DATA")?,
            );
            let verified_at = trusted_time(*now_secs)?;
            let claims = IntelTdxVerifier::production().verify_json(
                &raw_quote,
                &collateral_json,
                &policy,
                verified_at,
            )?;
            writer.write_json(&serde_json::json!({
                "verified": true,
                "tee_type": "intel_tdx",
                "claims": claims,
            }));
            writer.write_status("TEE attestation verified (Intel TDX)");
        }
        TeeVerifyCommands::Amd { report, ark, ask, vek, generation, policy, now_secs } => {
            let generation = amd_generation(*generation);
            if generation == AmdGeneration::Turin && policy.min_fmc.is_none() {
                anyhow::bail!("--min-fmc is required for AMD Turin verification");
            }
            let policy = amd_policy(policy)?;
            let verified_at = trusted_time(*now_secs)?;
            let claims = AmdSnpVerifier::production_vcek(generation).verify_der(
                &read_evidence(report, "AMD SEV-SNP report")?,
                &read_evidence(ark, "AMD ARK certificate")?,
                &read_evidence(ask, "AMD ASK certificate")?,
                &read_evidence(vek, "AMD VEK certificate")?,
                &policy,
                verified_at,
            )?;
            writer.write_json(&serde_json::json!({
                "verified": true,
                "tee_type": "amd_sev_snp",
                "claims": claims,
            }));
            writer.write_status("TEE attestation verified (AMD SEV-SNP)");
        }
        TeeVerifyCommands::AmdVlek {
            report,
            ark,
            asvk,
            vlek,
            crl,
            ark_sha384,
            product_name,
            csp_id,
            generation,
            policy,
            now_secs,
        } => {
            let generation = amd_generation(*generation);
            let raw_ark = read_evidence(ark, "AMD VLEK ARK certificate")?;
            let raw_asvk = read_evidence(asvk, "AMD ASVK certificate")?;
            let raw_crl = read_evidence(crl, "AMD VLEK CRL")?;
            let verifier = AmdSnpVerifier::pinned_vlek(
                generation,
                &raw_ark,
                &raw_asvk,
                &raw_crl,
                decode_fixed_hex(ark_sha384, "ARK SHA-384")?,
                product_name.clone(),
                csp_id.clone(),
            )?;
            let claims = verifier.verify_der(
                &read_evidence(report, "AMD SEV-SNP report")?,
                &raw_ark,
                &raw_asvk,
                &read_evidence(vlek, "AMD VLEK certificate")?,
                &amd_policy(policy)?,
                trusted_time(*now_secs)?,
            )?;
            writer.write_json(&serde_json::json!({
                "verified": true,
                "tee_type": "amd_sev_snp_vlek",
                "claims": claims,
                "product_name": product_name,
                "csp_id": csp_id,
            }));
            writer.write_status("TEE attestation verified (AMD SEV-SNP VLEK)");
        }
    }
    Ok(())
}

fn read_evidence(path: &str, label: &str) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|error| anyhow::anyhow!("failed to read {label} '{path}': {error}"))
}

fn amd_policy(args: &AmdVerificationPolicyArgs) -> Result<AmdSnpPolicy> {
    let mut policy = AmdSnpPolicy::strict(
        decode_fixed_hex(&args.measurement, "measurement")?,
        decode_fixed_hex(&args.report_data, "REPORT_DATA")?,
        AmdTcb {
            bootloader: args.min_bootloader,
            tee: args.min_tee,
            snp: args.min_snp,
            microcode: args.min_microcode,
            fmc: args.min_fmc,
        },
    );
    policy.allow_smt = args.allow_smt;
    policy.allow_migration = args.allow_migration;
    Ok(policy)
}

fn decode_fixed_hex<const N: usize>(value: &str, label: &str) -> Result<[u8; N]> {
    let decoded = hex::decode(value.strip_prefix("0x").unwrap_or(value))
        .map_err(|error| anyhow::anyhow!("invalid {label} hex: {error}"))?;
    decoded.try_into().map_err(|bytes: Vec<u8>| {
        anyhow::anyhow!("{label} must be exactly {N} bytes, got {}", bytes.len())
    })
}

fn trusted_time(explicit: Option<u64>) -> Result<u64> {
    explicit.map_or_else(
        || {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .map_err(|error| anyhow::anyhow!("system clock is before Unix epoch: {error}"))
        },
        Ok,
    )
}

fn amd_generation(generation: AmdGenerationArg) -> AmdGeneration {
    match generation {
        AmdGenerationArg::Milan => AmdGeneration::Milan,
        AmdGenerationArg::Genoa => AmdGeneration::Genoa,
        AmdGenerationArg::Turin => AmdGeneration::Turin,
    }
}

fn show_status(writer: &OutputWriter, _state: &AppState) -> Result<()> {
    let layers = [
        ("Automated", "Hash comparison, format validation", "enabled"),
        ("RepOps", "Bitwise reproducibility check", "enabled"),
        ("PeerReview", "2-of-3 reviewer committee", "enabled"),
        ("Bisection", "Verde-style dispute resolution", "enabled"),
        ("TEE", "Vendor-backed trusted execution proof", "unavailable"),
        ("ZK", "Zero-knowledge proof (placeholder)", "disabled"),
        ("Arbitration", "Kleros-style final arbitration", "disabled"),
    ];

    let headers = ["Layer", "Description", "Status"];
    let rows: Vec<Vec<String>> = layers
        .iter()
        .map(|(name, desc, status)| vec![name.to_string(), desc.to_string(), status.to_string()])
        .collect();

    writer.write_table(&headers, &rows);
    Ok(())
}

// --- Helpers ---

fn parse_comma_hashes(s: &str) -> Result<Vec<String>> {
    let hashes: Vec<String> =
        s.split(',').map(|h| h.trim().to_string()).filter(|h| !h.is_empty()).collect();
    if hashes.is_empty() {
        anyhow::bail!("no hashes found in input");
    }
    Ok(hashes)
}

fn build_repops_result(hashes: &[String]) -> RepOpsResult {
    if hashes.len() == 1 {
        RepOpsResult {
            output_hash: hashes[0].clone(),
            intermediate_hashes: vec![],
            op_count: 1,
            hash_count: 0,
            reproducible: true,
        }
    } else {
        let output_hash = hashes[hashes.len() - 1].clone();
        let intermediate_hashes = hashes[..hashes.len() - 1].to_vec();
        RepOpsResult {
            output_hash,
            intermediate_hashes,
            op_count: hashes.len() as u32,
            hash_count: (hashes.len() - 1) as u32,
            reproducible: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{human_writer, json_writer as test_writer, test_state};

    // --- Gauntlet tests ---

    #[test]
    fn gauntlet_matching_hashes_passes() {
        let state = test_state();
        let writer = test_writer();
        let result = run_gauntlet("test_match", "abc123", "abc123", &writer, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn gauntlet_mismatching_hashes_fails() {
        let state = test_state();
        let writer = test_writer();
        let result = run_gauntlet("test_mismatch", "abc123", "def456", &writer, &state);
        assert!(result.is_err());
    }

    #[test]
    fn gauntlet_empty_test_name_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(run_gauntlet("", "abc", "def", &writer, &state).is_err());
    }

    #[test]
    fn gauntlet_empty_input_hash_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(run_gauntlet("test", "", "def", &writer, &state).is_err());
    }

    #[test]
    fn gauntlet_empty_expected_hash_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(run_gauntlet("test", "abc", "", &writer, &state).is_err());
    }

    // --- SpotCheck tests ---

    #[test]
    fn spot_check_matching_files() {
        let state = test_state();
        let writer = test_writer();
        let dir = std::env::temp_dir().join("agnetd_verify_spot_match");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let data = b"hello world";
        let path_a = dir.join("original.bin");
        let path_b = dir.join("recomputed.bin");
        std::fs::write(&path_a, data).unwrap();
        std::fs::write(&path_b, data).unwrap();

        let result =
            run_spot_check(path_a.to_str().unwrap(), path_b.to_str().unwrap(), &writer, &state);
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn spot_check_different_files() {
        let state = test_state();
        let writer = test_writer();
        let dir = std::env::temp_dir().join("agnetd_verify_spot_diff");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let path_a = dir.join("original.bin");
        let path_b = dir.join("recomputed.bin");
        std::fs::write(&path_a, b"hello").unwrap();
        std::fs::write(&path_b, b"world").unwrap();

        let result =
            run_spot_check(path_a.to_str().unwrap(), path_b.to_str().unwrap(), &writer, &state);
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn spot_check_missing_original_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(run_spot_check(
            "/nonexistent/path/a.bin",
            "/nonexistent/path/b.bin",
            &writer,
            &state,
        )
        .is_err());
    }

    #[test]
    fn spot_check_empty_original_path_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(run_spot_check("", "/tmp/b.bin", &writer, &state).is_err());
    }

    #[test]
    fn spot_check_empty_recomputed_path_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(run_spot_check("/tmp/a.bin", "", &writer, &state).is_err());
    }

    // --- RepOps tests ---

    #[test]
    fn repops_matching_hashes() {
        let state = test_state();
        let writer = test_writer();
        let hashes = "h1,h2,h3";
        let result = run_repops(hashes, hashes, &writer, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn repops_different_hashes() {
        let state = test_state();
        let writer = test_writer();
        let result = run_repops("a,b,c", "a,b,x", &writer, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn repops_single_hash() {
        let state = test_state();
        let writer = test_writer();
        let result = run_repops("only_one", "only_one", &writer, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn repops_empty_a_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(run_repops("", "a,b", &writer, &state).is_err());
    }

    #[test]
    fn repops_empty_b_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(run_repops("a,b", "", &writer, &state).is_err());
    }

    // --- Bisection tests ---

    #[test]
    fn bisection_all_match() {
        let state = test_state();
        let writer = test_writer();
        let hashes = "h0,h1,h2,h3";
        let result = run_bisection(hashes, hashes, &writer, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn bisection_find_first_disagreement() {
        let state = test_state();
        let writer = test_writer();
        let result = run_bisection("a,b,c,d,e,f,g,h", "a,b,c,X,e,f,g,h", &writer, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn bisection_single_op_disagree() {
        let state = test_state();
        let writer = test_writer();
        let result = run_bisection("good", "bad", &writer, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn bisection_empty_claimant_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(run_bisection("", "a,b", &writer, &state).is_err());
    }

    #[test]
    fn bisection_empty_challenger_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(run_bisection("a,b", "", &writer, &state).is_err());
    }

    // --- TEE tests ---

    #[test]
    fn tee_policy_hex_is_exact_length() {
        assert_eq!(decode_fixed_hex::<2>("0x0102", "test").unwrap(), [1, 2]);
        assert!(decode_fixed_hex::<2>("01", "test").is_err());
        assert!(decode_fixed_hex::<2>("zzzz", "test").is_err());
    }

    // --- Status tests ---

    #[test]
    fn status_displays_layers() {
        let state = test_state();
        let writer = human_writer();
        let result = show_status(&writer, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn status_json_format() {
        let state = test_state();
        let writer = test_writer();
        let result = show_status(&writer, &state);
        assert!(result.is_ok());
    }

    // --- Unit tests for helpers ---

    #[test]
    fn parse_comma_hashes_valid() {
        let result = parse_comma_hashes("a,b,c").unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_comma_hashes_trims_whitespace() {
        let result = parse_comma_hashes(" a , b , c ").unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_comma_hashes_single() {
        let result = parse_comma_hashes("only").unwrap();
        assert_eq!(result, vec!["only"]);
    }

    #[test]
    fn parse_comma_hashes_empty_fails() {
        assert!(parse_comma_hashes("").is_err());
    }

    #[test]
    fn parse_comma_hashes_only_commas_fails() {
        assert!(parse_comma_hashes(",,,,").is_err());
    }

    #[test]
    fn build_repops_result_single_hash() {
        let result = build_repops_result(&["abc".to_string()]);
        assert_eq!(result.output_hash, "abc");
        assert!(result.intermediate_hashes.is_empty());
        assert_eq!(result.op_count, 1);
    }

    #[test]
    fn build_repops_result_multiple_hashes() {
        let result = build_repops_result(&["h1".to_string(), "h2".to_string(), "h3".to_string()]);
        assert_eq!(result.output_hash, "h3");
        assert_eq!(result.intermediate_hashes, vec!["h1", "h2"]);
        assert_eq!(result.op_count, 3);
    }
}
