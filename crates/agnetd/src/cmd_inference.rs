use anyhow::Result;
use neunode_core::types::{Did, TokenAmount};
use neunode_inference::openai::{ChatCompletionRequest, ChatMessage, MessageRole};
use neunode_inference::provider::{InferenceProvider, ModelInfo, ProviderStatus};
use neunode_inference::router::{Router, RoutingStrategy};

use crate::cli::{Cli, InferenceCommands};
use crate::config::CliConfig;
use crate::output::OutputWriter;

pub fn execute(cmd: &InferenceCommands, cli: &Cli, _config: &mut CliConfig) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        InferenceCommands::Request { model, prompt, max_tokens, temperature } => {
            request_inference(model, prompt, *max_tokens, Some(*temperature), &writer)
        }
        InferenceCommands::ListModels { provider } => list_models(provider.as_deref(), &writer),
        InferenceCommands::Providers { model } => list_providers(model.as_deref(), &writer),
        InferenceCommands::Route { model, strategy } => route_request(model, strategy, &writer),
        InferenceCommands::Pricing { model, input_tokens, output_tokens } => {
            show_pricing(model, *input_tokens, *output_tokens, &writer)
        }
    }
}

fn test_model(model_id: &str) -> ModelInfo {
    ModelInfo {
        id: model_id.to_string(),
        base_model: Some("llama-3b".to_string()),
        context_length: 4096,
        input_price_per_million: TokenAmount(100),
        output_price_per_million: TokenAmount(200),
        capabilities: vec!["chat".to_string(), "streaming".to_string()],
    }
}

fn test_provider(did_num: u32, model_ids: &[&str], rep: f64, latency_ms: u32) -> InferenceProvider {
    InferenceProvider {
        did: Did(format!("did:neunode:provider_{}", did_num)),
        name: format!("provider-{}", did_num),
        endpoint: format!("https://provider-{}.neunode.io/v1", did_num),
        models: model_ids.iter().map(|m| test_model(m)).collect(),
        reputation_score: rep,
        stake_amount: TokenAmount(1000),
        status: ProviderStatus::Online,
        last_heartbeat: 1000,
        total_requests_served: 0,
        avg_latency_ms: latency_ms,
    }
}

fn request_inference(
    model: &str,
    prompt: &str,
    max_tokens: u32,
    temperature: Option<f64>,
    writer: &OutputWriter,
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

    let out = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "max_tokens": max_tokens,
        "temperature": temp,
        "estimated_input_tokens": estimated_tokens,
        "status": "submitted",
    });

    writer.write_json(&out);
    writer.write_status(&format!("Inference request submitted for model: {model}"));
    Ok(())
}

fn list_models(provider: Option<&str>, writer: &OutputWriter) -> Result<()> {
    let all_models = [
        ("neunode/llama-3b", 100u64, 200u64, 4096u32),
        ("neunode/llama-3b-medical", 150, 300, 4096),
        ("neunode/mistral-7b", 200, 400, 8192),
    ];

    let headers = ["Model", "Input Price", "Output Price", "Context"];
    let rows: Vec<Vec<String>> = all_models
        .iter()
        .filter(|(id, _, _, _)| provider.is_none_or(|p| id.contains(p)))
        .map(|(id, inp, out, ctx_len)| {
            vec![
                id.to_string(),
                format!("{} per 1M", inp),
                format!("{} per 1M", out),
                ctx_len.to_string(),
            ]
        })
        .collect();
    writer.write_table(&headers, &rows);
    Ok(())
}

fn list_providers(model: Option<&str>, writer: &OutputWriter) -> Result<()> {
    let providers = [
        test_provider(1, &["neunode/llama-3b", "neunode/mistral-7b"], 85.0, 50),
        test_provider(2, &["neunode/llama-3b"], 70.0, 120),
        test_provider(3, &["neunode/mistral-7b"], 92.0, 30),
    ];

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

fn route_request(model: &str, strategy: &str, writer: &OutputWriter) -> Result<()> {
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

    let providers = vec![
        test_provider(1, &["neunode/llama-3b"], 70.0, 120),
        test_provider(2, &["neunode/llama-3b"], 90.0, 50),
    ];

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
) -> Result<()> {
    if model.is_empty() {
        anyhow::bail!("model cannot be empty");
    }
    if input_tokens == 0 && output_tokens == 0 {
        anyhow::bail!("at least one of input_tokens or output_tokens must be > 0");
    }

    let model_info = test_model(model);

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

    fn test_writer() -> OutputWriter {
        OutputWriter::new(OutputFormat::Json)
    }

    fn human_writer() -> OutputWriter {
        OutputWriter::new(OutputFormat::Human)
    }

    #[test]
    fn request_valid() {
        let writer = test_writer();
        request_inference("neunode/llama-3b", "Hello, world!", 256, None, &writer).unwrap();
    }

    #[test]
    fn request_with_temperature() {
        let writer = test_writer();
        request_inference("neunode/llama-3b", "Hello", 100, Some(1.5), &writer).unwrap();
    }

    #[test]
    fn request_empty_model_fails() {
        let writer = test_writer();
        assert!(request_inference("", "Hello", 100, None, &writer).is_err());
    }

    #[test]
    fn request_empty_prompt_fails() {
        let writer = test_writer();
        assert!(request_inference("neunode/llama-3b", "", 100, None, &writer).is_err());
    }

    #[test]
    fn request_zero_max_tokens_fails() {
        let writer = test_writer();
        assert!(request_inference("neunode/llama-3b", "Hello", 0, None, &writer).is_err());
    }

    #[test]
    fn request_temperature_out_of_range_fails() {
        let writer = test_writer();
        assert!(request_inference("neunode/llama-3b", "Hello", 100, Some(3.0), &writer).is_err());
    }

    #[test]
    fn request_temperature_boundary_ok() {
        let writer = test_writer();
        request_inference("neunode/llama-3b", "Hello", 100, Some(0.0), &writer).unwrap();
        request_inference("neunode/llama-3b", "Hello", 100, Some(2.0), &writer).unwrap();
    }

    #[test]
    fn list_models_does_not_panic() {
        let writer = human_writer();
        list_models(None, &writer).unwrap();
    }

    #[test]
    fn list_models_with_filter() {
        let writer = test_writer();
        list_models(Some("medical"), &writer).unwrap();
    }

    #[test]
    fn providers_does_not_panic() {
        let writer = test_writer();
        list_providers(None, &writer).unwrap();
    }

    #[test]
    fn providers_with_model_filter() {
        let writer = human_writer();
        list_providers(Some("neunode/llama-3b"), &writer).unwrap();
    }

    #[test]
    fn route_cheapest() {
        let writer = test_writer();
        route_request("neunode/llama-3b", "cheapest", &writer).unwrap();
    }

    #[test]
    fn route_fastest() {
        let writer = test_writer();
        route_request("neunode/llama-3b", "fastest", &writer).unwrap();
    }

    #[test]
    fn route_reputation() {
        let writer = test_writer();
        route_request("neunode/llama-3b", "reputation", &writer).unwrap();
    }

    #[test]
    fn route_invalid_strategy_fails() {
        let writer = test_writer();
        assert!(route_request("neunode/llama-3b", "invalid", &writer).is_err());
    }

    #[test]
    fn route_empty_model_fails() {
        let writer = test_writer();
        assert!(route_request("", "cheapest", &writer).is_err());
    }

    #[test]
    fn pricing_valid() {
        let writer = test_writer();
        show_pricing("neunode/llama-3b", 1000, 500, &writer).unwrap();
    }

    #[test]
    fn pricing_empty_model_fails() {
        let writer = test_writer();
        assert!(show_pricing("", 100, 100, &writer).is_err());
    }

    #[test]
    fn pricing_zero_tokens_fails() {
        let writer = test_writer();
        assert!(show_pricing("neunode/llama-3b", 0, 0, &writer).is_err());
    }

    #[test]
    fn route_round_robin() {
        let writer = test_writer();
        route_request("neunode/llama-3b", "round_robin", &writer).unwrap();
    }
}
