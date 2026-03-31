import { describe, it, expect, vi, beforeEach } from "vitest";
import { createReputationResource } from "./reputation.js";
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

describe("createReputationResource", () => {
  let mockClient: NeunodeClient;
  let execute: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockClient = makeMockClient();
    execute = mockClient.cli!.execute as ReturnType<typeof vi.fn>;
  });

  it("should throw if cli transport is missing", () => {
    expect(() => createReputationResource({ ...mockClient, cli: undefined })).toThrow("CLI transport required");
  });

  describe("show", () => {
    it("should call execute with reputation show (no agent)", async () => {
      execute.mockResolvedValue({ agent: "did:neunode:abc", score: 85, grade: "A" });
      const resource = createReputationResource(mockClient);
      await resource.show();
      expect(execute).toHaveBeenCalledWith(["reputation", "show"]);
    });

    it("should pass --agent when provided", async () => {
      execute.mockResolvedValue({ agent: "did:neunode:abc", score: 90 });
      const resource = createReputationResource(mockClient);
      await resource.show("did:neunode:abc");
      expect(execute).toHaveBeenCalledWith(["reputation", "show", "--agent", "did:neunode:abc"]);
    });
  });

  describe("attest", () => {
    it("should call execute with reputation attest --to --score", async () => {
      execute.mockResolvedValue({ attester: "did:neunode:me", target: "did:neunode:them", score: 8, signed: true });
      const resource = createReputationResource(mockClient);
      await resource.attest({ to: "did:neunode:them", score: 8 });
      expect(execute).toHaveBeenCalledWith([
        "reputation", "attest", "--to", "did:neunode:them", "--score", "8",
      ]);
    });

    it("should pass --comment when provided", async () => {
      execute.mockResolvedValue({ attester: "me", target: "them", score: 9, comment: "great" });
      const resource = createReputationResource(mockClient);
      await resource.attest({ to: "did:neunode:them", score: 9, comment: "Excellent work" });
      expect(execute).toHaveBeenCalledWith([
        "reputation", "attest", "--to", "did:neunode:them", "--score", "9", "--comment", "Excellent work",
      ]);
    });
  });

  describe("leaderboard", () => {
    it("should call execute with reputation leaderboard (no limit)", async () => {
      execute.mockResolvedValue({ data: [] });
      const resource = createReputationResource(mockClient);
      await resource.leaderboard();
      expect(execute).toHaveBeenCalledWith(["reputation", "leaderboard"]);
    });

    it("should pass --limit when provided", async () => {
      execute.mockResolvedValue({ data: [] });
      const resource = createReputationResource(mockClient);
      await resource.leaderboard(10);
      expect(execute).toHaveBeenCalledWith(["reputation", "leaderboard", "--limit", "10"]);
    });
  });

  describe("factors", () => {
    it("should call execute with reputation factors (no agent)", async () => {
      execute.mockResolvedValue({ agent: "me", total_score: "85", data: [] });
      const resource = createReputationResource(mockClient);
      await resource.factors();
      expect(execute).toHaveBeenCalledWith(["reputation", "factors"]);
    });

    it("should pass --agent when provided", async () => {
      execute.mockResolvedValue({ agent: "did:neunode:abc", data: [] });
      const resource = createReputationResource(mockClient);
      await resource.factors("did:neunode:abc");
      expect(execute).toHaveBeenCalledWith(["reputation", "factors", "--agent", "did:neunode:abc"]);
    });
  });
});
