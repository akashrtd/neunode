import { describe, it, expect, vi, beforeEach } from "vitest";
import { createConfigResource } from "./config.js";
import type { NeunodeClient } from "../client/client.js";
import { CliTransport } from "../transport/cli-transport.js";

function makeMockClient(): NeunodeClient {
  const execute = vi.fn();
  const transport = { execute, executeMulti: vi.fn(), executeRaw: vi.fn() } as unknown as CliTransport;
  return {
    cli: transport, viem: undefined, transportMode: "cli",
    identity: {} as never, config: {} as never, feed: {} as never, mesh: {} as never,
    model: {} as never, train: {} as never, bounty: {} as never, token: {} as never,
    reputation: {} as never, inference: {} as never, extend: vi.fn(),
  };
}

describe("createConfigResource", () => {
  let mockClient: NeunodeClient;
  let execute: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockClient = makeMockClient();
    execute = mockClient.cli!.execute as ReturnType<typeof vi.fn>;
  });

  it("should throw if cli transport is missing", () => {
    const noCliClient = { ...mockClient, cli: undefined };
    expect(() => createConfigResource(noCliClient)).toThrow("CLI transport required");
  });

  describe("set", () => {
    it("should call execute with config set key value", async () => {
      execute.mockResolvedValue(undefined);
      const resource = createConfigResource(mockClient);
      await resource.set({ key: "network", value: "testnet" });
      expect(execute).toHaveBeenCalledWith(["config", "set", "network", "testnet"]);
    });
  });

  describe("get", () => {
    it("should call execute with config get key", async () => {
      execute.mockResolvedValue({ network: "testnet" });
      const resource = createConfigResource(mockClient);
      const result = await resource.get("network");
      expect(result).toBe("testnet");
      expect(execute).toHaveBeenCalledWith(["config", "get", "network"]);
    });

    it("should return empty string for missing key", async () => {
      execute.mockResolvedValue({});
      const resource = createConfigResource(mockClient);
      const result = await resource.get("nonexistent");
      expect(result).toBe("");
    });
  });

  describe("list", () => {
    it("should call execute with config list", async () => {
      const expected = { network: "testnet", identity: "did:neunode:abc" };
      execute.mockResolvedValue(expected);
      const resource = createConfigResource(mockClient);
      const result = await resource.list();
      expect(result).toEqual(expected);
      expect(execute).toHaveBeenCalledWith(["config", "list"]);
    });
  });

  describe("path", () => {
    it("should call execute with config path and extract 'Config path'", async () => {
      execute.mockResolvedValue({ "Config path": "/home/user/.agnetd/config.toml" });
      const resource = createConfigResource(mockClient);
      const result = await resource.path();
      expect(result).toBe("/home/user/.agnetd/config.toml");
      expect(execute).toHaveBeenCalledWith(["config", "path"]);
    });

    it("should return empty string if Config path not in response", async () => {
      execute.mockResolvedValue({});
      const resource = createConfigResource(mockClient);
      const result = await resource.path();
      expect(result).toBe("");
    });
  });
});
