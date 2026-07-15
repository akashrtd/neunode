import type { NeunodeClient } from "../client/client.js";

export interface InferenceRequestParams {
	model: string;
	prompt: string;
	maxTokens: number;
	temperature?: number;
}

export interface InferenceRequestResult {
	model: string;
	prompt: string;
	max_tokens: number;
	temperature: number;
	estimated_input_tokens: number;
	status: string;
	pricing?: {
		input_price_per_mtok: number;
		output_price_per_mtok: number;
		estimated_cost: number;
	};
}

export interface InferenceListModelsResult {
	data: Array<{
		Model: string;
		"Input Price": string;
		"Output Price": string;
		Context: string;
	}>;
}

export interface InferenceProvidersResult {
	data: Array<{
		Provider: string;
		Status: string;
		Reputation: string;
		Latency: string;
		Models: string;
	}>;
}

export interface InferenceRouteResult {
	model: string;
	strategy: string;
	selected_provider: string;
	provider_name: string;
}

export interface InferencePricingResult {
	model: string;
	input_tokens: number;
	output_tokens: number;
	input_cost: number;
	output_cost: number;
	total_cost: number;
	protocol_fee: number;
	net_payout: number;
}

/** Inference marketplace for model queries, discovery, and pricing. */
export interface InferenceResource {
	/** Send an inference request to a model. */
	request(params: InferenceRequestParams): Promise<InferenceRequestResult>;
	/** List available models, optionally filtered by provider. */
	listModels(provider?: string): Promise<InferenceListModelsResult>;
	/** List inference providers, optionally filtered by model. */
	providers(model?: string): Promise<InferenceProvidersResult>;
	/** Route an inference request to the best provider for a given strategy. */
	route(model: string, strategy?: string): Promise<InferenceRouteResult>;
	/** Estimate pricing for a given token usage. */
	pricing(
		model: string,
		inputTokens: number,
		outputTokens: number,
	): Promise<InferencePricingResult>;
}

export function createInferenceResource(
	client: NeunodeClient,
): InferenceResource {
	return {
		async request(
			params: InferenceRequestParams,
		): Promise<InferenceRequestResult> {
			if (client.http) {
				return client.http.post<InferenceRequestResult>(
					"/api/v1/inference/request",
					params,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for inference operations",
				);
			const args = [
				"inference",
				"request",
				"--model",
				params.model,
				"--prompt",
				params.prompt,
				"--max-tokens",
				String(params.maxTokens),
			];
			if (params.temperature !== undefined)
				args.push("--temperature", String(params.temperature));
			return cli.execute<InferenceRequestResult>(args);
		},

		async listModels(provider?: string): Promise<InferenceListModelsResult> {
			if (client.http) {
				const qs = new URLSearchParams();
				if (provider) qs.set("provider", provider);
				const query = qs.toString();
				return client.http.get<InferenceListModelsResult>(
					query
						? `/api/v1/inference/models?${query}`
						: "/api/v1/inference/models",
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for inference operations",
				);
			const args = ["inference", "list-models"];
			if (provider) args.push("--provider", provider);
			return cli.execute<InferenceListModelsResult>(args);
		},

		async providers(model?: string): Promise<InferenceProvidersResult> {
			if (client.http) {
				const qs = new URLSearchParams();
				if (model) qs.set("model", model);
				const query = qs.toString();
				return client.http.get<InferenceProvidersResult>(
					query
						? `/api/v1/inference/providers?${query}`
						: "/api/v1/inference/providers",
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for inference operations",
				);
			const args = ["inference", "providers"];
			if (model) args.push("--model", model);
			return cli.execute<InferenceProvidersResult>(args);
		},

		async route(
			model: string,
			strategy?: string,
		): Promise<InferenceRouteResult> {
			if (client.http) {
				const qs = new URLSearchParams();
				qs.set("model", model);
				qs.set("strategy", strategy ?? "cheapest");
				return client.http.get<InferenceRouteResult>(
					`/api/v1/inference/route?${qs.toString()}`,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for inference operations",
				);
			const args = [
				"inference",
				"route",
				"--model",
				model,
				"--strategy",
				strategy ?? "cheapest",
			];
			return cli.execute<InferenceRouteResult>(args);
		},

		async pricing(
			model: string,
			inputTokens: number,
			outputTokens: number,
		): Promise<InferencePricingResult> {
			if (client.http) {
				const qs = new URLSearchParams();
				qs.set("model", model);
				qs.set("input_tokens", String(inputTokens));
				qs.set("output_tokens", String(outputTokens));
				return client.http.get<InferencePricingResult>(
					`/api/v1/inference/pricing?${qs.toString()}`,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for inference operations",
				);
			return cli.execute<InferencePricingResult>([
				"inference",
				"pricing",
				"--model",
				model,
				"--input-tokens",
				String(inputTokens),
				"--output-tokens",
				String(outputTokens),
			]);
		},
	};
}
