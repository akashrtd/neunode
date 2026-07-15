import type { NeunodeClient } from "../client/client.js";

export type AmdGeneration = "milan" | "genoa" | "turin";

export interface IntelTdxVerifyParams {
	readonly quote: string;
	readonly collateral: string;
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
	readonly report: string;
	readonly ark: string;
	readonly ask: string;
	readonly vek: string;
	readonly generation: AmdGeneration;
}

export interface AmdVlekVerifyParams extends AmdPolicyParams {
	readonly report: string;
	readonly ark: string;
	readonly asvk: string;
	readonly vlek: string;
	readonly crl: string;
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

function cli(client: NeunodeClient) {
	if (!client.cli) {
		throw new Error("CLI transport required for production TEE verification");
	}
	return client.cli;
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

function unsignedInteger(value: number, label: string): string {
	if (!Number.isSafeInteger(value) || value < 0) {
		throw new Error(`${label} must be a non-negative safe integer`);
	}
	return String(value);
}

function amdPolicyArgs(params: AmdPolicyParams): string[] {
	const tcb = params.minimumTcb;
	const args = [
		"--measurement",
		exactHex(params.measurement, 48, "measurement"),
		"--report-data",
		exactHex(params.reportData, 64, "reportData"),
		"--min-bootloader",
		unsignedInteger(tcb.bootloader, "minimumTcb.bootloader"),
		"--min-tee",
		unsignedInteger(tcb.tee, "minimumTcb.tee"),
		"--min-snp",
		unsignedInteger(tcb.snp, "minimumTcb.snp"),
		"--min-microcode",
		unsignedInteger(tcb.microcode, "minimumTcb.microcode"),
	];
	if (tcb.fmc !== undefined) {
		args.push("--min-fmc", unsignedInteger(tcb.fmc, "minimumTcb.fmc"));
	}
	if (params.allowSmt) args.push("--allow-smt");
	if (params.allowMigration) args.push("--allow-migration");
	if (params.nowSecs !== undefined) {
		args.push("--now-secs", unsignedInteger(params.nowSecs, "nowSecs"));
	}
	return args;
}

export function createVerificationResource(
	client: NeunodeClient,
): VerificationResource {
	return {
		async verifyIntelTdx(params) {
			const args = [
				"verify",
				"tee",
				"intel",
				"--quote",
				params.quote,
				"--collateral",
				params.collateral,
				"--mr-td",
				exactHex(params.mrTd, 48, "mrTd"),
				"--report-data",
				exactHex(params.reportData, 64, "reportData"),
			];
			if (params.nowSecs !== undefined) {
				args.push("--now-secs", unsignedInteger(params.nowSecs, "nowSecs"));
			}
			return cli(client).execute<IntelTdxVerifyResult>(args);
		},

		async verifyAmdSnp(params) {
			const tcb = params.minimumTcb;
			if (params.generation === "turin" && tcb.fmc === undefined) {
				throw new Error(
					"minimumTcb.fmc is required for AMD Turin verification",
				);
			}
			const args = [
				"verify",
				"tee",
				"amd",
				"--report",
				params.report,
				"--ark",
				params.ark,
				"--ask",
				params.ask,
				"--vek",
				params.vek,
				"--generation",
				params.generation,
				...amdPolicyArgs(params),
			];
			return cli(client).execute<AmdSnpVerifyResult>(args);
		},

		async verifyAmdVlek(params) {
			if (params.productName.length === 0 || params.cspId.length === 0) {
				throw new Error("productName and cspId cannot be empty");
			}
			return cli(client).execute<AmdVlekVerifyResult>([
				"verify",
				"tee",
				"amd-vlek",
				"--report",
				params.report,
				"--ark",
				params.ark,
				"--asvk",
				params.asvk,
				"--vlek",
				params.vlek,
				"--crl",
				params.crl,
				"--ark-sha384",
				exactHex(params.arkSha384, 48, "arkSha384"),
				"--product-name",
				params.productName,
				"--csp-id",
				params.cspId,
				"--generation",
				params.generation,
				...amdPolicyArgs(params),
			]);
		},
	};
}
