import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createReputationResource } from "./reputation.js";

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
		lifecycle: {} as never,
		lineage: {} as never,
		extend: vi.fn(),
	};
}

describe("createReputationResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both transports are missing", async () => {
		const resource = createReputationResource({
			...mockClient,
			cli: undefined,
			http: undefined,
		});
		await expect(resource.leaderboard()).rejects.toThrow(
			"HTTP or CLI transport required",
		);
	});

	describe("show", () => {
		it("should use HTTP transport when available", async () => {
			const expected = { agent: "did:neunode:http", score: 90 };
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue(expected);
			const resource = createReputationResource(dualClient);
			const result = await resource.show("did:neunode:http");
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/reputation?agent=did%3Aneunode%3Ahttp",
			);
			expect(result).toEqual(expected);
		});

		it("should call execute with reputation show (no agent) via CLI", async () => {
			execute.mockResolvedValue({
				agent: "did:neunode:abc",
				score: 85,
				grade: "A",
			});
			const resource = createReputationResource(mockClient);
			await resource.show();
			expect(execute).toHaveBeenCalledWith(["reputation", "show"]);
		});

		it("should pass --agent when provided via CLI", async () => {
			execute.mockResolvedValue({ agent: "did:neunode:abc", score: 90 });
			const resource = createReputationResource(mockClient);
			await resource.show("did:neunode:abc");
			expect(execute).toHaveBeenCalledWith([
				"reputation",
				"show",
				"--agent",
				"did:neunode:abc",
			]);
		});
	});

	describe("attest", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({
				attester: "me",
				target: "them",
				score: 8,
				signed: true,
			});
			const resource = createReputationResource(dualClient);
			await resource.attest({ to: "did:neunode:them", score: 8 });
			expect(http.post).toHaveBeenCalledWith("/api/v1/reputation/attest", {
				to: "did:neunode:them",
				score: 8,
			});
		});

		it("should call execute with reputation attest --to --score via CLI", async () => {
			execute.mockResolvedValue({
				attester: "did:neunode:me",
				target: "did:neunode:them",
				score: 8,
				signed: true,
			});
			const resource = createReputationResource(mockClient);
			await resource.attest({ to: "did:neunode:them", score: 8 });
			expect(execute).toHaveBeenCalledWith([
				"reputation",
				"attest",
				"--to",
				"did:neunode:them",
				"--score",
				"8",
			]);
		});

		it("should pass --comment when provided via CLI", async () => {
			execute.mockResolvedValue({
				attester: "me",
				target: "them",
				score: 9,
				comment: "great",
			});
			const resource = createReputationResource(mockClient);
			await resource.attest({
				to: "did:neunode:them",
				score: 9,
				comment: "Excellent work",
			});
			expect(execute).toHaveBeenCalledWith([
				"reputation",
				"attest",
				"--to",
				"did:neunode:them",
				"--score",
				"9",
				"--comment",
				"Excellent work",
			]);
		});
	});

	describe("leaderboard", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createReputationResource(dualClient);
			await resource.leaderboard(10);
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/reputation/leaderboard?limit=10",
			);
		});

		it("should call execute with reputation leaderboard (no limit) via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createReputationResource(mockClient);
			await resource.leaderboard();
			expect(execute).toHaveBeenCalledWith(["reputation", "leaderboard"]);
		});

		it("should pass --limit when provided via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createReputationResource(mockClient);
			await resource.leaderboard(10);
			expect(execute).toHaveBeenCalledWith([
				"reputation",
				"leaderboard",
				"--limit",
				"10",
			]);
		});
	});

	describe("factors", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ agent: "me", total_score: "85", data: [] });
			const resource = createReputationResource(dualClient);
			await resource.factors("did:neunode:abc");
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/reputation/factors?agent=did%3Aneunode%3Aabc",
			);
		});

		it("should call execute with reputation factors (no agent) via CLI", async () => {
			execute.mockResolvedValue({ agent: "me", total_score: "85", data: [] });
			const resource = createReputationResource(mockClient);
			await resource.factors();
			expect(execute).toHaveBeenCalledWith(["reputation", "factors"]);
		});

		it("should pass --agent when provided via CLI", async () => {
			execute.mockResolvedValue({ agent: "did:neunode:abc", data: [] });
			const resource = createReputationResource(mockClient);
			await resource.factors("did:neunode:abc");
			expect(execute).toHaveBeenCalledWith([
				"reputation",
				"factors",
				"--agent",
				"did:neunode:abc",
			]);
		});
	});
});
