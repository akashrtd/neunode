import { describe, it, expect } from "vitest";
import { createNeunodeClient, type NeunodeClient } from "./client.js";
import type { PublicClient, WalletClient, Chain } from "viem";

const mockPublicClient = { readContract: () => Promise.resolve() } as unknown as PublicClient;
const mockWalletClient = { writeContract: () => Promise.resolve("0xhash") } as unknown as WalletClient;
const mockChain = { id: 31337, name: "Anvil" } as unknown as Chain;

describe("createNeunodeClient", () => {
  describe("with CLI config only", () => {
    it("should return a client with transportMode 'cli'", () => {
      const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });
      expect(client.transportMode).toBe("cli");
    });

    it("should have cli transport defined", () => {
      const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });
      expect(client.cli).toBeDefined();
    });

    it("should have viem transport undefined", () => {
      const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });
      expect(client.viem).toBeUndefined();
    });
  });

  describe("with Viem config only", () => {
    it("should throw because resources require CLI transport", () => {
      expect(() =>
        createNeunodeClient({
          viem: { publicClient: mockPublicClient, chain: mockChain },
        })
      ).toThrow("CLI transport required");
    });
  });

  describe("with both transports (dual mode)", () => {
    it("should return a client with transportMode 'dual'", () => {
      const client = createNeunodeClient({
        cli: { binaryPath: "agnetd" },
        viem: { publicClient: mockPublicClient, walletClient: mockWalletClient, chain: mockChain },
      });
      expect(client.transportMode).toBe("dual");
    });

    it("should have both transports defined", () => {
      const client = createNeunodeClient({
        cli: { binaryPath: "agnetd" },
        viem: { publicClient: mockPublicClient, chain: mockChain },
      });
      expect(client.cli).toBeDefined();
      expect(client.viem).toBeDefined();
    });
  });

  describe("with no config (empty)", () => {
    it("should throw because resources require CLI transport", () => {
      expect(() => createNeunodeClient()).toThrow("CLI transport required");
    });
  });

  describe("resource properties", () => {
    it("should have all 10 resource properties", () => {
      const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });
      const resourceKeys = [
        "identity", "config", "feed", "mesh", "model",
        "train", "bounty", "token", "reputation", "inference",
      ] as const;
      for (const key of resourceKeys) {
        expect(client[key]).toBeDefined();
        expect(typeof client[key]).toBe("object");
      }
    });
  });

  describe("extend", () => {
    it("should add custom properties via extend", () => {
      const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });
      const extended = client.extend(() => ({ customProp: "hello" }));
      expect((extended as NeunodeClient & { customProp: string }).customProp).toBe("hello");
    });

    it("should preserve existing resource properties after extend", () => {
      const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });
      const extended = client.extend(() => ({ customProp: 42 }));
      expect(extended.identity).toBeDefined();
      expect(extended.config).toBeDefined();
      expect(extended.feed).toBeDefined();
    });

    it("should return same object reference (mutates in place)", () => {
      const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });
      const extended = client.extend(() => ({ extra: true }));
      expect(extended).toBe(client);
    });
  });
});
