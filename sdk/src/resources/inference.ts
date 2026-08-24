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
	models: Array<{
		id: string;
		input_price_per_million: number;
		output_price_per_million: number;
		context_length: number;
	}>;
}

export interface InferenceProvidersResult {
	providers: Array<{
		name: string;
		did: string;
		status: string;
		reputation_score: number;
		avg_latency_ms: number;
		model_count: number;
	}>;
}

export interface InferenceRouteResult {
	model: string;
	strategy: string;
	selected_provider: string | null;
	provider_name: string | null;
	status: string;
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

export interface InferenceRegisterProviderParams {
	name: string;
	endpoint: string;
	models: string[];
}

export interface InferenceRegisterProviderResult {
	did: string;
	name: string;
	endpoint: string;
	models: string[];
	status: string;
}

/** Inference marketplace for model queries, discovery, and pricing. */
export interface InferenceResource {
	/** Advertise locally registered models as an inference provider. */
	registerProvider(
		params: InferenceRegisterProviderParams,
	): Promise<InferenceRegisterProviderResult>;
	/** Send an inference request to a model. */
	request(params: InferenceRequestParams): Promise<InferenceRequestResult>;
	/** Submit an inference request and receive its result over WebSocket. */
	stream(
		params: InferenceRequestParams,
		callback: (result: InferenceRequestResult) => void,
	): () => void;
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
		async registerProvider(params) {
			return client.http.post<InferenceRegisterProviderResult>(
				"/api/v1/inference/providers",
				params,
			);
		},

		async request(
			params: InferenceRequestParams,
		): Promise<InferenceRequestResult> {
			return client.http.post<InferenceRequestResult>(
				"/api/v1/inference/request",
				{
					model: params.model,
					prompt: params.prompt,
					max_tokens: params.maxTokens,
					temperature: params.temperature,
				},
			);
		},

		stream(params, callback): () => void {
			const url = `${client.http.getBaseUrl().replace(/^http/, "ws")}/ws/inference`;
			const socket = new WebSocket(url);
			socket.onopen = () =>
				socket.send(
					JSON.stringify({
						model: params.model,
						prompt: params.prompt,
						max_tokens: params.maxTokens,
						temperature: params.temperature,
					}),
				);
			socket.onmessage = (event: MessageEvent) => {
				const result = JSON.parse(
					event.data as string,
				) as InferenceRequestResult;
				callback(result);
			};
			return () => socket.close();
		},

		async listModels(provider?: string): Promise<InferenceListModelsResult> {
			const qs = new URLSearchParams();
			if (provider) qs.set("provider", provider);
			const query = qs.toString();
			return client.http.get<InferenceListModelsResult>(
				query
					? `/api/v1/inference/models?${query}`
					: "/api/v1/inference/models",
			);
		},

		async providers(model?: string): Promise<InferenceProvidersResult> {
			const qs = new URLSearchParams();
			if (model) qs.set("model", model);
			const query = qs.toString();
			return client.http.get<InferenceProvidersResult>(
				query
					? `/api/v1/inference/providers?${query}`
					: "/api/v1/inference/providers",
			);
		},

		async route(
			model: string,
			strategy?: string,
		): Promise<InferenceRouteResult> {
			const qs = new URLSearchParams();
			qs.set("model", model);
			qs.set("strategy", strategy ?? "cheapest");
			return client.http.get<InferenceRouteResult>(
				`/api/v1/inference/route?${qs.toString()}`,
			);
		},

		async pricing(
			model: string,
			inputTokens: number,
			outputTokens: number,
		): Promise<InferencePricingResult> {
			const qs = new URLSearchParams();
			qs.set("model", model);
			qs.set("input_tokens", String(inputTokens));
			qs.set("output_tokens", String(outputTokens));
			return client.http.get<InferencePricingResult>(
				`/api/v1/inference/pricing?${qs.toString()}`,
			);
		},
	};
}
