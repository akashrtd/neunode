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
	generateCodebook(
		params: TurboquantCodebookParams,
	): Promise<TurboquantCodebookResult>;
}

export function createTurboquantResource(
	client: NeunodeClient,
): TurboquantResource {
	return {
		async compress(
			params: TurboquantCompressParams,
		): Promise<TurboquantCompressResult> {
			return client.http.post<TurboquantCompressResult>(
				"/api/v1/turboquant/compress",
				params,
			);
		},

		async generateCodebook(
			params: TurboquantCodebookParams,
		): Promise<TurboquantCodebookResult> {
			return client.http.post<TurboquantCodebookResult>(
				"/api/v1/turboquant/codebook",
				params,
			);
		},
	};
}
