// @neunode/sdk — Reputation types mirroring neunode-reputation (score, factors, attestation)

import type {
	Did,
	Hash256,
	Signature,
	Timestamp,
	TokenAmount,
} from "./core.js";

export const ReputationGrade = {
	A: "A",
	B: "B",
	C: "C",
	D: "D",
	F: "F",
} as const;

export type ReputationGrade =
	(typeof ReputationGrade)[keyof typeof ReputationGrade];

export interface FactorInputs {
	readonly staked_amount: TokenAmount;
	readonly total_staked: TokenAmount;
	readonly attestation_count: number;
	readonly avg_attestation_score: number;
	readonly events_per_day: number;
	readonly days_active: number;
	readonly tasks_completed: number;
	readonly tasks_failed: number;
	readonly days_since_creation: number;
}

export interface ReputationScore {
	readonly total: number;
	readonly stake_factor: number;
	readonly attest_factor: number;
	readonly activity_factor: number;
	readonly verify_factor: number;
	readonly tenure_factor: number;
	readonly updated_at: Timestamp;
}

export interface FactorWeights {
	readonly stake: number;
	readonly attest: number;
	readonly activity: number;
	readonly verify: number;
	readonly tenure: number;
}

export interface ReputationAttestation {
	readonly attester: Did;
	readonly target: Did;
	readonly claim: string;
	readonly score: number;
	readonly evidence_hash: Hash256;
	readonly timestamp: Timestamp;
	readonly signature?: Signature;
}
