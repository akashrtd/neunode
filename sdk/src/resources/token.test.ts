import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import { createTokenResource } from "./token.js";

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

describe("createTokenResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if cli transport is missing", () => {
		expect(() =>
			createTokenResource({ ...mockClient, cli: undefined }),
		).toThrow("CLI transport required");
	});

	describe("balance", () => {
		it("should call execute with token balance (no token)", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createTokenResource(mockClient);
			await resource.balance();
			expect(execute).toHaveBeenCalledWith(["token", "balance"]);
		});

		it("should pass --token when provided", async () => {
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
		it("should call execute with token transfer --to --amount --token", async () => {
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
		it("should call execute with token stake --amount --token", async () => {
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
		it("should call execute with token unstake --amount", async () => {
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
		it("should call execute with token stake-status", async () => {
			execute.mockResolvedValue({ total_staked: 1000, entries: [] });
			const resource = createTokenResource(mockClient);
			await resource.stakeStatus();
			expect(execute).toHaveBeenCalledWith(["token", "stake-status"]);
		});
	});

	describe("decayInfo", () => {
		it("should call execute with token decay-info", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createTokenResource(mockClient);
			await resource.decayInfo();
			expect(execute).toHaveBeenCalledWith(["token", "decay-info"]);
		});
	});
});
