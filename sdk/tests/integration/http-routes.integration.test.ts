import type { ChildProcess } from "node:child_process";
import { execFile } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { createNeunodeClient, type NeunodeClient } from "../../src/index.js";
import { BINARY_PATH } from "./helpers/agnetd.js";

const execFileAsync = promisify(execFile);

async function availablePort(): Promise<number> {
	return new Promise((resolve, reject) => {
		const server = createServer();
		server.once("error", reject);
		server.listen(0, "127.0.0.1", () => {
			const address = server.address();
			if (!address || typeof address === "string") {
				server.close();
				reject(new Error("failed to allocate HTTP test port"));
				return;
			}
			server.close((error) => {
				if (error) reject(error);
				else resolve(address.port);
			});
		});
	});
}

async function waitForHealth(baseUrl: string): Promise<void> {
	const deadline = Date.now() + 15_000;
	while (Date.now() < deadline) {
		try {
			const response = await fetch(`${baseUrl}/api/v1/health`);
			if (response.ok) return;
		} catch {
			// The daemon is still starting.
		}
		await new Promise((resolve) => setTimeout(resolve, 50));
	}
	throw new Error("agnetd HTTP server did not become healthy");
}

describe("Integration: live HTTP resource routes", () => {
	let daemon: ChildProcess;
	let home: string;
	let client: NeunodeClient;
	let baseUrl: string;

	beforeAll(async () => {
		if (!BINARY_PATH)
			throw new Error("agnetd binary is required for HTTP integration tests");
		home = await mkdtemp(join(tmpdir(), "neunode-http-integration-"));
		const env = { ...process.env, HOME: home };
		await execFileAsync(
			BINARY_PATH,
			[
				"--output",
				"json-compact",
				"identity",
				"create",
				"--name",
				"http-integration",
			],
			{ env },
		);
		await execFileAsync(BINARY_PATH, ["token", "seed"], { env });
		await execFileAsync(
			BINARY_PATH,
			["config", "set", "tokens.unbonding_period_secs", "0"],
			{ env },
		);
		await execFileAsync(BINARY_PATH, ["token", "unstake", "--amount", "100"], {
			env,
		});
		await execFileAsync(BINARY_PATH, ["token", "claim-unbonded"], { env });
		await execFileAsync(
			BINARY_PATH,
			["config", "set", "tokens.unbonding_period_secs", "3600"],
			{ env },
		);

		const port = await availablePort();
		baseUrl = `http://127.0.0.1:${port}`;
		daemon = execFile(BINARY_PATH, ["serve", "--port", String(port)], { env });
		await waitForHealth(baseUrl);
		client = createNeunodeClient({ http: { baseUrl, timeout: 5_000 } });
	});

	afterAll(async () => {
		daemon?.kill("SIGTERM");
		if (home) await rm(home, { recursive: true, force: true });
	});

	it("serves discovery operations used by the SDK", async () => {
		const weights = await client.discovery.weights();
		expect(weights.data).toHaveLength(5);
		const results = await client.discovery.search({
			capabilities: "inference:llm",
		});
		expect(results.data).toEqual([]);
	});

	it("serves TurboQuant operations used by the SDK", async () => {
		const strategy = await client.turboquant.compress({
			profile: "kv_cache",
			dimension: 4096,
			targetBits: 3.5,
		});
		expect(strategy).toEqual({ strategy: "mse", bits: 3.5 });
	});

	it("serves the knowledge join-job operation", async () => {
		const identity = await client.identity.show();
		const result = await client.knowledge.joinJob({
			did: identity.did,
			jobId: "job:http-integration",
		});
		expect(result.agent).toBe(identity.did);
		expect(result.job).toBe("job:http-integration");
	});

	it("uses the daemon's canonical train jobs route", async () => {
		const jobs = await client.train.list();
		expect(jobs).toBeDefined();
	});

	it("settles a bounty with reward and provider bond through the shared service", async () => {
		const created = await client.bounty.create({
			title: "HTTP atomic settlement",
			description: "Exercises the live economic lifecycle",
			reward: 80,
			token: "compute",
		});
		const claimed = await client.bounty.claim({ id: created.id, stake: 12 });
		expect(claimed).toMatchObject({
			bounty_id: created.id,
			bond: 12,
			state: "Claimed",
		});
		const submitted = await client.bounty.submit({
			id: created.id,
			artifact: "ipfs://QmHttpAtomicSettlement",
		});
		expect(submitted.artifact_cid).toBe("ipfs://QmHttpAtomicSettlement");
		const reviewed = await client.bounty.review({
			id: created.id,
			score: 85,
			feedback: "accepted by integration test",
		});
		expect(reviewed).toMatchObject({ score: 85, state: "Accepted" });

		const paid = await client.bounty.pay(created.id);

		expect(paid).toMatchObject({
			bounty_id: created.id,
			amount_paid: 80,
			bond_returned: 12,
			state: "Paid",
		});
	});

	it("keeps newly unbonded tokens locked before maturity", async () => {
		await client.token.stake({ amount: 100, token: "compute" });
		const position = await client.token.unstake(100);
		expect(position.id).toMatch(/^unbond_/);
		expect(position.state).toBe("Unbonding");

		const earlyClaim = await client.token.claimUnbonded();
		const status = await client.token.stakeStatus();
		const balance = await client.token.balance("compute");

		expect(earlyClaim).toEqual({
			claimed_amount: 0,
			claimed_positions: 0,
			state: "NothingMatured",
		});
		expect(status.unbonding).toHaveLength(1);
		expect(status.unbonding[0]).toMatchObject({
			id: position.id,
			amount: 100,
			token: "nCompute",
		});
		expect(balance).toMatchObject({ balance: "0", staked: "0" });
	});
});
