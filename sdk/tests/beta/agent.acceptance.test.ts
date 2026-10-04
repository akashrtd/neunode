import { type ChildProcess, execFile } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { BINARY_PATH } from "../integration/helpers/agnetd.js";

const execute = promisify(execFile);
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

interface Fixture {
	directory: string;
	env: NodeJS.ProcessEnv;
	process: ChildProcess;
	url: string;
}

interface Envelope {
	success: boolean;
	data?: Record<string, unknown>;
	error?: { code: string; message: string };
}

const fixtures: Fixture[] = [];
let provider: Server | undefined;
let providerCalls = 0;

async function availablePort(): Promise<number> {
	const server = createServer();
	server.listen(0, "127.0.0.1");
	await once(server, "listening");
	const address = server.address();
	if (!address || typeof address === "string") throw new Error("No test port");
	await new Promise<void>((resolve, reject) =>
		server.close((error) => (error ? reject(error) : resolve())),
	);
	return address.port;
}

async function startFixture(bootstrap: boolean): Promise<Fixture> {
	if (!BINARY_PATH)
		throw new Error("Build agnetd before running beta acceptance tests");
	const directory = await mkdtemp(join(tmpdir(), "neunode-beta-"));
	// A child-only home isolates the daemon's hardcoded identity/config paths.
	// The parent environment and the user's actual identities are untouched.
	const env = { ...process.env, HOME: directory };
	await execute(
		BINARY_PATH,
		["config", "set", "network.listen_addr", "/ip4/127.0.0.1/tcp/0"],
		{ env },
	);
	if (bootstrap) {
		await execute(BINARY_PATH, ["identity", "create", "--name", "beta-agent"], {
			env,
		});
		await execute(BINARY_PATH, ["token", "seed"], { env });
	}
	const port = await availablePort();
	const daemon = execFile(BINARY_PATH, ["serve", "--port", String(port)], {
		env,
	});
	const fixture = {
		directory,
		env,
		process: daemon,
		url: `http://127.0.0.1:${port}`,
	};
	fixtures.push(fixture);
	const deadline = Date.now() + 15_000;
	while (Date.now() < deadline) {
		if (daemon.exitCode !== null)
			throw new Error("Test daemon exited during startup");
		try {
			if ((await fetch(`${fixture.url}/api/v1/health`)).ok) return fixture;
		} catch {
			// Wait for the real listener, rather than treating startup as a test failure.
		}
		await delay(50);
	}
	throw new Error("Test daemon did not become healthy");
}

async function request(fixture: Fixture, path: string, body?: unknown) {
	const response = await fetch(`${fixture.url}${path}`, {
		method: body === undefined ? "GET" : "POST",
		headers: { "content-type": "application/json" },
		...(body === undefined ? {} : { body: JSON.stringify(body) }),
		signal: AbortSignal.timeout(5_000),
	});
	return { status: response.status, body: (await response.json()) as Envelope };
}

afterAll(async () => {
	if (provider) {
		await new Promise<void>((resolve, reject) =>
			provider?.close((error) => (error ? reject(error) : resolve())),
		);
	}
	for (const fixture of fixtures) {
		if (
			fixture.process.exitCode === null &&
			fixture.process.signalCode === null
		) {
			const exited = once(fixture.process, "exit");
			fixture.process.kill("SIGINT");
			await Promise.race([exited, delay(3_000)]);
			if (
				fixture.process.exitCode === null &&
				fixture.process.signalCode === null
			) {
				fixture.process.kill("SIGKILL");
				await exited;
			}
		}
		await rm(fixture.directory, { recursive: true, force: true });
	}
});

describe("Beta: public agent promises", () => {
	let fresh: Fixture;
	let agent: Fixture;
	let other: Fixture;

	beforeAll(async () => {
		fresh = await startFixture(false);
		agent = await startFixture(true);
		other = await startFixture(true);
	});

	it("boots a real daemon and serves its public schema", async () => {
		expect((await request(agent, "/api/v1/health")).status).toBe(200);
		const schema = await fetch(`${agent.url}/api-docs/openapi.json`);
		expect(schema.status).toBe(200);
	});

	it("creates the first identity through the agent's HTTP interface", async () => {
		const result = await request(fresh, "/api/v1/identity/create", {
			name: "first",
		});
		expect(result.status, JSON.stringify(result.body)).toBe(201);
	});

	it("creates distinct identities for distinct agents", async () => {
		const first = await request(agent, "/api/v1/identity/create", {
			name: "one",
		});
		const second = await request(agent, "/api/v1/identity/create", {
			name: "two",
		});
		expect(first.status).toBe(201);
		expect(second.status).toBe(201);
		expect(second.body.data?.identity).not.toEqual(first.body.data?.identity);
		const identityOne = first.body.data?.identity as Record<string, unknown>;
		const identityTwo = second.body.data?.identity as Record<string, unknown>;
		expect(identityTwo.did).not.toBe(identityOne.did);
	});

	it("returns a signed public feed event", async () => {
		const posted = await request(agent, "/api/v1/feed", {
			kind: 0,
			content: "signed evidence",
		});
		expect(posted.status).toBe(201);
		const eventId = posted.body.data?.event_id;
		const shown = await request(agent, `/api/v1/feed/${String(eventId)}`);
		expect(shown.body.data?.signature).toBeTruthy();
	});

	it("rejects feed kinds that overflow the protocol wire type", async () => {
		const result = await request(agent, "/api/v1/feed", {
			kind: 65_536,
			content: "overflow",
		});
		expect(result.status).toBe(400);
	});

	it("assigns globally distinct feed IDs to distinct authors", async () => {
		const a = await request(agent, "/api/v1/feed", {
			kind: 0,
			content: "author A",
		});
		const b = await request(other, "/api/v1/feed", {
			kind: 0,
			content: "author B",
		});
		// Use the same sequence on each daemon: IDs must bind the full author DID.
		const sequence = a.body.data?.sequence as number;
		let matching = b;
		for (
			let current = b.body.data?.sequence as number;
			current < sequence;
			current++
		) {
			matching = await request(other, "/api/v1/feed", {
				kind: 0,
				content: "align sequence",
			});
		}
		expect(matching.body.data?.sequence).toBe(sequence);
		expect(matching.body.data?.event_id).not.toBe(a.body.data?.event_id);
	});

	it("rejects inference when the requested model has no provider", async () => {
		const result = await request(agent, "/api/v1/inference/request", {
			model: "no-such-provider",
			prompt: "Return a real answer",
		});
		expect([404, 503]).toContain(result.status);
	});

	it("dispatches inference to the registered provider", async () => {
		provider = createServer(async (incoming, response) => {
			for await (const _chunk of incoming) {
				/* Consume the request body. */
			}
			providerCalls++;
			response.writeHead(200, { "content-type": "application/json" });
			response.end(
				JSON.stringify({
					id: "beta-completion",
					object: "chat.completion",
					created: 1,
					model: "beta-model",
					choices: [
						{
							index: 0,
							message: { role: "assistant", content: "real provider answer" },
							finish_reason: "stop",
						},
					],
					usage: { prompt_tokens: 3, completion_tokens: 3, total_tokens: 6 },
				}),
			);
		});
		provider.listen(0, "127.0.0.1");
		await once(provider, "listening");
		const address = provider.address();
		if (!address || typeof address === "string")
			throw new Error("No provider port");
		expect(
			(
				await request(agent, "/api/v1/models", {
					name: "beta-model",
					path: "fixture",
				})
			).status,
		).toBe(201);
		expect(
			(
				await request(agent, "/api/v1/inference/providers", {
					name: "beta-provider",
					endpoint: `http://127.0.0.1:${address.port}`,
					models: ["beta-model"],
				})
			).status,
		).toBe(201);
		const result = await request(agent, "/api/v1/inference/request", {
			model: "beta-model",
			prompt: "Return a real answer",
			max_tokens: 16,
		});
		expect(result.status).toBe(200);
		await delay(100);
		expect(providerCalls, JSON.stringify(result.body)).toBeGreaterThan(0);
	});

	it("assigns distinct IDs to concurrently submitted training jobs", async () => {
		const results = await Promise.all(
			Array.from({ length: 10 }, () =>
				request(agent, "/api/v1/train/start", {
					model: "tiny",
					dataset: "same-dataset",
				}),
			),
		);
		for (const result of results) expect(result.status).toBe(201);
		expect(
			new Set(results.map((result) => result.body.data?.job_id)).size,
		).toBe(10);
	});

	it("connects the serving daemon to its P2P runtime", async () => {
		const result = await request(agent, "/api/v1/mesh/status");
		expect(result.status).toBe(200);
		expect(result.body.data?.running).toBe(true);
	});

	it("uses actual staked balances when calculating reputation", async () => {
		const staking = await request(agent, "/api/v1/tokens/stake-status");
		expect(Number(staking.body.data?.total_staked)).toBeGreaterThan(0);
		const reputation = await request(agent, "/api/v1/reputation");
		const factors = reputation.body.data?.factors as Record<string, unknown>;
		expect(Number(factors.stake)).toBeGreaterThan(0);
	});

	it("rejects capability writes attributed to an identity the signer does not control", async () => {
		const result = await request(agent, "/api/v1/knowledge/register-agent", {
			did: "did:neunode:unowned-beta-agent",
			capabilities: "beta-capability",
		});
		expect([401, 403]).toContain(result.status);
	});

	it("does not invent stake and quality measurements for discovered agents", async () => {
		const registered = await request(
			agent,
			"/api/v1/knowledge/register-agent",
			{
				did: "did:neunode:beta-unmeasured",
				capabilities: "beta-unmeasured-capability",
			},
		);
		expect(registered.status).toBe(201);
		const capability = encodeURIComponent(
			"https://neunode.io/ontology/beta-unmeasured-capability",
		);
		const result = await request(
			agent,
			`/api/v1/discovery/search?capabilities=${capability}`,
		);
		expect(result.status, JSON.stringify(result.body)).toBe(200);
		const rows = result.body.data?.data as Array<Record<string, unknown>>;
		expect(rows).toHaveLength(1);
		const candidate = rows[0]?.candidate as Record<string, unknown>;
		expect(Number(candidate.stake_amount)).toBe(0);
		expect(Number(candidate.reputation_score)).toBe(0);
	});

	it("finds an agent using the capability name supplied during registration", async () => {
		expect(
			(
				await request(agent, "/api/v1/knowledge/register-agent", {
					did: "did:neunode:beta-searchable",
					capabilities: "beta-searchable-capability",
				})
			).status,
		).toBe(201);
		const result = await request(
			agent,
			"/api/v1/discovery/search?capabilities=beta-searchable-capability",
		);
		expect(result.status, JSON.stringify(result.body)).toBe(200);
		const rows = result.body.data?.data as Array<Record<string, unknown>>;
		expect(rows).toHaveLength(1);
	});

	it("rejects an unfunded bounty without creating a payable promise", async () => {
		const result = await request(other, "/api/v1/bounties", {
			title: "unfunded",
			description: "must fail",
			reward: 50,
			token: "compute",
		});
		expect(result.status).toBeGreaterThanOrEqual(400);
		expect(result.body.success).toBe(false);
	});

	it("requires authorization for a caller to mutate daemon state", async () => {
		const result = await request(agent, "/api/v1/train/start", {
			model: "unauthorized",
			dataset: "untrusted-caller",
		});
		expect([401, 403]).toContain(result.status);
	});

	it("preserves stored events when the daemon restarts", async () => {
		const posted = await request(agent, "/api/v1/feed", {
			kind: 0,
			content: "restart evidence",
		});
		const eventId = posted.body.data?.event_id;
		const stopped = once(agent.process, "exit");
		agent.process.kill("SIGINT");
		await stopped;
		if (!BINARY_PATH) throw new Error("Daemon binary disappeared");
		const port = new URL(agent.url).port;
		agent.process = execFile(BINARY_PATH, ["serve", "--port", port], {
			env: agent.env,
		});
		const deadline = Date.now() + 10_000;
		while (Date.now() < deadline) {
			try {
				if ((await request(agent, "/api/v1/health")).status === 200) break;
			} catch {
				/* Wait for the restarted listener. */
			}
			await delay(50);
		}
		const restored = await request(agent, `/api/v1/feed/${String(eventId)}`);
		expect(restored.status).toBe(200);
		expect(restored.body.data?.content).toBe("restart evidence");
	});
});
