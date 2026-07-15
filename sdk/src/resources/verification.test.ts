import { describe, expect, it, vi } from "vitest";
import { createNeunodeClient } from "../client/client.js";

describe("VerificationResource", () => {
	it("builds a strict Intel TDX verification command", async () => {
		const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });
		const cli = client.cli;
		if (!cli) throw new Error("expected CLI transport");
		const execute = vi
			.spyOn(cli, "execute")
			.mockResolvedValue({ verified: true });

		await client.verification.verifyIntelTdx({
			quote: "quote.bin",
			collateral: "collateral.json",
			mrTd: `0x${"11".repeat(48)}`,
			reportData: "22".repeat(64),
			nowSecs: 1_751_000_000,
		});

		expect(execute).toHaveBeenCalledWith([
			"verify",
			"tee",
			"intel",
			"--quote",
			"quote.bin",
			"--collateral",
			"collateral.json",
			"--mr-td",
			"11".repeat(48),
			"--report-data",
			"22".repeat(64),
			"--now-secs",
			"1751000000",
		]);
	});

	it("keeps AMD SMT and migration disabled unless explicitly requested", async () => {
		const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });
		const cli = client.cli;
		if (!cli) throw new Error("expected CLI transport");
		const execute = vi
			.spyOn(cli, "execute")
			.mockResolvedValue({ verified: true });

		await client.verification.verifyAmdSnp({
			report: "report.bin",
			ark: "ark.der",
			ask: "ask.der",
			vek: "vcek.der",
			generation: "milan",
			measurement: "11".repeat(48),
			reportData: "22".repeat(64),
			minimumTcb: { bootloader: 3, tee: 0, snp: 8, microcode: 115 },
		});

		const args = execute.mock.calls[0]?.[0] as readonly string[];
		expect(args).not.toContain("--allow-smt");
		expect(args).not.toContain("--allow-migration");
		expect(args).toContain("--min-microcode");
	});

	it("rejects malformed policy values before invoking the CLI", async () => {
		const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });
		const cli = client.cli;
		if (!cli) throw new Error("expected CLI transport");
		const execute = vi.spyOn(cli, "execute");

		await expect(
			client.verification.verifyIntelTdx({
				quote: "quote.bin",
				collateral: "collateral.json",
				mrTd: "11",
				reportData: "22".repeat(64),
			}),
		).rejects.toThrow("mrTd must be exactly 48 bytes");
		expect(execute).not.toHaveBeenCalled();
	});

	it("requires an FMC floor for Turin", async () => {
		const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });

		await expect(
			client.verification.verifyAmdSnp({
				report: "report.bin",
				ark: "ark.der",
				ask: "ask.der",
				vek: "vcek.der",
				generation: "turin",
				measurement: "11".repeat(48),
				reportData: "22".repeat(64),
				minimumTcb: { bootloader: 1, tee: 1, snp: 1, microcode: 1 },
			}),
		).rejects.toThrow("minimumTcb.fmc is required");
	});

	it("requires the CLI transport", async () => {
		const client = createNeunodeClient({
			mock: { responses: {} },
		});
		await expect(
			client.verification.verifyIntelTdx({
				quote: "quote.bin",
				collateral: "collateral.json",
				mrTd: "11".repeat(48),
				reportData: "22".repeat(64),
			}),
		).rejects.toThrow("CLI transport required");
	});
});
