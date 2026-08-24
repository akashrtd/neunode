import type { ChildProcess } from "node:child_process";
import { execFile } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

import {
	type CID,
	createNeunodeClient,
	type NeunodeClient,
} from "../../src/index.js";
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

	it("serves lossless canonical token balances", async () => {
		const all = await client.token.balance();
		expect(all).toEqual({
			balances: [
				{ token: "nCompute", balance: "100", staked: "0" },
				{ token: "nTrain", balance: "0", staked: "50" },
				{ token: "nBandwidth", balance: "0", staked: "50" },
				{ token: "nStorage", balance: "0", staked: "50" },
			],
		});
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

	it("shows and exports the requested identity over HTTP", async () => {
		const active = await client.identity.show();
		const selected = await client.identity.show(active.did);
		expect(selected).toMatchObject({
			did: active.did,
			method: "persisted",
		});
		expect(selected.document).toMatchObject({ id: active.did });

		const exported = await client.identity.export({ did: active.did });
		expect(exported).toMatchObject({
			did: active.did,
			verification_methods: selected.verification_methods,
		});
		expect(exported.did_document).toEqual(selected.document);

		await expect(client.identity.show("did:neunode:missing")).rejects.toThrow(
			"identity 'did:neunode:missing' not found",
		);
	});

	it("round-trips canonical feed event IDs over HTTP", async () => {
		const posted = await client.feed.post({ kind: 4242, content: "http feed event" });
		expect(posted).toMatchObject({ sequence: 1, kind: 4242 });
		const shown = await client.feed.show(posted.event_id);
		expect(shown).toMatchObject({
			sequence: posted.sequence,
			kind: 4242,
			content: "http feed event",
		});
	});

	it("uses the daemon's canonical train jobs route", async () => {
		const jobs = await client.train.list();
		expect(jobs).toBeDefined();
	});

	it("uses canonical HTTP field names across training and inference", async () => {
		const job = await client.train.start({ model: "tiny", dataset: "fixture" });
		expect(job.job_id).toMatch(/^train_/);
		expect((await client.train.status(job.job_id)).job_id).toBe(job.job_id);

		const worker = await client.train.registerWorker({
			gpuCount: 2,
			gpuMemoryGb: 24,
			maxModelParams: 7_000_000_000,
			supportsBf16: true,
		});
		expect(worker).toMatchObject({ gpu_count: 2, supports_bf16: true });
		expect(
			await client.train.listWorkers({ minGpu: 2, minMemory: 20 }),
		).toEqual([expect.objectContaining({ worker_id: worker.worker_id })]);
		expect(
			await client.train.coordinatorStatus({ jobId: job.job_id }),
		).toMatchObject({
			job_id: job.job_id,
			coordinator: { active_workers: 1 },
		});
		expect((await client.train.stop(job.job_id)).status).toBe("stopped");

		const pricing = await client.inference.pricing("tiny", 1_000, 500);
		expect(pricing).toMatchObject({
			model: "tiny",
			input_tokens: 1_000,
			output_tokens: 500,
		});
	});

	it("registers and discovers inference providers over HTTP", async () => {
		await expect(
			client.inference.registerProvider({
				name: "Invalid provider",
				endpoint: "file:///etc/passwd",
				models: ["missing"],
			}),
		).rejects.toThrow("invalid provider endpoint");
		await client.model.push({
			path: "/tmp/provider.gguf",
			name: "provider-fixture",
		});
		await expect(
			client.inference.registerProvider({
				name: "Unknown model provider",
				endpoint: "https://provider.example/v1",
				models: ["missing"],
			}),
		).rejects.toThrow("model not found");
		const registered = await client.inference.registerProvider({
			name: "HTTP provider",
			endpoint: "https://provider.example/v1",
			models: ["provider-fixture"],
		});
		expect(registered).toMatchObject({
			name: "HTTP provider",
			models: ["provider-fixture"],
			status: "online",
		});
		const providers = await client.inference.providers("provider-fixture");
		expect(providers.providers).toEqual([
			expect.objectContaining({ name: "HTTP provider", model_count: 1 }),
		]);
	});

	it("streams inference results over WebSocket", async () => {
		const result = await new Promise<unknown>((resolve, reject) => {
			const timeout = setTimeout(() => reject(new Error("inference stream timed out")), 5_000);
			const cancel = client.inference.stream(
				{ model: "tiny", prompt: "hello", maxTokens: 16 },
				(value) => {
					clearTimeout(timeout);
					cancel();
					resolve(value);
				},
			);
		});
		expect(result).toMatchObject({
			model: "tiny",
			prompt: "hello",
			max_tokens: 16,
			status: "submitted",
		});
	});

	it("exposes fail-closed TEE verification over HTTP", async () => {
		await expect(
			client.verification.verifyIntelTdx({
				quoteHex: "00",
				collateralJson: "{}",
				mrTd: "11".repeat(48),
				reportData: "22".repeat(64),
				nowSecs: 1_751_000_000,
			}),
		).rejects.toThrow("Intel");

		await expect(
			client.verification.verifyAmdSnp({
				reportHex: "00",
				arkHex: "00",
				askHex: "00",
				vekHex: "00",
				generation: "milan",
				measurement: "11".repeat(48),
				reportData: "22".repeat(64),
				minimumTcb: { bootloader: 1, tee: 1, snp: 1, microcode: 1 },
			}),
		).rejects.toThrow("SEV-SNP");
	});

	it("publishes the lifecycle and lineage SDK contract in live OpenAPI", async () => {
		const response = await fetch(`${baseUrl}/api-docs/openapi.json`);
		expect(response.ok).toBe(true);
		const document = (await response.json()) as {
			paths?: Record<string, Record<string, unknown>>;
			components?: { schemas?: Record<string, unknown> };
		};
		expect(
			document.paths?.["/api/v1/verification/tee/intel-tdx"]?.post,
		).toBeDefined();
		expect(
			document.paths?.["/api/v1/verification/tee/amd-snp"]?.post,
		).toBeDefined();
		expect(
			document.paths?.["/api/v1/verification/tee/amd-vlek"]?.post,
		).toBeDefined();
		const operations = [
			["/api/v1/lifecycle/status", "get"],
			["/api/v1/lifecycle/activate", "post"],
			["/api/v1/lifecycle/hibernate", "post"],
			["/api/v1/lifecycle/reactivate", "post"],
			["/api/v1/lifecycle/list", "get"],
			["/api/v1/lifecycle/reap", "post"],
			["/api/v1/lineage/register", "post"],
			["/api/v1/lineage/{cid}", "get"],
			["/api/v1/lineage/{cid}/parents", "get"],
			["/api/v1/lineage/{cid}/children", "get"],
			["/api/v1/lineage/{cid}/ancestors", "get"],
			["/api/v1/lineage/{cid}/depth", "get"],
			["/api/v1/lineage/{cid}/royalties", "post"],
			["/api/v1/lineage/hash", "post"],
			["/api/v1/lineage/verify", "post"],
		] as const;
		for (const [path, method] of operations) {
			const operation = document.paths?.[path]?.[method];
			expect(operation, `${method.toUpperCase()} ${path}`).toBeDefined();
			expect(JSON.stringify(operation)).toContain("SuccessEnvelope");
		}
		for (const schema of [
			"LifecycleStatusBody",
			"ReapResult",
			"RegisterLineageRequest",
			"LineageDetailResponse",
			"RoyaltyAllocation",
			"VerifyResponse",
		]) {
			expect(document.components?.schemas?.[schema], schema).toBeDefined();
		}
	});

	it("executes lifecycle transitions through the typed SDK", async () => {
		expect(await client.lifecycle.status()).toHaveProperty("message");
		expect((await client.lifecycle.activate()).message).toContain("activated");
		const status = await client.lifecycle.status();
		expect(status).toMatchObject({ state: "ACTIVE" });
		expect((await client.lifecycle.hibernate()).message).toContain(
			"hibernating",
		);
		expect((await client.lifecycle.reactivate()).message).toContain(
			"reactivated",
		);
		expect(await client.lifecycle.list()).toEqual([
			expect.objectContaining({ state: "ACTIVE" }),
		]);
		expect(await client.lifecycle.reap()).toEqual({
			transitions: [],
			count: 0,
		});
	});

	it("executes lineage DAG operations alongside bincode model records", async () => {
		await client.model.push({
			path: "/tmp/local.gguf",
			name: "local-test-model",
		});
		const root = `sha256:${"a".repeat(64)}` as CID;
		const child = `sha256:${"b".repeat(64)}` as CID;
		const registeredRoot = await client.lineage.register({
			cid: root,
			contributionType: "pre_training",
		});
		expect(registeredRoot.parent_cids).toEqual([]);
		await client.lineage.register({
			cid: child,
			parents: [root],
			contributionType: "fine_tune",
			loraRank: 8,
			loraAlpha: 16,
		});
		expect(await client.lineage.parents(child)).toEqual([
			expect.objectContaining({ cid: root }),
		]);
		expect(await client.lineage.children(root)).toEqual([
			expect.objectContaining({ cid: child }),
		]);
		expect(await client.lineage.ancestors(child)).toEqual([
			expect.objectContaining({ cid: root }),
		]);
		expect(await client.lineage.depth(child)).toEqual({
			cid: child,
			lineage_depth: 1,
		});
		expect(await client.lineage.royalties(child, 10_000)).toHaveLength(1);
		expect(await client.lineage.show(child)).toMatchObject({
			cid: child,
			parent_cids: [root],
			signature_length: 64,
		});
		expect(await client.lineage.verify(child)).toMatchObject({
			cid: child,
			signature_valid: true,
			verified: true,
		});
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
		await client.config.set({
			key: "tokens.unbonding_period_secs",
			value: "7200",
		});
		expect(await client.config.get("tokens.unbonding_period_secs")).toBe(
			"7200",
		);

		const beforeUnstake = Math.floor(Date.now() / 1000);
		await client.token.stake({ amount: 100, token: "compute" });
		const position = await client.token.unstake(100);
		expect(position.id).toMatch(/^unbond_/);
		expect(position.state).toBe("Unbonding");
		expect(position.unbond_at - beforeUnstake).toBeGreaterThanOrEqual(7200);
		expect(position.unbond_at - beforeUnstake).toBeLessThanOrEqual(7205);

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
