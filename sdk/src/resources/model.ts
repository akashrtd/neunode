import type { NeunodeClient } from "../client/client.js";

export interface ModelListResult {
	data: Array<{
		"Model ID": string;
		"Base Model": string;
		Context: string;
		"Input/MTok": string;
		"Output/MTok": string;
		Capabilities: string;
	}>;
}

export interface ModelShowResult {
	"Model ID": string;
	"Base Model": string;
	"Context Length": string;
	"Input Price/MTok": string;
	"Output Price/MTok": string;
	"Total Price/MTok": string;
	Capabilities: string;
}

export interface ModelPushParams {
	path: string;
	name: string;
}

export interface ModelPushResult {
	Status: string;
	"Model ID": string;
	Source: string;
	"Context Length": string;
	"Input Price/MTok": string;
	"Output Price/MTok": string;
}

export interface ModelRmResult {
	action: string;
	model_id: string;
	status: string;
}

/** Model registry operations. */
export interface ModelResource {
	/** List registered models, optionally filtered by provider. */
	list(provider?: string): Promise<ModelListResult>;
	/** Show details for a specific model. */
	show(modelId: string): Promise<ModelShowResult>;
	/** Push a local model to the network. */
	push(params: ModelPushParams): Promise<ModelPushResult>;
	/** Remove a model from the local registry. */
	rm(modelId: string): Promise<ModelRmResult>;
}

export function createModelResource(client: NeunodeClient): ModelResource {
	return {
		async list(provider?: string): Promise<ModelListResult> {
			if (client.http) {
				const qs = new URLSearchParams();
				if (provider) qs.set("provider", provider);
				const query = qs.toString();
				return client.http.get<ModelListResult>(
					query ? `/api/v1/models?${query}` : "/api/v1/models",
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for model operations");
			const args = ["model", "list"];
			if (provider) args.push("--provider", provider);
			return cli.execute<ModelListResult>(args);
		},

		async show(modelId: string): Promise<ModelShowResult> {
			if (client.http) {
				return client.http.get<ModelShowResult>(
					`/api/v1/models/${encodeURIComponent(modelId)}`,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for model operations");
			return cli.execute<ModelShowResult>([
				"model",
				"show",
				"--model-id",
				modelId,
			]);
		},

		async push(params: ModelPushParams): Promise<ModelPushResult> {
			if (client.http) {
				return client.http.post<ModelPushResult>("/api/v1/models", params);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for model operations");
			return cli.execute<ModelPushResult>([
				"model",
				"push",
				"--path",
				params.path,
				"--name",
				params.name,
			]);
		},

		async rm(modelId: string): Promise<ModelRmResult> {
			if (client.http) {
				return client.http.delete<ModelRmResult>(
					`/api/v1/models/${encodeURIComponent(modelId)}`,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for model operations");
			return cli.execute<ModelRmResult>(["model", "rm", "--model-id", modelId]);
		},
	};
}
