import type { Chain, PublicClient, WalletClient } from "viem";
import { describe, expect, it } from "vitest";
import { ViemTransport } from "./viem-transport.js";

const mockPublicClient = {
	readContract: () => Promise.resolve(),
} as unknown as PublicClient;
const mockWalletClient = {
	writeContract: () => Promise.resolve("0xhash"),
} as unknown as WalletClient;
const mockChain = { id: 31337, name: "Anvil" } as unknown as Chain;

describe("ViemTransport", () => {
	describe("constructor", () => {
		it("should store publicClient, walletClient, and chain", () => {
			const transport = new ViemTransport({
				publicClient: mockPublicClient,
				walletClient: mockWalletClient,
				chain: mockChain,
			});
			expect(transport.publicClient).toBe(mockPublicClient);
			expect(transport.walletClient).toBe(mockWalletClient);
			expect(transport.chain).toBe(mockChain);
		});

		it("should allow undefined walletClient", () => {
			const transport = new ViemTransport({
				publicClient: mockPublicClient,
				chain: mockChain,
			});
			expect(transport.walletClient).toBeUndefined();
		});
	});

	describe("canWrite", () => {
		it("should return true when walletClient is provided", () => {
			const transport = new ViemTransport({
				publicClient: mockPublicClient,
				walletClient: mockWalletClient,
				chain: mockChain,
			});
			expect(transport.canWrite).toBe(true);
		});

		it("should return false when walletClient is undefined", () => {
			const transport = new ViemTransport({
				publicClient: mockPublicClient,
				chain: mockChain,
			});
			expect(transport.canWrite).toBe(false);
		});
	});

	describe("chainId", () => {
		it("should return the chain id", () => {
			const transport = new ViemTransport({
				publicClient: mockPublicClient,
				chain: mockChain,
			});
			expect(transport.chainId).toBe(31337);
		});

		it("should return different chain ids for different chains", () => {
			const mainnetChain = { id: 1, name: "Mainnet" } as unknown as Chain;
			const transport = new ViemTransport({
				publicClient: mockPublicClient,
				chain: mainnetChain,
			});
			expect(transport.chainId).toBe(1);
		});
	});
});
