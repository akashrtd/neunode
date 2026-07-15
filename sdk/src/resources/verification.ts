import type { NeunodeClient } from "../client/client.js";

export type AmdGeneration = "milan" | "genoa" | "turin";

export interface IntelTdxVerifyParams {
	/** Raw TDX quote encoded as hexadecimal. */
	readonly quoteHex: string;
	/** Complete Intel DCAP QuoteCollateralV3 JSON. */
	readonly collateralJson: string;
	readonly mrTd: string;
	readonly reportData: string;
	readonly nowSecs?: number;
}

interface AmdPolicyParams {
	readonly measurement: string;
	readonly reportData: string;
	readonly minimumTcb: {
		readonly bootloader: number;
		readonly tee: number;
		readonly snp: number;
		readonly microcode: number;
		readonly fmc?: number;
	};
	readonly allowSmt?: boolean;
	readonly allowMigration?: boolean;
	readonly nowSecs?: number;
}

export interface AmdSnpVerifyParams extends AmdPolicyParams {
	readonly reportHex: string;
	readonly arkHex: string;
	readonly askHex: string;
	readonly vekHex: string;
	readonly generation: AmdGeneration;
}

export interface AmdVlekVerifyParams extends AmdPolicyParams {
	readonly reportHex: string;
	readonly arkHex: string;
	readonly asvkHex: string;
	readonly vlekHex: string;
	readonly crlHex: string;
	readonly arkSha384: string;
	readonly productName: string;
	readonly cspId: string;
	readonly generation: Exclude<AmdGeneration, "turin">;
}

export interface IntelTdxClaims {
	readonly mr_td: readonly number[];
	readonly report_data: readonly number[];
	readonly tcb_status: string;
	readonly advisory_ids: readonly string[];
	readonly verified_at_secs: number;
}

export interface AmdTcbClaims {
	readonly bootloader: number;
	readonly tee: number;
	readonly snp: number;
	readonly microcode: number;
	readonly fmc: number | null;
}

export interface AmdSnpClaims {
	readonly generation: AmdGeneration;
	readonly measurement: readonly number[];
	readonly report_data: readonly number[];
	readonly chip_id: readonly number[];
	readonly reported_tcb: AmdTcbClaims;
	readonly guest_svn: number;
	readonly vmpl: number;
	readonly verified_at_secs: number;
}

export interface IntelTdxVerifyResult {
	readonly verified: true;
	readonly tee_type: "intel_tdx";
	readonly claims: IntelTdxClaims;
}

export interface AmdSnpVerifyResult {
	readonly verified: true;
	readonly tee_type: "amd_sev_snp";
	readonly claims: AmdSnpClaims;
}

export interface AmdVlekVerifyResult {
	readonly verified: true;
	readonly tee_type: "amd_sev_snp_vlek";
	readonly claims: AmdSnpClaims;
	readonly product_name: string;
	readonly csp_id: string;
}

export interface VerificationResource {
	verifyIntelTdx(params: IntelTdxVerifyParams): Promise<IntelTdxVerifyResult>;
	verifyAmdSnp(params: AmdSnpVerifyParams): Promise<AmdSnpVerifyResult>;
	verifyAmdVlek(params: AmdVlekVerifyParams): Promise<AmdVlekVerifyResult>;
}

function exactHex(value: string, bytes: number, label: string): string {
	const normalized = value.startsWith("0x") ? value.slice(2) : value;
	if (normalized.length !== bytes * 2 || !/^[0-9a-fA-F]+$/.test(normalized)) {
		throw new Error(
			`${label} must be exactly ${bytes} bytes of hexadecimal data`,
		);
	}
	return normalized.toLowerCase();
}

function unsignedInteger(value: number, label: string): number {
	if (!Number.isSafeInteger(value) || value < 0) {
		throw new Error(`${label} must be a non-negative safe integer`);
	}
	return value;
}

function amdPolicyBody(params: AmdPolicyParams) {
	const tcb = params.minimumTcb;
	return {
		measurement: exactHex(params.measurement, 48, "measurement"),
		report_data: exactHex(params.reportData, 64, "reportData"),
		minimum_tcb: {
			bootloader: unsignedInteger(tcb.bootloader, "minimumTcb.bootloader"),
			tee: unsignedInteger(tcb.tee, "minimumTcb.tee"),
			snp: unsignedInteger(tcb.snp, "minimumTcb.snp"),
			microcode: unsignedInteger(tcb.microcode, "minimumTcb.microcode"),
			...(tcb.fmc === undefined
				? {}
				: { fmc: unsignedInteger(tcb.fmc, "minimumTcb.fmc") }),
		},
		allow_smt: params.allowSmt ?? false,
		allow_migration: params.allowMigration ?? false,
		...(params.nowSecs === undefined
			? {}
			: { now_secs: unsignedInteger(params.nowSecs, "nowSecs") }),
	};
}

export function createVerificationResource(
	client: NeunodeClient,
): VerificationResource {
	const http = () => {
		if (!client.http)
			throw new Error("HTTP transport required for TEE verification");
		return client.http;
	};
	return {
		async verifyIntelTdx(params) {
			return http().post<IntelTdxVerifyResult>(
				"/api/v1/verification/tee/intel-tdx",
				{
					quote_hex: params.quoteHex,
					collateral_json: params.collateralJson,
					mr_td: exactHex(params.mrTd, 48, "mrTd"),
					report_data: exactHex(params.reportData, 64, "reportData"),
					...(params.nowSecs === undefined
						? {}
						: { now_secs: unsignedInteger(params.nowSecs, "nowSecs") }),
				},
			);
		},

		async verifyAmdSnp(params) {
			const tcb = params.minimumTcb;
			if (params.generation === "turin" && tcb.fmc === undefined) {
				throw new Error(
					"minimumTcb.fmc is required for AMD Turin verification",
				);
			}
			return http().post<AmdSnpVerifyResult>(
				"/api/v1/verification/tee/amd-snp",
				{
					report_hex: params.reportHex,
					ark_hex: params.arkHex,
					ask_hex: params.askHex,
					vek_hex: params.vekHex,
					generation: params.generation,
					...amdPolicyBody(params),
				},
			);
		},

		async verifyAmdVlek(params) {
			if (params.productName.length === 0 || params.cspId.length === 0) {
				throw new Error("productName and cspId cannot be empty");
			}
			return http().post<AmdVlekVerifyResult>(
				"/api/v1/verification/tee/amd-vlek",
				{
					report_hex: params.reportHex,
					ark_hex: params.arkHex,
					asvk_hex: params.asvkHex,
					vlek_hex: params.vlekHex,
					crl_hex: params.crlHex,
					ark_sha384: exactHex(params.arkSha384, 48, "arkSha384"),
					product_name: params.productName,
					csp_id: params.cspId,
					generation: params.generation,
					...amdPolicyBody(params),
				},
			);
		},
	};
}
