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
	const cli = client.cli;
	if (!cli) throw new Error("CLI transport required for reputation operations");

	return {
		async show(agent?: string): Promise<ReputationShowResult> {
			const args = ["reputation", "show"];
			if (agent) args.push("--agent", agent);
			return cli.execute<ReputationShowResult>(args);
		},

		async attest(
			params: ReputationAttestParams,
		): Promise<ReputationAttestResult> {
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
			const args = ["reputation", "leaderboard"];
			if (limit) args.push("--limit", String(limit));
			return cli.execute<ReputationLeaderboardResult>(args);
		},

		async factors(agent?: string): Promise<ReputationFactorsResult> {
			const args = ["reputation", "factors"];
			if (agent) args.push("--agent", agent);
			return cli.execute<ReputationFactorsResult>(args);
		},
	};
}
