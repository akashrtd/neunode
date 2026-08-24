import { describe, expect, it, vi } from "vitest";
import { createNeunodeClient } from "../client/client.js";

const policy = {
	measurement: "11".repeat(48),
	reportData: "22".repeat(64),
	minimumTcb: { bootloader: 3, tee: 0, snp: 8, microcode: 115 },
} as const;

describe("VerificationResource", () => {
	it("posts strict Intel TDX evidence over HTTP", async () => {
		const client = createNeunodeClient({
			http: { baseUrl: "http://localhost:8080" },
		});
		const http = client.http;
		if (!http) throw new Error("expected HTTP transport");
		const post = vi.spyOn(http, "post").mockResolvedValue({ verified: true });

		await client.verification.verifyIntelTdx({
			quoteHex: "aa55",
			collateralJson: "{}",
			mrTd: `0x${"11".repeat(48)}`,
			reportData: "22".repeat(64),
			nowSecs: 1_751_000_000,
		});

		expect(post).toHaveBeenCalledWith("/api/v1/verification/tee/intel-tdx", {
			quote_hex: "aa55",
			collateral_json: "{}",
			mr_td: "11".repeat(48),
			report_data: "22".repeat(64),
			now_secs: 1_751_000_000,
		});
	});

	it("keeps AMD SMT and migration disabled by default", async () => {
		const client = createNeunodeClient({
			http: { baseUrl: "http://localhost:8080" },
		});
		const http = client.http;
		if (!http) throw new Error("expected HTTP transport");
		const post = vi.spyOn(http, "post").mockResolvedValue({ verified: true });
		await client.verification.verifyAmdSnp({
			reportHex: "01",
			arkHex: "02",
			askHex: "03",
			vekHex: "04",
			generation: "milan",
			...policy,
		});
		expect(post).toHaveBeenCalledWith(
			"/api/v1/verification/tee/amd-snp",
			expect.objectContaining({ allow_smt: false, allow_migration: false }),
		);
	});

	it("rejects malformed policy values before HTTP dispatch", async () => {
		const client = createNeunodeClient({
			http: { baseUrl: "http://localhost:8080" },
		});
		const http = client.http;
		if (!http) throw new Error("expected HTTP transport");
		const post = vi.spyOn(http, "post");
		await expect(
			client.verification.verifyIntelTdx({
				quoteHex: "00",
				collateralJson: "{}",
				mrTd: "11",
				reportData: "22".repeat(64),
			}),
		).rejects.toThrow("mrTd must be exactly 48 bytes");
		expect(post).not.toHaveBeenCalled();
	});

	it("requires an FMC floor for Turin", async () => {
		const client = createNeunodeClient({
			http: { baseUrl: "http://localhost:8080" },
		});
		await expect(
			client.verification.verifyAmdSnp({
				reportHex: "01",
				arkHex: "02",
				askHex: "03",
				vekHex: "04",
				generation: "turin",
				...policy,
			}),
		).rejects.toThrow("minimumTcb.fmc is required");
	});

	it("posts separately pinned AMD VLEK evidence", async () => {
		const client = createNeunodeClient({
			http: { baseUrl: "http://localhost:8080" },
		});
		const http = client.http;
		if (!http) throw new Error("expected HTTP transport");
		const post = vi.spyOn(http, "post").mockResolvedValue({ verified: true });
		await client.verification.verifyAmdVlek({
			reportHex: "01",
			arkHex: "02",
			asvkHex: "03",
			vlekHex: "04",
			crlHex: "05",
			arkSha384: "aa".repeat(48),
			productName: "Milan-B0",
			cspId: "cloud.example",
			generation: "milan",
			...policy,
		});
		expect(post).toHaveBeenCalledWith(
			"/api/v1/verification/tee/amd-vlek",
			expect.objectContaining({
				ark_sha384: "aa".repeat(48),
				product_name: "Milan-B0",
				csp_id: "cloud.example",
			}),
		);
	});
});
