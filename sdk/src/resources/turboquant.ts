import type { NeunodeClient } from "../client/client.js";

export interface TurboquantCompressParams {
	profile: "gradient" | "kv_cache" | "custom";
	workers?: number;
	bandwidthMbps?: number;
	targetBits?: number;
	bits?: number;
	dimension: number;
}

export interface TurboquantCompressResult {
	strategy: "int8" | "mse";
	bits?: number;
}

export interface TurboquantDecompressParams {
	data: readonly number[];
	scale: number;
	minValue: number;
	strategy: "int8";
}

export interface TurboquantDecompressResult {
	data: number[];
}

export interface TurboquantCodebookParams {
	bits: number;
	dimension: number;
	maxIterations?: number;
	convergenceThreshold?: number;
	numSamples?: number;
}

export interface TurboquantCodebookResult {
	bits: number;
	levels: number[];
	dimension: number;
	iterations: number;
	mse: number;
}

/** TurboQuant compression: adaptive strategy selection, codebook generation, quantization. */
export interface TurboquantResource {
	/** Select the optimal quantization strategy for a compression profile. */
	compress(params: TurboquantCompressParams): Promise<TurboquantCompressResult>;
	/** Generate a scalar quantization codebook for the given configuration. */
	generateCodebook(params: TurboquantCodebookParams): Promise<TurboquantCodebookResult>;
}

export function createTurboquantResource(client: NeunodeClient): TurboquantResource {
	const cli = client.cli;
	if (!cli) throw new Error("CLI transport required for turboquant operations");

	return {
		async compress(params: TurboquantCompressParams): Promise<TurboquantCompressResult> {
			const args = [
				"turboquant",
				"compress",
				"--profile",
				params.profile,
				"--dimension",
				String(params.dimension),
			];
			if (params.profile === "gradient") {
				if (params.workers !== undefined) {
					args.push("--workers", String(params.workers));
				}
				if (params.bandwidthMbps !== undefined) {
					args.push("--bandwidth-mbps", String(params.bandwidthMbps));
				}
			}
			if (params.profile === "kv_cache" && params.targetBits !== undefined) {
				args.push("--target-bits", String(params.targetBits));
			}
			if (params.profile === "custom" && params.bits !== undefined) {
				args.push("--bits", String(params.bits));
			}
			return cli.execute<TurboquantCompressResult>(args);
		},

		async generateCodebook(
			params: TurboquantCodebookParams,
		): Promise<TurboquantCodebookResult> {
			const args = [
				"turboquant",
				"generate-codebook",
				"--bits",
				String(params.bits),
				"--dimension",
				String(params.dimension),
			];
			if (params.maxIterations !== undefined) {
				args.push("--max-iterations", String(params.maxIterations));
			}
			if (params.convergenceThreshold !== undefined) {
				args.push("--convergence-threshold", String(params.convergenceThreshold));
			}
			if (params.numSamples !== undefined) {
				args.push("--num-samples", String(params.numSamples));
			}
			return cli.execute<TurboquantCodebookResult>(args);
		},
	};
}
