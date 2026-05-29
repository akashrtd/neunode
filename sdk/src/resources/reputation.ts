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
			if (client.http) {
				const qs = new URLSearchParams();
				if (agent) qs.set("agent", agent);
				const query = qs.toString();
				return client.http.get<ReputationShowResult>(
					query ? `/api/v1/reputation?${query}` : "/api/v1/reputation",
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for reputation operations",
				);
			const args = ["reputation", "show"];
			if (agent) args.push("--agent", agent);
			return cli.execute<ReputationShowResult>(args);
		},

		async attest(
			params: ReputationAttestParams,
		): Promise<ReputationAttestResult> {
			if (client.http) {
				return client.http.post<ReputationAttestResult>(
					"/api/v1/reputation/attest",
					params,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for reputation operations",
				);
			const args = [
				"reputation",
				"attest",
				"--to",
				params.to,
				"--score",
				String(params.score),
			];
			if (params.comment) args.push("--comment", params.comment);
			return cli.execute<ReputationAttestResult>(args);
		},

		async leaderboard(limit?: number): Promise<ReputationLeaderboardResult> {
			if (client.http) {
				const qs = new URLSearchParams();
				if (limit) qs.set("limit", String(limit));
				const query = qs.toString();
				return client.http.get<ReputationLeaderboardResult>(
					query
						? `/api/v1/reputation/leaderboard?${query}`
						: "/api/v1/reputation/leaderboard",
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for reputation operations",
				);
			const args = ["reputation", "leaderboard"];
			if (limit) args.push("--limit", String(limit));
			return cli.execute<ReputationLeaderboardResult>(args);
		},

		async factors(agent?: string): Promise<ReputationFactorsResult> {
			if (client.http) {
				const qs = new URLSearchParams();
				if (agent) qs.set("agent", agent);
				const query = qs.toString();
				return client.http.get<ReputationFactorsResult>(
					query
						? `/api/v1/reputation/factors?${query}`
						: "/api/v1/reputation/factors",
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for reputation operations",
				);
			const args = ["reputation", "factors"];
			if (agent) args.push("--agent", agent);
			return cli.execute<ReputationFactorsResult>(args);
		},
	};
}
