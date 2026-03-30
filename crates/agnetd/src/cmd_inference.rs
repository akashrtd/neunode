use anyhow::Result;
use neunode_core::types::TokenAmount;
use neunode_inference::openai::{ChatCompletionRequest, ChatMessage, MessageRole};
use neunode_inference::provider::{InferenceProvider, ModelInfo, ProviderStatus};
use neunode_inference::router::{Router, RoutingStrategy};
use neunode_storage::db::NeunodeDb;

use crate::cli::{Cli, InferenceCommands};
use crate::output::OutputWriter;
use crate::state::AppState;

pub fn execute(cmd: &InferenceCommands, cli: &Cli, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        InferenceCommands::Request { model, prompt, max_tokens, temperature } => {
            request_inference(model, prompt, *max_tokens, Some(*temperature), &writer, state)
        }
        InferenceCommands::ListModels { provider } => {
            list_models(provider.as_deref(), &writer, state)
        }
        InferenceCommands::Providers { model } => list_providers(model.as_deref(), &writer, state),
        InferenceCommands::Route { model, strategy } => {
            route_request(model, strategy, &writer, state)
        }
        InferenceCommands::Pricing { model, input_tokens, output_tokens } => {
            show_pricing(model, *input_tokens, *output_tokens, &writer, state)
        }
    }
}

#[cfg(test)]
fn store_provider(db: &NeunodeDb, provider: &InferenceProvider) -> Result<()> {
    let key = format!("prov:{}", provider.did);
    let key_bytes = bincode::serialize(&key).map_err(|e| anyhow::anyhow!("key serialize: {e}"))?;
    let value =
        bincode::serialize(provider).map_err(|e| anyhow::anyhow!("serialize provider: {e}"))?;
    db.put_raw(neunode_storage::cf::CF_MODELS, &key_bytes, &value)?;
    Ok(())
}

fn load_all_providers(db: &NeunodeDb) -> Vec<InferenceProvider> {
    let entries = match db.prefix_scan(neunode_storage::cf::CF_MODELS, &[]) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .filter(|(k, _)| {
            let key_str = bincode::deserialize::<String>(k).unwrap_or_default();
            key_str.starts_with("prov:")
        })
        .filter_map(|(_, v)| bincode::deserialize::<InferenceProvider>(v).ok())
        .collect()
}

fn request_inference(
    model: &str,
    prompt: &str,
    max_tokens: u32,
    temperature: Option<f64>,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if model.is_empty() {
        anyhow::bail!("model cannot be empty");
    }
    if prompt.is_empty() {
        anyhow::bail!("prompt cannot be empty");
    }
    if max_tokens == 0 {
        anyhow::bail!("max_tokens must be greater than 0");
    }

    let temp = temperature.unwrap_or(0.7);
    if !(0.0..=2.0).contains(&temp) {
        anyhow::bail!("temperature {} out of range (0.0-2.0)", temp);
    }

    let request = ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: prompt.to_string(),
            name: None,
        }],
        temperature: Some(temp),
        max_tokens: Some(max_tokens),
        top_p: None,
        stream: None,
        stop: None,
        frequency_penalty: None,
        presence_penalty: None,
    };

    request.validate()?;

    let estimated_tokens = request.estimate_tokens();

    let providers = load_all_providers(state.db());
    let pricing_info = providers.iter().find_map(|p| p.find_model(model)).map(|m| {
        let cost = (estimated_tokens as u64 * m.input_price_per_million.0 / 1_000_000).max(1);
        serde_json::json!({
            "input_price_per_mtok": m.input_price_per_million.0,
            "output_price_per_mtok": m.output_price_per_million.0,
            "estimated_cost": cost,
        })
    });

    let mut out = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "max_tokens": max_tokens,
        "temperature": temp,
        "estimated_input_tokens": estimated_tokens,
        "status": "submitted",
    });
    if let Some(pricing) = pricing_info {
        out["pricing"] = pricing;
    }

    writer.write_json(&out);
    writer.write_status(&format!("Inference request submitted for model: {model}"));
    Ok(())
}

fn list_models(provider: Option<&str>, writer: &OutputWriter, state: &AppState) -> Result<()> {
    let providers = load_all_providers(state.db());

    let mut models: Vec<&ModelInfo> = Vec::new();
    for p in &providers {
        for m in &p.models {
            if !models.iter().any(|existing| existing.id == m.id) {
                models.push(m);
            }
        }
    }

    if models.is_empty() {
        writer
            .write_status("No models found — register providers with model push or P2P discovery");
        return Ok(());
    }

    let filtered: Vec<&&ModelInfo> =
        models.iter().filter(|m| provider.is_none_or(|p| m.id.contains(p))).collect();

    let headers = ["Model", "Input Price", "Output Price", "Context"];
    let rows: Vec<Vec<String>> = filtered
        .iter()
        .map(|m| {
            vec![
                m.id.clone(),
                format!("{} per 1M", m.input_price_per_million),
                format!("{} per 1M", m.output_price_per_million),
                m.context_length.to_string(),
            ]
        })
        .collect();

    if rows.is_empty() {
        writer.write_status("No models matching filter");
    } else {
        writer.write_table(&headers, &rows);
    }
    Ok(())
}

fn list_providers(model: Option<&str>, writer: &OutputWriter, state: &AppState) -> Result<()> {
    let providers = load_all_providers(state.db());

    if providers.is_empty() {
        writer.write_status("No inference providers registered");
        return Ok(());
    }

    let filtered: Vec<&InferenceProvider> =
        providers.iter().filter(|p| model.is_none_or(|m| p.has_model(m))).collect();

    let headers = ["Provider", "Status", "Reputation", "Latency", "Models"];
    let rows: Vec<Vec<String>> = filtered
        .iter()
        .map(|p| {
            let status = match p.status {
                ProviderStatus::Online => "online",
                ProviderStatus::Degraded => "degraded",
                ProviderStatus::Offline => "offline",
            };
            vec![
                p.name.clone(),
                status.to_string(),
                format!("{:.1}", p.reputation_score),
                format!("{}ms", p.avg_latency_ms),
                p.models.len().to_string(),
            ]
        })
        .collect();
    writer.write_table(&headers, &rows);
    Ok(())
}

fn route_request(
    model: &str,
    strategy: &str,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if model.is_empty() {
        anyhow::bail!("model cannot be empty");
    }

    let strat = match strategy.to_lowercase().as_str() {
        "cheapest" => RoutingStrategy::Cheapest,
        "fastest" => RoutingStrategy::Fastest,
        "reputation" | "highest_reputation" => RoutingStrategy::HighestReputation,
        "random" => RoutingStrategy::Random,
        "round_robin" => RoutingStrategy::RoundRobin,
        _ => {
            anyhow::bail!(
                "invalid strategy '{}'. Must be: cheapest, fastest, reputation, random, round_robin",
                strategy
            );
        }
    };

    let providers = load_all_providers(state.db());

    if providers.is_empty() {
        writer.write_status("No providers registered — routing unavailable");
        let info = serde_json::json!({
            "model": model,
            "strategy": strategy,
            "status": "no_providers",
        });
        writer.write_json(&info);
        return Ok(());
    }

    let router = Router::new(strat);
    let chosen = router.route(&providers, model, Some(0))?;

    let out = serde_json::json!({
        "model": model,
        "strategy": strategy,
        "selected_provider": chosen.did.to_string(),
        "provider_name": chosen.name.clone(),
    });

    writer.write_json(&out);
    writer.write_status(&format!("Routed to: {} ({})", chosen.name, chosen.did));
    Ok(())
}

fn show_pricing(
    model: &str,
    input_tokens: u32,
    output_tokens: u32,
    writer: &OutputWriter,
    state: &AppState,
) -> Result<()> {
    if model.is_empty() {
        anyhow::bail!("model cannot be empty");
    }
    if input_tokens == 0 && output_tokens == 0 {
        anyhow::bail!("at least one of input_tokens or output_tokens must be > 0");
    }

    let providers = load_all_providers(state.db());
    let model_info =
        providers.iter().find_map(|p| p.find_model(model)).cloned().unwrap_or_else(|| ModelInfo {
            id: model.to_string(),
            base_model: None,
            context_length: 4096,
            input_price_per_million: TokenAmount(100),
            output_price_per_million: TokenAmount(200),
            capabilities: vec!["chat".to_string()],
        });

    let input_cost = (input_tokens as u64) * model_info.input_price_per_million.0 / 1_000_000;
    let output_cost = (output_tokens as u64) * model_info.output_price_per_million.0 / 1_000_000;
    let total = input_cost.saturating_add(output_cost);
    let total_cost =
        if total == 0 && (input_tokens > 0 || output_tokens > 0) { 1u64 } else { total };

    let protocol_fee = ((total_cost as f64) * 2.0 / 100.0).ceil() as u64;
    let net_payout = total_cost.saturating_sub(protocol_fee);

    let out = serde_json::json!({
        "model": model,
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "input_cost": input_cost,
        "output_cost": output_cost,
        "total_cost": total_cost,
        "protocol_fee": protocol_fee,
        "net_payout": net_payout,
    });

    writer.write_json(&out);
    writer.write_status(&format!("Pricing for: {model}"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OutputFormat;
    use crate::config::CliConfig;
    use crate::state::AppState;
    use neunode_core::types::Did;
    use neunode_identity::keyring::Keyring;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };

    fn test_writer() -> OutputWriter {
        OutputWriter::new(OutputFormat::Json)
    }

    fn human_writer() -> OutputWriter {
        OutputWriter::new(OutputFormat::Human)
    }

    fn test_state() -> AppState {
        static TEST_ID: AtomicU64 = AtomicU64::new(0);
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("agnetd_test_inf_{:?}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = neunode_storage::db::NeunodeDb::open(&dir).unwrap();
        let kr = Keyring::generate();
        let did = kr.to_did();
        AppState {
            db: Arc::new(db),
            config: CliConfig::load(None).unwrap(),
            active_keyring: Some(kr),
            active_did: Some(did),
            mesh_handle: None,
        }
    }

    #[test]
    fn request_valid() {
        let state = test_state();
        let writer = test_writer();
        request_inference("neunode/llama-3b", "Hello, world!", 256, None, &writer, &state).unwrap();
    }

    #[test]
    fn request_with_temperature() {
        let state = test_state();
        let writer = test_writer();
        request_inference("neunode/llama-3b", "Hello", 100, Some(1.5), &writer, &state).unwrap();
    }

    #[test]
    fn request_empty_model_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(request_inference("", "Hello", 100, None, &writer, &state).is_err());
    }

    #[test]
    fn request_empty_prompt_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(request_inference("neunode/llama-3b", "", 100, None, &writer, &state).is_err());
    }

    #[test]
    fn request_zero_max_tokens_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(request_inference("neunode/llama-3b", "Hello", 0, None, &writer, &state).is_err());
    }

    #[test]
    fn request_temperature_out_of_range_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(request_inference("neunode/llama-3b", "Hello", 100, Some(3.0), &writer, &state)
            .is_err());
    }

    #[test]
    fn request_temperature_boundary_ok() {
        let state = test_state();
        let writer = test_writer();
        request_inference("neunode/llama-3b", "Hello", 100, Some(0.0), &writer, &state).unwrap();
        request_inference("neunode/llama-3b", "Hello", 100, Some(2.0), &writer, &state).unwrap();
    }

    #[test]
    fn list_models_empty_db() {
        let state = test_state();
        let writer = human_writer();
        list_models(None, &writer, &state).unwrap();
    }

    #[test]
    fn list_models_with_filter() {
        let state = test_state();
        let writer = test_writer();
        list_models(Some("medical"), &writer, &state).unwrap();
    }

    #[test]
    fn providers_empty_db() {
        let state = test_state();
        let writer = test_writer();
        list_providers(None, &writer, &state).unwrap();
    }

    #[test]
    fn providers_with_model_filter() {
        let state = test_state();
        let writer = human_writer();
        list_providers(Some("neunode/llama-3b"), &writer, &state).unwrap();
    }

    #[test]
    fn route_no_providers() {
        let state = test_state();
        let writer = test_writer();
        route_request("neunode/llama-3b", "cheapest", &writer, &state).unwrap();
    }

    #[test]
    fn route_invalid_strategy_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(route_request("neunode/llama-3b", "invalid", &writer, &state).is_err());
    }

    #[test]
    fn route_empty_model_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(route_request("", "cheapest", &writer, &state).is_err());
    }

    #[test]
    fn pricing_default_rates() {
        let state = test_state();
        let writer = test_writer();
        show_pricing("neunode/llama-3b", 1000, 500, &writer, &state).unwrap();
    }

    #[test]
    fn pricing_empty_model_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(show_pricing("", 100, 100, &writer, &state).is_err());
    }

    #[test]
    fn pricing_zero_tokens_fails() {
        let state = test_state();
        let writer = test_writer();
        assert!(show_pricing("neunode/llama-3b", 0, 0, &writer, &state).is_err());
    }

    #[test]
    fn store_and_load_provider() {
        let state = test_state();
        let provider = InferenceProvider {
            did: Did("did:neunode:0xtestprovider1".to_string()),
            name: "test-provider".to_string(),
            endpoint: "https://test.neunode.io/v1".to_string(),
            models: vec![ModelInfo {
                id: "neunode/test-model".to_string(),
                base_model: None,
                context_length: 4096,
                input_price_per_million: TokenAmount(100),
                output_price_per_million: TokenAmount(200),
                capabilities: vec!["chat".to_string()],
            }],
            reputation_score: 80.0,
            stake_amount: TokenAmount(500),
            status: ProviderStatus::Online,
            last_heartbeat: 1000,
            total_requests_served: 0,
            avg_latency_ms: 50,
        };

        store_provider(state.db(), &provider).unwrap();
        let loaded = load_all_providers(state.db());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "test-provider");
        assert_eq!(loaded[0].did, provider.did);
    }
}
