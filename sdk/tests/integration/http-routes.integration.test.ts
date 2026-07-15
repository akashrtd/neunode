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
});
