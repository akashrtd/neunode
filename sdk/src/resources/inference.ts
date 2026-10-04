import type { NeunodeClient } from "../client/client.js";
import type { ChatCompletionResponse } from "../types/inference.js";

export interface InferenceRequestParams {
	model: string;
	prompt: string;
	maxTokens: number;
	temperature?: number;
	idempotencyKey?: string;
}

export interface InferenceRequestResult {
	model: string;
	prompt: string;
	max_tokens: number;
	temperature: number;
	estimated_input_tokens: number;
	status: string;
	request_id: string;
	completion: ChatCompletionResponse | null;
	settlement: {
		requester: string;
		provider: string;
		gross_cost: string;
		protocol_fee: string;
		net_payout: string;
		response_hash: string;
		ledger: string;
	} | null;
	pricing?: {
		input_price_per_mtok: string;
		output_price_per_mtok: string;
		estimated_cost: string;
	};
}

export interface InferenceListModelsResult {
	models: Array<{
		id: string;
		input_price_per_million: string;
		output_price_per_million: string;
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
	input_cost: string;
	output_cost: string;
	total_cost: string;
	protocol_fee: string;
	net_payout: string;
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
		onError?: (error: Error) => void,
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
					idempotency_key: params.idempotencyKey,
				},
			);
		},

		stream(params, callback, onError): () => void {
			const url = `${client.http.getBaseUrl().replace(/^http/, "ws")}/ws/inference`;
			const token = client.http.getApiKey?.();
			const protocol = token
				? `neunode-auth.${Array.from(new TextEncoder().encode(token), (byte) => byte.toString(16).padStart(2, "0")).join("")}`
				: undefined;
			const socket = protocol
				? new WebSocket(url, protocol)
				: new WebSocket(url);
			socket.onerror = () =>
				onError?.(
					new Error("Inference WebSocket failed to connect or authenticate"),
				);
			socket.onopen = () =>
				socket.send(
					JSON.stringify({
						model: params.model,
						prompt: params.prompt,
						max_tokens: params.maxTokens,
						temperature: params.temperature,
						idempotency_key: params.idempotencyKey,
					}),
				);
			socket.onmessage = (event: MessageEvent) => {
				try {
					const result = JSON.parse(
						event.data as string,
					) as InferenceRequestResult;
					if ("error" in result) onError?.(new Error(String(result.error)));
					else callback(result);
				} catch (error) {
					onError?.(error instanceof Error ? error : new Error(String(error)));
				}
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
