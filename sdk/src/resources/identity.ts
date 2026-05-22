import type { NeunodeClient } from "../client/client.js";

export interface IdentityCreateParams {
	name: string;
	method?: "key" | "neunode";
	outputDir?: string;
}

export interface IdentityCreateResult {
	DID: string;
	"DID (key)": string;
	"Peer ID": string;
	Ethereum: string;
	Name: string;
	Method: string;
	Directory: string;
	"Card CID": string;
}

export interface IdentityShowResult {
	did: string;
	method: string;
	verification_methods: number;
	services: number;
	document: Record<string, unknown>;
	DID: string;
	"Verification Methods": string;
	Services: string;
}

export interface IdentityListResult {
	data: Array<{ DID: string; Status: string }>;
}

export interface IdentityExportParams {
	did?: string;
	file: string;
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
	/** Export a DID document and verification methods to a file. */
	export(params: IdentityExportParams): Promise<IdentityExportResult>;
}

export function createIdentityResource(
	client: NeunodeClient,
): IdentityResource {
	return {
		async create(params: IdentityCreateParams): Promise<IdentityCreateResult> {
			if (client.http) {
				return client.http.post<IdentityCreateResult>(
					"/api/v1/identity/create",
					params,
				);
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for identity operations");
			const args = ["identity", "create", "--name", params.name];
			if (params.method) args.push("--method", params.method);
			if (params.outputDir) args.push("--output-dir", params.outputDir);
			return cli.execute<IdentityCreateResult>(args);
		},

		async show(did?: string): Promise<IdentityShowResult> {
			if (client.http) {
				const params = new URLSearchParams();
				if (did) params.set("did", did);
				const qs = params.toString();
				return client.http.get<IdentityShowResult>(
					qs ? `/api/v1/identity?${qs}` : "/api/v1/identity",
				);
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for identity operations");
			const args = ["identity", "show"];
			if (did) args.push("--did", did);
			return cli.execute<IdentityShowResult>(args);
		},

		async list(): Promise<IdentityListResult> {
			if (client.http) {
				return client.http.get<IdentityListResult>("/api/v1/identity/list");
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for identity operations");
			return cli.execute<IdentityListResult>(["identity", "list"]);
		},

		async export(params: IdentityExportParams): Promise<IdentityExportResult> {
			// File operation — CLI only
			const cli = client.cli;
			if (!cli) throw new Error("CLI transport required for identity export (file operation)");
			const args = ["identity", "export", "--file", params.file];
			if (params.did) args.push("--did", params.did);
			return cli.execute<IdentityExportResult>(args);
		},
	};
}
