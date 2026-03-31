import { describe, it, expect, vi, beforeEach } from "vitest";
import { createMeshResource } from "./mesh.js";
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

describe("createMeshResource", () => {
  let mockClient: NeunodeClient;
  let execute: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockClient = makeMockClient();
    execute = mockClient.cli!.execute as ReturnType<typeof vi.fn>;
  });

  it("should throw if cli transport is missing", () => {
    expect(() => createMeshResource({ ...mockClient, cli: undefined })).toThrow("CLI transport required");
  });

  describe("status", () => {
    it("should call execute with mesh status", async () => {
      const expected = {
        Status: "Connected", "Peer ID": "12D3KooW...", Listeners: "2",
        "Connected Peers": "5", "Subscribed Topics": "3",
      };
      execute.mockResolvedValue(expected);
      const resource = createMeshResource(mockClient);
      const result = await resource.status();
      expect(result).toEqual(expected);
      expect(execute).toHaveBeenCalledWith(["mesh", "status"]);
    });
  });

  describe("peers", () => {
    it("should call execute with mesh peers (no verbose)", async () => {
      execute.mockResolvedValue({ data: [] });
      const resource = createMeshResource(mockClient);
      await resource.peers();
      expect(execute).toHaveBeenCalledWith(["mesh", "peers"]);
    });

    it("should pass --verbose when verbose=true", async () => {
      execute.mockResolvedValue({ data: [] });
      const resource = createMeshResource(mockClient);
      await resource.peers(true);
      expect(execute).toHaveBeenCalledWith(["mesh", "peers", "--verbose"]);
    });

    it("should not pass --verbose when verbose=false", async () => {
      execute.mockResolvedValue({ data: [] });
      const resource = createMeshResource(mockClient);
      await resource.peers(false);
      expect(execute).toHaveBeenCalledWith(["mesh", "peers"]);
    });
  });

  describe("connect", () => {
    it("should call execute with mesh connect --addr", async () => {
      execute.mockResolvedValue({ action: "connect", address: "/ip4/1.2.3.4/tcp/4001", status: "ok" });
      const resource = createMeshResource(mockClient);
      await resource.connect("/ip4/1.2.3.4/tcp/4001/p2p/QmX7b");
      expect(execute).toHaveBeenCalledWith(["mesh", "connect", "--addr", "/ip4/1.2.3.4/tcp/4001/p2p/QmX7b"]);
    });
  });

  describe("disconnect", () => {
    it("should call execute with mesh disconnect --peer-id", async () => {
      execute.mockResolvedValue({ action: "disconnect", peer_id: "12D3KooW", status: "ok" });
      const resource = createMeshResource(mockClient);
      await resource.disconnect("12D3KooW");
      expect(execute).toHaveBeenCalledWith(["mesh", "disconnect", "--peer-id", "12D3KooW"]);
    });
  });
});
