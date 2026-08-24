import type { NeunodeClient } from "../client/client.js";

export interface BountyCreateParams {
	title: string;
	description: string;
	reward: number;
	token: string;
	claimDeadline?: number;
	workDeadline?: number;
}

export interface BountyCreateResult {
	id: string;
	title: string;
	reward: number;
	token: string;
	state: string;
	claim_deadline: number;
	work_deadline: number;
}

export interface BountyClaimParams {
	id: string;
	stake: number;
}

export interface BountyClaimResult {
	bounty_id: string;
	claimant: string;
	bond: number;
	state: string;
}

export interface BountySubmitParams {
	id: string;
	artifact: string;
	evidence?: string;
}

export interface BountySubmitResult {
	bounty_id: string;
	artifact_cid: string;
	state: string;
}

export interface BountyReviewParams {
	id: string;
	score: number;
	feedback: string;
}

export interface BountyReviewResult {
	bounty_id: string;
	score: number;
	feedback: string;
	state: string;
}

export interface BountyListParams {
	state?: string;
	creator?: string;
	limit?: number;
}

export interface BountyListItem {
	ID: string;
	State: string;
	Creator: string;
	Claimant: string;
	Reward: string;
	Deadline: string;
	Created: string;
	Escrow: string;
}

export interface BountyShowResult {
	ID: string;
	State: string;
	Creator: string;
	Claimant: string;
	Reward: string;
	Deadline: string;
	Created: string;
	Escrow: string;
}

export interface BountyCancelResult {
	bounty_id: string;
	state: string;
	reason: string;
}

export interface BountyPayResult {
	bounty_id: string;
	claimant: string;
	amount_paid: number;
	bond_returned: number;
	state: string;
}

/** Bounty marketplace for task creation, claiming, submission, and review. */
export interface BountyResource {
	/** Create a new bounty with escrowed reward. */
	create(params: BountyCreateParams): Promise<BountyCreateResult>;
	/** Claim an open bounty by staking tokens. */
	claim(params: BountyClaimParams): Promise<BountyClaimResult>;
	/** Submit work artifact for a claimed bounty. */
	submit(params: BountySubmitParams): Promise<BountySubmitResult>;
	/** Review a submitted bounty and assign a score. */
	review(params: BountyReviewParams): Promise<BountyReviewResult>;
	/** List bounties, optionally filtered by state or creator. */
	list(params?: BountyListParams): Promise<BountyListItem[]>;
	/** Show details for a specific bounty. */
	show(id: string): Promise<BountyShowResult>;
	/** Cancel an open bounty and refund escrow. */
	cancel(id: string, reason?: string): Promise<BountyCancelResult>;
	/** Settle an accepted bounty, paying its reward and returning the provider bond. */
	pay(id: string): Promise<BountyPayResult>;
}

export function createBountyResource(client: NeunodeClient): BountyResource {
	return {
		async create(params: BountyCreateParams): Promise<BountyCreateResult> {
			return client.http.post<BountyCreateResult>("/api/v1/bounties", params);
		},

		async claim(params: BountyClaimParams): Promise<BountyClaimResult> {
			return client.http.post<BountyClaimResult>(
				`/api/v1/bounties/${encodeURIComponent(params.id)}/claim`,
				{ stake: params.stake },
			);
		},

		async submit(params: BountySubmitParams): Promise<BountySubmitResult> {
			return client.http.post<BountySubmitResult>(
				`/api/v1/bounties/${encodeURIComponent(params.id)}/submit`,
				params,
			);
		},

		async review(params: BountyReviewParams): Promise<BountyReviewResult> {
			return client.http.post<BountyReviewResult>(
				`/api/v1/bounties/${encodeURIComponent(params.id)}/review`,
				params,
			);
		},

		async list(params?: BountyListParams): Promise<BountyListItem[]> {
			const qs = new URLSearchParams();
			if (params?.state) qs.set("state", params.state);
			if (params?.creator) qs.set("creator", params.creator);
			if (params?.limit) qs.set("limit", String(params.limit));
			const query = qs.toString();
			return client.http.get<BountyListItem[]>(
				query ? `/api/v1/bounties?${query}` : "/api/v1/bounties",
			);
		},

		async show(id: string): Promise<BountyShowResult> {
			return client.http.get<BountyShowResult>(
				`/api/v1/bounties/${encodeURIComponent(id)}`,
			);
		},

		async cancel(id: string, reason?: string): Promise<BountyCancelResult> {
			return client.http.post<BountyCancelResult>(
				`/api/v1/bounties/${encodeURIComponent(id)}/cancel`,
				reason ? { reason } : undefined,
			);
		},

		async pay(id: string): Promise<BountyPayResult> {
			return client.http.post<BountyPayResult>(
				`/api/v1/bounties/${encodeURIComponent(id)}/pay`,
			);
		},
	};
}
