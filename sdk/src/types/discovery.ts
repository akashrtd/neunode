// @neunode/sdk — Discovery types mirroring neunode-discovery crate
// (types, scoring, search, gap, complement)

// ─── Scoring Weights ──────────────────────────────────────────────────────────────

/** 5-factor scoring weights for discovery ranking.
 *  Each weight must be in [0, 1] and all weights must sum to 1.0. */
export interface ScoringWeights {
	readonly capability_match: number;
	readonly quality: number;
	readonly availability: number;
	readonly cost_efficiency: number;
	readonly complementarity: number;
}

// ─── Agent Candidate ──────────────────────────────────────────────────────────────

/** An agent being evaluated as a discovery candidate. */
export interface AgentCandidate {
	readonly did: string;
	readonly capabilities: readonly string[];
	readonly reputation_score: number;
	readonly stake_amount: number;
	readonly availability_score: number;
	readonly latency_ms: number;
	readonly cost_per_unit: number;
	readonly is_online: boolean;
}

// ─── Scored Agent ──────────────────────────────────────────────────────────────────

/** A single scored result from discovery search with factor breakdown. */
export interface ScoredAgent {
	readonly candidate: AgentCandidate;
	readonly final_score: number;
	readonly capability_score: number;
	readonly quality_score: number;
	readonly availability_score: number;
	readonly cost_score: number;
	readonly complementarity_score: number;
}

// ─── Discovery Request ────────────────────────────────────────────────────────────

/** Discovery search request specifying what capabilities are needed. */
export interface DiscoveryRequest {
	readonly required_capabilities: readonly string[];
	readonly min_reputation?: number;
	readonly max_cost_per_unit?: number;
	readonly must_be_online: boolean;
	readonly max_results: number;
	readonly requester_capabilities: readonly string[];
}

// ─── Capability Gap ───────────────────────────────────────────────────────────────

/** A capability with no available providers. */
export interface CapabilityGap {
	readonly capability_uri: string;
	readonly demand_count: number;
}

// ─── Complement Result ─────────────────────────────────────────────────────────────

/** An agent ranked by complementarity (Jaccard distance) to the requester. */
export interface ComplementResult {
	readonly candidate: AgentCandidate;
	readonly complementarity_score: number;
}

// ─── Score Result ──────────────────────────────────────────────────────────────────

/** Single agent scoring result with all factor breakdowns. */
export interface AgentScoreResult {
	readonly did: string;
	readonly final_score: string;
	readonly capability: string;
	readonly quality: string;
	readonly availability: string;
	readonly cost_efficiency: string;
	readonly complementarity: string;
}
