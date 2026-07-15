import type { NeunodeClient } from "../client/client.js";
import type { CID, Did } from "../types/core.js";

export type LineageContributionType =
	| "pre_training"
	| "fine_tune"
	| "merge"
	| "rl"
	| "data"
	| "compute";

export interface RegisterLineageParams {
	readonly cid: CID;
	readonly parents?: readonly CID[];
	readonly contributionType?: LineageContributionType;
	readonly loraRank?: number;
	readonly loraAlpha?: number;
}

export interface RegisterLineageResult {
	readonly cid: CID;
	readonly parent_cids: readonly CID[];
	readonly contribution_type: string;
	readonly contributor_did: Did;
	readonly created_at: number;
}

export interface LineageDetail extends RegisterLineageResult {
	readonly signature_length: number;
	readonly dataset_hash: string;
	readonly base_model_hash: string;
	readonly training_duration_secs: string | null;
}

export interface LineageModelSummary {
	readonly cid: CID;
	readonly contribution_type: string;
	readonly contributor_did: Did;
	readonly created_at: number;
}

export interface LineageDepth {
	readonly cid: CID;
	readonly lineage_depth: number;
}

export interface RoyaltyAllocation {
	readonly contributor_did: Did;
	readonly contribution_type: string;
	readonly hops: number;
	readonly weight: number;
	readonly amount_basis_points: number;
}

export interface LineageHashResult {
	readonly file: string;
	readonly hash: string;
	readonly method: "sha256" | "safetensors";
	readonly size_bytes: number;
}

export interface LineageVerifyResult {
	readonly cid: CID;
	readonly signature_valid: boolean;
	readonly fields_valid: boolean;
	readonly verified: boolean;
	readonly signature_length: number;
	readonly contributor_did: Did;
}

export interface LineageResource {
	register(params: RegisterLineageParams): Promise<RegisterLineageResult>;
	show(cid: CID): Promise<LineageDetail>;
	parents(cid: CID): Promise<readonly LineageModelSummary[]>;
	children(cid: CID): Promise<readonly LineageModelSummary[]>;
	ancestors(cid: CID): Promise<readonly LineageModelSummary[]>;
	depth(cid: CID): Promise<LineageDepth>;
	royalties(cid: CID, amount: number): Promise<readonly RoyaltyAllocation[]>;
	hash(file: string): Promise<LineageHashResult>;
	verify(cid: CID): Promise<LineageVerifyResult>;
}

export function createLineageResource(client: NeunodeClient): LineageResource {
	const http = () => {
		if (!client.http)
			throw new Error("HTTP transport required for lineage operations");
		return client.http;
	};
	const path = (cid: CID, suffix = "") =>
		`/api/v1/lineage/${encodeURIComponent(cid)}${suffix}`;
	return {
		async register(params) {
			return http().post<RegisterLineageResult>("/api/v1/lineage/register", {
				cid: params.cid,
				parents: params.parents?.join(","),
				contribution_type: params.contributionType ?? "pre_training",
				lora_rank: params.loraRank,
				lora_alpha: params.loraAlpha,
			});
		},
		async show(cid) {
			return http().get<LineageDetail>(path(cid));
		},
		async parents(cid) {
			return http().get<readonly LineageModelSummary[]>(path(cid, "/parents"));
		},
		async children(cid) {
			return http().get<readonly LineageModelSummary[]>(path(cid, "/children"));
		},
		async ancestors(cid) {
			return http().get<readonly LineageModelSummary[]>(
				path(cid, "/ancestors"),
			);
		},
		async depth(cid) {
			return http().get<LineageDepth>(path(cid, "/depth"));
		},
		async royalties(cid, amount) {
			return http().post<readonly RoyaltyAllocation[]>(
				path(cid, "/royalties"),
				{ amount },
			);
		},
		async hash(file) {
			return http().post<LineageHashResult>("/api/v1/lineage/hash", { file });
		},
		async verify(cid) {
			return http().post<LineageVerifyResult>("/api/v1/lineage/verify", {
				cid,
			});
		},
	};
}
