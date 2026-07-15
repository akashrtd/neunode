import type { NeunodeClient } from "../client/client.js";

export interface IdentityCreateParams {
	name: string;
	method?: "key" | "neunode";
}

export interface IdentityCreateResult {
	identity: {
		readonly did: string;
		readonly method: string;
		readonly name: string;
		readonly ethereum: string;
		readonly peer_id: string;
	};
	readonly card_cid: string;
}

export interface IdentityShowResult {
	did: string;
	method: string;
	verification_methods: number;
	services: number;
	document: Record<string, unknown>;
}

export type IdentityListResult = ReadonlyArray<{
	readonly did: string;
	readonly status: string;
}>;

export interface IdentityExportParams {
	did?: string;
}

export interface IdentityExportResult {
	did: string;
	exported_at: string;
	did_document: Record<string, unknown>;
	verification_methods: number;
}

/** DID identity management. */
export interface IdentityResource {
	/** Create a new DID identity with Ed25519 + secp256k1 keypairs. */
	create(params: IdentityCreateParams): Promise<IdentityCreateResult>;
	/** Show the DID document for a given identity. */
	show(did?: string): Promise<IdentityShowResult>;
	/** List all local DID identities. */
	list(): Promise<IdentityListResult>;
	/** Export a portable public DID document as JSON. */
	export(params?: IdentityExportParams): Promise<IdentityExportResult>;
}

export function createIdentityResource(
	client: NeunodeClient,
): IdentityResource {
	const http = () => {
		if (!client.http)
			throw new Error("HTTP transport required for identity operations");
		return client.http;
	};
	return {
		async create(params: IdentityCreateParams): Promise<IdentityCreateResult> {
			return http().post<IdentityCreateResult>(
				"/api/v1/identity/create",
				params,
			);
		},

		async show(did?: string): Promise<IdentityShowResult> {
			const params = new URLSearchParams();
			if (did) params.set("did", did);
			const qs = params.toString();
			return http().get<IdentityShowResult>(
				qs ? `/api/v1/identity?${qs}` : "/api/v1/identity",
			);
		},

		async list(): Promise<IdentityListResult> {
			return http().get<IdentityListResult>("/api/v1/identity/list");
		},

		async export(params?: IdentityExportParams): Promise<IdentityExportResult> {
			const query = new URLSearchParams();
			if (params?.did) query.set("did", params.did);
			const qs = query.toString();
			return http().get<IdentityExportResult>(
				qs ? `/api/v1/identity/export?${qs}` : "/api/v1/identity/export",
			);
		},
	};
}
