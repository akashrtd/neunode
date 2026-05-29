import type { NeunodeClient } from "../client/client.js";

export interface DiscoverySearchParams {
	capabilities: string;
	minReputation?: number;
	maxCost?: number;
	onlineOnly?: boolean;
	limit?: number;
}

export interface DiscoveryComplementParams {
	capabilities: string;
	limit?: number;
}

export interface DiscoveryScoreParams {
	agent: string;
	capabilities: string;
}

export interface ScoredAgentResult {
	candidate: {
		did: string;
		capabilities: string[];
		reputation_score: number;
		stake_amount: number;
		availability_score: number;
		latency_ms: number;
		cost_per_unit: number;
		is_online: boolean;
	};
	final_score: number;
	capability_score: number;
	quality_score: number;
	availability_score: number;
	cost_score: number;
	complementarity_score: number;
}

export interface ComplementAgentResult {
	candidate: {
		did: string;
		capabilities: string[];
		reputation_score: number;
		stake_amount: number;
		availability_score: number;
		latency_ms: number;
		cost_per_unit: number;
		is_online: boolean;
	};
	complementarity_score: number;
}

export interface DiscoverySearchResult {
	data: ScoredAgentResult[];
}

export interface DiscoveryComplementResult {
	data: ComplementAgentResult[];
}

export interface DiscoveryGapsResult {
	data: Array<{ capability_uri: string; demand_count: number }>;
}

export interface DiscoveryScoreResult {
	did: string;
	final_score: string;
	capability: string;
	quality: string;
	availability: string;
	cost_efficiency: string;
	complementarity: string;
}

export interface DiscoveryWeightsResult {
	data: Array<{ factor: string; weight: string; pct: string }>;
}

/** Agent discovery: search, complement analysis, capability gaps, scoring. */
export interface DiscoveryResource {
	/** Search for agents matching required capabilities with filters. */
	search(params: DiscoverySearchParams): Promise<DiscoverySearchResult>;
	/** Find agents with complementary capabilities to the requester. */
	complement(
		params: DiscoveryComplementParams,
	): Promise<DiscoveryComplementResult>;
	/** Find capability gaps — capabilities with demand but no providers. */
	gaps(): Promise<DiscoveryGapsResult>;
	/** Score a specific agent against required capabilities. */
	score(params: DiscoveryScoreParams): Promise<DiscoveryScoreResult>;
	/** Show current discovery scoring weights. */
	weights(): Promise<DiscoveryWeightsResult>;
}

export function createDiscoveryResource(
	client: NeunodeClient,
): DiscoveryResource {
	return {
		async search(
			params: DiscoverySearchParams,
		): Promise<DiscoverySearchResult> {
			if (client.http) {
				const qs = new URLSearchParams();
				qs.set("capabilities", params.capabilities);
				if (params.minReputation !== undefined && params.minReputation > 0) {
					qs.set("minReputation", String(params.minReputation));
				}
				if (params.maxCost !== undefined) {
					qs.set("maxCost", String(params.maxCost));
				}
				if (params.onlineOnly) qs.set("onlineOnly", "true");
				if (params.limit !== undefined) {
					qs.set("limit", String(params.limit));
				}
				return client.http.get<DiscoverySearchResult>(
					`/api/v1/discovery/search?${qs.toString()}`,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for discovery operations",
				);
			const args = [
				"discover",
				"search",
				"--capabilities",
				params.capabilities,
			];
			if (params.minReputation !== undefined && params.minReputation > 0) {
				args.push("--min-reputation", String(params.minReputation));
			}
			if (params.maxCost !== undefined) {
				args.push("--max-cost", String(params.maxCost));
			}
			if (params.onlineOnly) args.push("--online-only");
			if (params.limit !== undefined) {
				args.push("--limit", String(params.limit));
			}
			return cli.execute<DiscoverySearchResult>(args);
		},

		async complement(
			params: DiscoveryComplementParams,
		): Promise<DiscoveryComplementResult> {
			if (client.http) {
				const qs = new URLSearchParams();
				qs.set("capabilities", params.capabilities);
				if (params.limit !== undefined) {
					qs.set("limit", String(params.limit));
				}
				return client.http.get<DiscoveryComplementResult>(
					`/api/v1/discovery/complement?${qs.toString()}`,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for discovery operations",
				);
			const args = [
				"discover",
				"complement",
				"--capabilities",
				params.capabilities,
			];
			if (params.limit !== undefined) {
				args.push("--limit", String(params.limit));
			}
			return cli.execute<DiscoveryComplementResult>(args);
		},

		async gaps(): Promise<DiscoveryGapsResult> {
			if (client.http) {
				return client.http.get<DiscoveryGapsResult>("/api/v1/discovery/gaps");
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for discovery operations",
				);
			return cli.execute<DiscoveryGapsResult>(["discover", "gaps"]);
		},

		async score(params: DiscoveryScoreParams): Promise<DiscoveryScoreResult> {
			if (client.http) {
				const qs = new URLSearchParams();
				qs.set("agent", params.agent);
				qs.set("capabilities", params.capabilities);
				return client.http.get<DiscoveryScoreResult>(
					`/api/v1/discovery/score?${qs.toString()}`,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for discovery operations",
				);
			const args = [
				"discover",
				"score",
				"--agent",
				params.agent,
				"--capabilities",
				params.capabilities,
			];
			return cli.execute<DiscoveryScoreResult>(args);
		},

		async weights(): Promise<DiscoveryWeightsResult> {
			if (client.http) {
				return client.http.get<DiscoveryWeightsResult>(
					"/api/v1/discovery/weights",
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"HTTP or CLI transport required for discovery operations",
				);
			return cli.execute<DiscoveryWeightsResult>(["discover", "weights"]);
		},
	};
}
