import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import { createDiscoveryResource } from "./discovery.js";

function makeMockClient(): NeunodeClient {
	const execute = vi.fn();
	const transport = {
		execute,
		executeMulti: vi.fn(),
		executeRaw: vi.fn(),
	} as unknown as CliTransport;
	return {
		cli: transport,
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

	it("should throw if cli transport is missing", () => {
		expect(() =>
			createDiscoveryResource({ ...mockClient, cli: undefined }),
		).toThrow("CLI transport required");
	});

	describe("search", () => {
		it("should call execute with discover search --capabilities", async () => {
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

		it("should pass all filter params when provided", async () => {
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

		it("should not pass --min-reputation when zero", async () => {
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
		it("should call execute with discover complement --capabilities", async () => {
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

		it("should pass --limit when provided", async () => {
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
		it("should call execute with discover gaps", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(mockClient);
			await resource.gaps();
			expect(execute).toHaveBeenCalledWith(["discover", "gaps"]);
		});
	});

	describe("score", () => {
		it("should call execute with discover score args", async () => {
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
		it("should call execute with discover weights", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createDiscoveryResource(mockClient);
			await resource.weights();
			expect(execute).toHaveBeenCalledWith(["discover", "weights"]);
		});
	});
});
