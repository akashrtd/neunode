// @neunode/sdk — TurboQuant compression types mirroring neunode-turboquant crate
// (codebook, adaptive, int8, mse, rotation)

// ─── Compression Profile ──────────────────────────────────────────────────────────

/** Describes the compression use case and its constraints. */
export type CompressionProfile =
	| {
			readonly type: "gradient";
			readonly workers: number;
			readonly bandwidth_mbps: number;
	  }
	| {
			readonly type: "kv_cache";
			readonly target_bits: number;
			readonly dimension: number;
	  }
	| {
			readonly type: "custom";
			readonly bits: number;
			readonly dimension: number;
	  };

// ─── Quantization Strategy ────────────────────────────────────────────────────────

/** The quantization strategy selected by the adaptive selector. */
export type QuantizationStrategy =
	| { readonly type: "int8" }
	| { readonly type: "mse"; readonly bits: number };

// ─── Codebook Config ──────────────────────────────────────────────────────────────

/** Configuration for codebook generation. */
export interface CodebookConfig {
	readonly bits: number;
	readonly dimension: number;
	readonly max_iterations: number;
	readonly convergence_threshold: number;
	readonly num_samples: number;
}

// ─── Codebook ──────────────────────────────────────────────────────────────────────

/** A scalar quantization codebook with optimized levels. */
export interface Codebook {
	readonly bits: number;
	readonly levels: readonly number[];
	readonly dimension: number;
	readonly iterations: number;
	readonly mse: number;
}

// ─── Quantized Gradients ──────────────────────────────────────────────────────────

/** Result of Int8 gradient quantization. */
export interface QuantizedGradients {
	readonly data: readonly number[];
	readonly scale: number;
	readonly min_value: number;
}
