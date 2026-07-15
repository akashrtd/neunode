/**
 * Integration tests for CliTransport against the real `agnetd` binary.
 *
 * These tests spawn actual `agnetd` processes and verify JSON envelope
 * responses end-to-end. The entire suite is skipped when the binary
 * is not available (e.g., CI without a Rust build step).
 */

import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
	CliTransport,
	CliTransportError,
} from "../../src/transport/cli-transport.js";
import { BINARY_PATH } from "./helpers/agnetd.js";

const INTEGRATION_HOME = mkdtempSync(
	join(tmpdir(), "neunode-cli-integration-"),
);
const INTEGRATION_ENV = { ...process.env, HOME: INTEGRATION_HOME };

afterAll(() => {
	rmSync(INTEGRATION_HOME, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// Shared transport instance (uses discovered binary path)
// ---------------------------------------------------------------------------

function makeTransport(): CliTransport {
	return new CliTransport({
		binaryPath: BINARY_PATH ?? "agnetd",
		timeout: 15_000,
		env: INTEGRATION_ENV,
	});
}

// ===========================================================================
// A. Binary & Envelope Format
// ===========================================================================

describe.sequential("Integration: Binary & Envelope Format", () => {
	const transport = makeTransport();

	it("should find agnetd binary", () => {
		expect(BINARY_PATH).not.toBeNull();
	});

	it("should return JSON envelope with success=true for config list", async () => {
		const result = await transport.execute<Record<string, unknown>>([
			"config",
			"list",
		]);
		expect(result).toBeDefined();
		expect(typeof result).toBe("object");
	});

	it("should parse data field from envelope with expected config keys", async () => {
		const result = await transport.execute<Record<string, unknown>>([
			"config",
			"list",
		]);
		expect(result).toHaveProperty("agent.name");
		expect(result).toHaveProperty("storage.db_path");
	});
});

// ===========================================================================
// B. Identity Commands
// ===========================================================================

describe.sequential("Integration: Identity Commands", () => {
	const transport = makeTransport();
	let createdDid: string;

	it("should create a new identity", async () => {
		const ts = Date.now();
		const result = await transport.execute<Record<string, string>>([
			"identity",
			"create",
			"--name",
			`integration-test-${ts}`,
		]);
		expect(result.DID).toBeDefined();
		expect(result.DID).toMatch(/^did:neunode:/);
		createdDid = result.DID;
	});

	it("should list identities", async () => {
		// identity list returns a single envelope with data as an array.
		// parseSingleEnvelope spreads arrays into indexed objects, so we use
		// executeRaw + manual parsing to preserve the array structure.
		const raw = await transport.executeRaw([
			"--output",
			"json-compact",
			"identity",
			"list",
		]);
		const envelope = JSON.parse(raw.trim()) as {
			data: Record<string, string>[];
			success: boolean;
		};
		expect(envelope.success).toBe(true);
		expect(Array.isArray(envelope.data)).toBe(true);
		expect(envelope.data.length).toBeGreaterThanOrEqual(1);
		const entry = envelope.data[0];
		expect(entry).toHaveProperty("DID");
		expect(entry).toHaveProperty("Status");
	});

	it("should show identity details (multi-envelope merge)", async () => {
		// `identity show` returns 2 envelopes; execute<T> merges them
		const result = await transport.execute<Record<string, unknown>>([
			"identity",
			"show",
			"--did",
			createdDid,
		]);
		expect(result).toHaveProperty("did");
		expect(result).toHaveProperty("document");
		expect(result).toHaveProperty("verification_methods");
	});

	it("should honor the global identity override over the configured identity", async () => {
		const second = await transport.execute<Record<string, string>>([
			"identity",
			"create",
			"--name",
			`override-control-${Date.now()}`,
		]);
		expect(second.DID).not.toBe(createdDid);

		const overridden = new CliTransport({
			binaryPath: BINARY_PATH ?? "agnetd",
			timeout: 15_000,
			env: INTEGRATION_ENV,
			identity: createdDid,
		});
		const result = await overridden.execute<{ did: string }>([
			"identity",
			"show",
		]);
		expect(result.did).toBe(createdDid);
	});
});

// ===========================================================================
// C. Config Commands
// ===========================================================================

describe.sequential("Integration: Config Commands", () => {
	const transport = makeTransport();

	it("should list all config values", async () => {
		const result = await transport.execute<Record<string, unknown>>([
			"config",
			"list",
		]);
		expect(result).toHaveProperty("agent.name");
		expect(result).toHaveProperty("storage.db_path");
	});

	it("should get a single config value", async () => {
		const result = await transport.execute<Record<string, unknown>>([
			"config",
			"get",
			"agent.name",
		]);
		expect(result).toHaveProperty("agent.name");
	});

	it("should set, get, and restore the active identity", async () => {
		const original = await transport.execute<Record<string, string>>([
			"config",
			"get",
			"active_identity",
		]);
		const originalDid = original.active_identity ?? "";
		const configuredDid = "did:neunode:config-integration";

		await transport.executeRaw([
			"config",
			"set",
			"active_identity",
			configuredDid,
		]);
		try {
			const updated = await transport.execute<Record<string, string>>([
				"config",
				"get",
				"active_identity",
			]);
			expect(updated.active_identity).toBe(configuredDid);
		} finally {
			await transport.executeRaw([
				"config",
				"set",
				"active_identity",
				originalDid,
			]);
		}
	});
});

// ===========================================================================
// D. Bounty Commands
// ===========================================================================

describe.sequential("Integration: Bounty Commands", () => {
	const transport = makeTransport();

	beforeAll(async () => {
		await transport.executeRaw([
			"config",
			"set",
			"tokens.unbonding_period_secs",
			"0",
		]);
		await transport.execute(["token", "seed"]);
		await transport.execute(["token", "unstake", "--amount", "100"]);
		const claimed = await transport.execute<{ claimed_amount: number }>([
			"token",
			"claim-unbonded",
		]);
		expect(claimed.claimed_amount).toBe(100);
		const balance = await transport.execute<Record<string, string>>([
			"token",
			"balance",
			"--token",
			"compute",
		]);
		expect(balance.balance).toBe("100");
	});

	it("should create a bounty", async () => {
		const result = await transport.execute<Record<string, unknown>>([
			"bounty",
			"create",
			"--title",
			"Integration test bounty",
			"--description",
			"Created by integration test",
			"--reward",
			"100",
			"--work-deadline",
			"259200",
		]);
		expect(result).toHaveProperty("id");
		expect((result as Record<string, unknown>).id).toMatch(/^bnty_/);
		expect(result).toHaveProperty("state", "Open");
		expect(result).toHaveProperty("reward", 100);
		expect(result).toHaveProperty("title", "Integration test bounty");
	});

	it("should list bounties", async () => {
		// bounty list returns a single envelope with data as an array.
		// parseSingleEnvelope spreads arrays into indexed objects, so we use
		// executeRaw + manual parsing to preserve the array structure.
		const raw = await transport.executeRaw([
			"--output",
			"json-compact",
			"bounty",
			"list",
		]);
		const envelope = JSON.parse(raw.trim()) as {
			data: Record<string, unknown>[];
			success: boolean;
		};
		expect(envelope.success).toBe(true);
		expect(Array.isArray(envelope.data)).toBe(true);
		expect(envelope.data.length).toBeGreaterThanOrEqual(1);
		const entry = envelope.data[0];
		expect(entry).toHaveProperty("ID");
		expect(entry).toHaveProperty("State");
		expect(entry).toHaveProperty("Creator");
		expect(entry).toHaveProperty("Reward");
	});
});

// ===========================================================================
// E. Token Commands (canonical wire contract)
// ===========================================================================

describe.sequential("Integration: Token Commands (canonical wire contract)", () => {
	const transport = makeTransport();

	it("should return one lossless balance object", async () => {
		const result = await transport.execute<{
			token: string;
			balance: string;
			staked: string;
		}>(["token", "balance", "--token", "nCompute"]);
		expect(result).toEqual({ token: "nCompute", balance: "0", staked: "0" });
	});

	it("should return all balances in the same canonical shape", async () => {
		const result = await transport.execute<{
			balances: Array<{ token: string; balance: string; staked: string }>;
		}>(["token", "balance"]);

		expect(result.balances).toHaveLength(4);
		expect(result.balances[0]).toEqual({
			token: "nCompute",
			balance: "0",
			staked: "0",
		});
	});
});

// ===========================================================================
// F. TurboQuant transport parity
// ===========================================================================

describe.sequential("Integration: TurboQuant Commands", () => {
	const transport = makeTransport();

	it("selects the same compression strategy exposed over HTTP", async () => {
		const result = await transport.execute<{
			strategy: string;
			bits?: number;
		}>([
			"turboquant",
			"compress",
			"--profile",
			"kv_cache",
			"--dimension",
			"4096",
			"--target-bits",
			"3.5",
		]);
		expect(result).toEqual({ strategy: "mse", bits: 3.5 });
	});

	it("generates codebooks through the CLI transport", async () => {
		const result = await transport.execute<{
			bits: number;
			levels: number[];
			dimension: number;
		}>([
			"turboquant",
			"generate-codebook",
			"--bits",
			"2",
			"--dimension",
			"256",
			"--num-samples",
			"32",
		]);
		expect(result.bits).toBe(2);
		expect(result.levels).toHaveLength(4);
		expect(result.dimension).toBe(256);
	});
});

// ===========================================================================
// G. Reputation Commands
// ===========================================================================

describe.sequential("Integration: Reputation Commands", () => {
	const transport = makeTransport();
	let agentDid: string;

	it("should create identity for reputation test", async () => {
		const ts = Date.now();
		const result = await transport.execute<Record<string, string>>([
			"identity",
			"create",
			"--name",
			`rep-test-${ts}`,
		]);
		agentDid = result.DID;
	});

	it("should show reputation for an agent", async () => {
		const result = await transport.execute<Record<string, unknown>>([
			"reputation",
			"show",
			"--agent",
			agentDid,
		]);
		expect(result).toHaveProperty("score");
		expect(result).toHaveProperty("grade");
		expect(result).toHaveProperty("factors");
		const factors = result.factors as Record<string, unknown>;
		expect(factors).toHaveProperty("stake");
		expect(factors).toHaveProperty("attest");
		expect(factors).toHaveProperty("activity");
		expect(factors).toHaveProperty("verify");
		expect(factors).toHaveProperty("tenure");
	});
});

// ===========================================================================
// G. Feed Commands
// ===========================================================================

describe.sequential("Integration: Feed Commands", () => {
	const transport = makeTransport();

	it("should post to feed", async () => {
		const result = await transport.execute<Record<string, string>>([
			"feed",
			"post",
			"--kind",
			"1000",
			"--content",
			'{"title":"integration test"}',
		]);
		expect(result).toHaveProperty("Author");
		expect(result).toHaveProperty("Event ID");
		expect(result).toHaveProperty("Kind");
		expect(result).toHaveProperty("Topic");
		expect(result).toHaveProperty("Sequence");
	});
});

// ===========================================================================
// H. Error Handling
// ===========================================================================

describe.sequential("Integration: Error Handling", () => {
	const transport = makeTransport();

	it("should throw CliTransportError for invalid identity", async () => {
		try {
			await transport.execute(["identity", "show", "--did", "nonexistent"]);
			expect.unreachable("Expected CliTransportError to be thrown");
		} catch (err) {
			expect(err).toBeInstanceOf(CliTransportError);
			const transportErr = err as CliTransportError;
			expect(transportErr.stderr).toContain("not found");
		}
	});

	it("should throw CliTransportError for invalid subcommand", async () => {
		await expect(
			transport.execute([
				"model",
				"register",
				"--name",
				"test",
				"--cid",
				"abc",
			]),
		).rejects.toThrow(CliTransportError);
	});
});
