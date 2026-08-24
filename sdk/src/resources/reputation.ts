import type { NeunodeClient } from "../client/client.js";

export interface ReputationShowResult {
	agent: string;
	score: number;
	grade: string;
	attestation_count: number;
	avg_attestation_score: number;
	factors: {
		stake: number;
		attest: number;
		activity: number;
		verify: number;
		tenure: number;
	};
}

export interface ReputationAttestParams {
	to: string;
	score: number;
	comment?: string;
}

export interface ReputationAttestResult {
	attester: string;
	target: string;
	score: number;
	comment: string;
	signed: boolean;
}

export interface ReputationLeaderboardResult {
	data: Array<{
		Rank: string;
		Agent: string;
		Score: string;
		Grade: string;
	}>;
}

export interface ReputationFactorsResult {
	agent: string;
	total_score: string;
	grade: string;
	data: Array<{
		Factor: string;
		Weight: string;
		Score: string;
	}>;
}

/** Agent reputation scores, attestations, and leaderboards. */
export interface ReputationResource {
	/** Show the reputation score for an agent (defaults to the active identity). */
	show(agent?: string): Promise<ReputationShowResult>;
	/** Attest to another agent's capabilities or work quality. */
	attest(params: ReputationAttestParams): Promise<ReputationAttestResult>;
	/** Get the reputation leaderboard. */
	leaderboard(limit?: number): Promise<ReputationLeaderboardResult>;
	/** Get a breakdown of reputation factor scores. */
	factors(agent?: string): Promise<ReputationFactorsResult>;
}

export function createReputationResource(
	client: NeunodeClient,
): ReputationResource {
	return {
		async show(agent?: string): Promise<ReputationShowResult> {
			const qs = new URLSearchParams();
			if (agent) qs.set("agent", agent);
			const query = qs.toString();
			return client.http.get<ReputationShowResult>(
				query ? `/api/v1/reputation?${query}` : "/api/v1/reputation",
			);
		},

		async attest(
			params: ReputationAttestParams,
		): Promise<ReputationAttestResult> {
			return client.http.post<ReputationAttestResult>(
				"/api/v1/reputation/attest",
				params,
			);
		},

		async leaderboard(limit?: number): Promise<ReputationLeaderboardResult> {
			const qs = new URLSearchParams();
			if (limit) qs.set("limit", String(limit));
			const query = qs.toString();
			return client.http.get<ReputationLeaderboardResult>(
				query
					? `/api/v1/reputation/leaderboard?${query}`
					: "/api/v1/reputation/leaderboard",
			);
		},

		async factors(agent?: string): Promise<ReputationFactorsResult> {
			const qs = new URLSearchParams();
			if (agent) qs.set("agent", agent);
			const query = qs.toString();
			return client.http.get<ReputationFactorsResult>(
				query
					? `/api/v1/reputation/factors?${query}`
					: "/api/v1/reputation/factors",
			);
		},
	};
}
