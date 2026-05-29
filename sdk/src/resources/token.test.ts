import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createTokenResource } from "./token.js";

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

describe("createTokenResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both transports are missing", async () => {
		const resource = createTokenResource({
			...mockClient,
			cli: undefined,
			http: undefined,
		});
		await expect(resource.stakeStatus()).rejects.toThrow(
			"HTTP or CLI transport required",
		);
	});

	describe("balance", () => {
		it("should use HTTP transport when available", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({
				token: "nCompute",
				balance: "1000",
				staked: "100",
			});
			const resource = createTokenResource(dualClient);
			await resource.balance("nCompute");
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/tokens/balance?token=nCompute",
			);
		});

		it("should call execute with token balance (no token) via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createTokenResource(mockClient);
			await resource.balance();
			expect(execute).toHaveBeenCalledWith(["token", "balance"]);
		});

		it("should pass --token when provided via CLI", async () => {
			execute.mockResolvedValue({
				token: "nCompute",
				balance: "1000",
				staked: "100",
			});
			const resource = createTokenResource(mockClient);
			await resource.balance("nCompute");
			expect(execute).toHaveBeenCalledWith([
				"token",
				"balance",
				"--token",
				"nCompute",
			]);
		});
	});

	describe("transfer", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({ state: "sent" });
			const resource = createTokenResource(dualClient);
			await resource.transfer({
				to: "did:neunode:def",
				amount: 100,
				token: "nCompute",
			});
			expect(http.post).toHaveBeenCalledWith("/api/v1/tokens/transfer", {
				to: "did:neunode:def",
				amount: 100,
				token: "nCompute",
			});
		});

		it("should call execute with token transfer --to --amount --token via CLI", async () => {
			execute.mockResolvedValue({
				from: "did:neunode:abc",
				to: "did:neunode:def",
				amount: 100,
				token: "nCompute",
				state: "sent",
			});
			const resource = createTokenResource(mockClient);
			await resource.transfer({
				to: "did:neunode:def",
				amount: 100,
				token: "nCompute",
			});
			expect(execute).toHaveBeenCalledWith([
				"token",
				"transfer",
				"--to",
				"did:neunode:def",
				"--amount",
				"100",
				"--token",
				"nCompute",
			]);
		});
	});

	describe("stake", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({ state: "staked" });
			const resource = createTokenResource(dualClient);
			await resource.stake({ amount: 500, token: "nCompute" });
			expect(http.post).toHaveBeenCalledWith("/api/v1/tokens/stake", {
				amount: 500,
				token: "nCompute",
			});
		});

		it("should call execute with token stake --amount --token via CLI", async () => {
			execute.mockResolvedValue({
				amount: 500,
				token: "nCompute",
				state: "staked",
				unbonding_period_secs: 86400,
			});
			const resource = createTokenResource(mockClient);
			await resource.stake({ amount: 500, token: "nCompute" });
			expect(execute).toHaveBeenCalledWith([
				"token",
				"stake",
				"--amount",
				"500",
				"--token",
				"nCompute",
			]);
		});
	});

	describe("unstake", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({ state: "unbonding" });
			const resource = createTokenResource(dualClient);
			await resource.unstake(200);
			expect(http.post).toHaveBeenCalledWith("/api/v1/tokens/unstake", {
				amount: 200,
			});
		});

		it("should call execute with token unstake --amount via CLI", async () => {
			execute.mockResolvedValue({
				amount: 200,
				token: "nCompute",
				unbond_at: 1234567890,
				state: "unbonding",
			});
			const resource = createTokenResource(mockClient);
			await resource.unstake(200);
			expect(execute).toHaveBeenCalledWith([
				"token",
				"unstake",
				"--amount",
				"200",
			]);
		});
	});

	describe("stakeStatus", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ total_staked: 1000, entries: [] });
			const resource = createTokenResource(dualClient);
			await resource.stakeStatus();
			expect(http.get).toHaveBeenCalledWith("/api/v1/tokens/stake-status");
		});

		it("should call execute with token stake-status via CLI", async () => {
			execute.mockResolvedValue({ total_staked: 1000, entries: [] });
			const resource = createTokenResource(mockClient);
			await resource.stakeStatus();
			expect(execute).toHaveBeenCalledWith(["token", "stake-status"]);
		});
	});

	describe("decayInfo", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createTokenResource(dualClient);
			await resource.decayInfo();
			expect(http.get).toHaveBeenCalledWith("/api/v1/tokens/decay-info");
		});

		it("should call execute with token decay-info via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createTokenResource(mockClient);
			await resource.decayInfo();
			expect(execute).toHaveBeenCalledWith(["token", "decay-info"]);
		});
	});
});
