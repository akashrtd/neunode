import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createDiscoveryResource } from "./discovery.js";

function makeMockClient(
	opts: { withHttp?: boolean; withCli?: boolean } = {},
): NeunodeClient {
	const execute = vi.fn();
	const transport = {
		execute,
		executeMulti: vi.fn(),
		executeRaw: vi.fn(),
	} as unknown as CliTransport;

	const httpGet = vi.fn();
	const httpPost = vi.fn();
	const httpTransport = {
		get: httpGet,
		post: httpPost,
		put: vi.fn(),
		delete: vi.fn(),
	} as unknown as HttpTransport;

	return {
		cli: opts.withHttp && !opts.withCli ? undefined : transport,
		http: opts.withHttp ? httpTransport : undefined,
		viem: undefined,
		transportMode: "cli",
		identity: {} as never,
		config: {} as never,
		feed: {} as never,
		mesh: {} as never,
		model: {} as never,
		train: {} as never,
		bounty: {} as never,
		token: {} as never,
		reputation: {} as never,
		inference: {} as never,
		knowledge: {} as never,
		discovery: {} as never,
		turboquant: {} as never,
		extend: vi.fn(),
	};
}

describe("createDiscoveryResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both transports are missing", async () => {
		const resource = createDiscoveryResource({ ...mockClient, cli: undefined, http: undefined });
		await expect(resource.gaps()).rejects.toThrow("HTTP or CLI transport required");
	});

	describe("search", () => {
		it("should use HTTP transport with query params", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(dualClient);
			await resource.search({
				capabilities: "inference:llm,training:lora",
				minReputation: 3.0,
				maxCost: 20.0,
				onlineOnly: true,
				limit: 5,
			});
			expect(http.get).toHaveBeenCalled();
			const callUrl = http.get.mock.calls[0]?.[0] as string;
			expect(callUrl).toContain("/api/v1/discovery/search?");
			expect(callUrl).toContain("capabilities=inference%3Allm%2Ctraining%3Alora");
			expect(callUrl).toContain("minReputation=3");
			expect(callUrl).toContain("maxCost=20");
			expect(callUrl).toContain("onlineOnly=true");
			expect(callUrl).toContain("limit=5");
		});

		it("should call execute with discover search --capabilities via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(mockClient);
			await resource.search({ capabilities: "inference:llm" });
			expect(execute).toHaveBeenCalledWith([
				"discover",
				"search",
				"--capabilities",
				"inference:llm",
			]);
		});

		it("should pass all filter params when provided via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(mockClient);
			await resource.search({
				capabilities: "inference:llm,training:lora",
				minReputation: 3.0,
				maxCost: 20.0,
				onlineOnly: true,
				limit: 5,
			});
			expect(execute).toHaveBeenCalledWith([
				"discover",
				"search",
				"--capabilities",
				"inference:llm,training:lora",
				"--min-reputation",
				"3",
				"--max-cost",
				"20",
				"--online-only",
				"--limit",
				"5",
			]);
		});

		it("should not pass --min-reputation when zero via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(mockClient);
			await resource.search({ capabilities: "a", minReputation: 0 });
			expect(execute).toHaveBeenCalledWith([
				"discover",
				"search",
				"--capabilities",
				"a",
			]);
		});
	});

	describe("complement", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(dualClient);
			await resource.complement({ capabilities: "inference:llm", limit: 5 });
			expect(http.get).toHaveBeenCalled();
			const callUrl = http.get.mock.calls[0]?.[0] as string;
			expect(callUrl).toContain("/api/v1/discovery/complement?");
			expect(callUrl).toContain("capabilities=inference%3Allm");
			expect(callUrl).toContain("limit=5");
		});

		it("should call execute with discover complement --capabilities via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(mockClient);
			await resource.complement({ capabilities: "inference:llm" });
			expect(execute).toHaveBeenCalledWith([
				"discover",
				"complement",
				"--capabilities",
				"inference:llm",
			]);
		});

		it("should pass --limit when provided via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(mockClient);
			await resource.complement({ capabilities: "a", limit: 5 });
			expect(execute).toHaveBeenCalledWith([
				"discover",
				"complement",
				"--capabilities",
				"a",
				"--limit",
				"5",
			]);
		});
	});

	describe("gaps", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(dualClient);
			await resource.gaps();
			expect(http.get).toHaveBeenCalledWith("/api/v1/discovery/gaps");
		});

		it("should call execute with discover gaps via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(mockClient);
			await resource.gaps();
			expect(execute).toHaveBeenCalledWith(["discover", "gaps"]);
		});
	});

	describe("score", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ did: "did:neunode:abc", final_score: "0.7500" });
			const resource = createDiscoveryResource(dualClient);
			await resource.score({ agent: "did:neunode:abc", capabilities: "inference:llm" });
			expect(http.get).toHaveBeenCalled();
			const callUrl = http.get.mock.calls[0]?.[0] as string;
			expect(callUrl).toContain("/api/v1/discovery/score?");
			expect(callUrl).toContain("agent=did%3Aneunode%3Aabc");
			expect(callUrl).toContain("capabilities=inference%3Allm");
		});

		it("should call execute with discover score args via CLI", async () => {
			execute.mockResolvedValue({
				did: "did:neunode:abc",
				final_score: "0.7500",
				capability: "0.9000",
				quality: "0.8000",
				availability: "0.6000",
				cost_efficiency: "0.5000",
				complementarity: "0.4000",
			});
			const resource = createDiscoveryResource(mockClient);
			await resource.score({
				agent: "did:neunode:abc",
				capabilities: "inference:llm",
			});
			expect(execute).toHaveBeenCalledWith([
				"discover",
				"score",
				"--agent",
				"did:neunode:abc",
				"--capabilities",
				"inference:llm",
			]);
		});
	});

	describe("weights", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(dualClient);
			await resource.weights();
			expect(http.get).toHaveBeenCalledWith("/api/v1/discovery/weights");
		});

		it("should call execute with discover weights via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(mockClient);
			await resource.weights();
			expect(execute).toHaveBeenCalledWith(["discover", "weights"]);
		});
	});
});
