// @neunode/sdk — Bounty types mirroring neunode-bounty (state_machine, escrow, review, verification, lifecycle)

import type {
	BountyId,
	BountyState,
	Did,
	Hash256,
	Signature,
	Timestamp,
	TokenAmount,
	TokenType,
} from "./core.js";

export interface Deadlines {
	readonly claim: Timestamp;
	readonly work: Timestamp;
	readonly review: Timestamp;
	readonly revision: Timestamp;
	readonly dispute: Timestamp;
}

/** Full bounty data with deadlines, state, and escrow info. */
export interface BountyData {
	readonly id: BountyId;
	readonly creator: Did;
	readonly title: string;
	readonly description: string;
	readonly reward_amount: TokenAmount;
	readonly reward_token: TokenType;
	readonly state: BountyState;
	readonly claimant?: Did;
	readonly created_at: Timestamp;
	readonly deadlines: Deadlines;
	readonly artifact_hash?: Hash256;
	readonly bond?: TokenAmount;
}

/** Discriminated union of all bounty lifecycle events. */
export type BountyEvent =
	| {
			readonly type: "Claim";
			readonly claimant: Did;
			readonly bond: TokenAmount;
	  }
	| { readonly type: "Submit"; readonly artifact_hash: Hash256 }
	| { readonly type: "StartReview" }
	| {
			readonly type: "SubmitReview";
			readonly reviewer: Did;
			readonly score: number;
			readonly notes: string;
			readonly signature?: Signature;
	  }
	| { readonly type: "RequestRevision" }
	| { readonly type: "Accept" }
	| { readonly type: "Reject" }
	| { readonly type: "Dispute"; readonly reason: string }
	| { readonly type: "Resolve"; readonly accept: boolean }
	| { readonly type: "Pay" }
	| { readonly type: "Cancel" }
	| { readonly type: "Expire" };

export const EscrowState = {
	Funded: "Funded",
	Released: "Released",
	Refunded: "Refunded",
	Disputed: "Disputed",
} as const;

export type EscrowState = (typeof EscrowState)[keyof typeof EscrowState];

export interface Escrow {
	readonly amount: TokenAmount;
	readonly token_type: TokenType;
	readonly depositor: Did;
	readonly beneficiary?: Did;
	readonly state: EscrowState;
	readonly created_at: Timestamp;
}

export interface FeeBreakdown {
	readonly gross_amount: TokenAmount;
	readonly protocol_fee: TokenAmount;
	readonly reviewer_fee: TokenAmount;
	readonly net_amount: TokenAmount;
}

export interface Review {
	readonly reviewer: Did;
	readonly score: number;
	readonly notes: string;
	readonly submitted_at: Timestamp;
	readonly signature?: Signature;
}

export const ReviewOutcome = {
	Approved: "Approved",
	Rejected: "Rejected",
	NeedsRevision: "NeedsRevision",
} as const;

export type ReviewOutcome = (typeof ReviewOutcome)[keyof typeof ReviewOutcome];

export interface ReviewCommittee {
	readonly reviewers: readonly Did[];
	readonly reviews: readonly Review[];
	readonly required_count: number;
}

export const VerificationLayer = {
	Layer1: "Layer1",
	Layer2: "Layer2",
	Layer3: "Layer3",
	Layer4: "Layer4",
} as const;

export type VerificationLayer =
	(typeof VerificationLayer)[keyof typeof VerificationLayer];

export interface VerificationResult {
	readonly layer: VerificationLayer;
	readonly passed: boolean;
	readonly confidence: number;
	readonly evidence_hash: Hash256;
	readonly timestamp: Timestamp;
}

export interface VerificationPipeline {
	readonly layers: readonly VerificationLayer[];
}

export interface BountyDeadlines {
	readonly created_at: Timestamp;
	readonly claim_deadline: Timestamp;
	readonly work_deadline: Timestamp;
	readonly review_deadline: Timestamp;
	readonly revision_deadline: Timestamp;
	readonly dispute_deadline: Timestamp;
	readonly grace_period_secs: number;
}

export interface BountyRecord {
	readonly id: BountyId;
	readonly title: string;
	readonly description: string;
	readonly creator: Did;
	readonly claimant?: Did;
	readonly reward: TokenAmount;
	readonly reward_token: TokenType;
	readonly state: BountyState;
	readonly deadlines: BountyDeadlines;
	readonly reviewers: readonly Did[];
	readonly artifact_cid?: string;
	readonly created_at: Timestamp;
}
