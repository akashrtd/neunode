// @neunode/sdk — Model lineage & training types (from research context, not yet in Rust)

import type { CID, Did, JobId, ModelId, Timestamp } from "./core.js";

export const TrainingStatus = {
	Queued: "Queued",
	Running: "Running",
	Completed: "Completed",
	Failed: "Failed",
	Cancelled: "Cancelled",
} as const;

export type TrainingStatus =
	(typeof TrainingStatus)[keyof typeof TrainingStatus];

/** How a model was contributed: pre-training, fine-tune, RL, data, compute, or serving. */
export const ContributionType = {
	PreTraining: "PreTraining",
	FineTune: "FineTune",
	RL: "RL",
	Data: "Data",
	Compute: "Compute",
	Serving: "Serving",
} as const;

export type ContributionType =
	(typeof ContributionType)[keyof typeof ContributionType];

export interface LoRAConfig {
	readonly r: number;
	readonly lora_alpha: number;
	readonly target_modules: readonly string[];
	readonly base_model: string;
}

export interface TrainingConfig {
	readonly epochs: number;
	readonly learning_rate: number;
	readonly batch_size: number;
	readonly lora?: LoRAConfig;
}

/** Content-addressed model metadata with lineage tracking. */
export interface ModelMetadata {
	readonly id: ModelId;
	readonly name: string;
	readonly cid: CID;
	readonly parent_cids: readonly CID[];
	readonly contributor_did: Did;
	readonly contribution_type: ContributionType;
	readonly base_model?: string;
	readonly parameters_count?: number;
	readonly context_length?: number;
	readonly training_config?: TrainingConfig;
	readonly created_at: Timestamp;
}

export interface TrainingJob {
	readonly id: JobId;
	readonly model_id: ModelId;
	readonly requester: Did;
	readonly status: TrainingStatus;
	readonly progress: number;
	readonly config: TrainingConfig;
	readonly started_at?: Timestamp;
	readonly completed_at?: Timestamp;
	readonly artifact_cid?: CID;
	readonly error?: string;
}

/** A signed link in the model lineage DAG, connecting a model to its parents. */
export interface ModelLineage {
	readonly model_cid: CID;
	readonly parent_cids: readonly CID[];
	readonly contributor_did: Did;
	readonly contribution_type: ContributionType;
	readonly signature: Uint8Array;
	readonly timestamp: Timestamp;
}

export interface RoyaltyRecipient {
	readonly did: Did;
	readonly weight: number;
	readonly contribution_type: ContributionType;
}

export interface RoyaltyDistribution {
	readonly model_cid: CID;
	readonly recipients: readonly RoyaltyRecipient[];
	readonly total_inference_count: number;
}
