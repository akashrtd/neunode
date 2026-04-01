import { describe, it, expect, vi, beforeEach } from "vitest";
import { createBountyResource } from "./bounty.js";
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

describe("createBountyResource", () => {
  let mockClient: NeunodeClient;
  let execute: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockClient = makeMockClient();
    execute = mockClient.cli!.execute as ReturnType<typeof vi.fn>;
  });

  it("should throw if cli transport is missing", () => {
    expect(() => createBountyResource({ ...mockClient, cli: undefined })).toThrow("CLI transport required");
  });

  describe("create", () => {
    it("should call execute with required params only", async () => {
      execute.mockResolvedValue({ id: "bnty_123", state: "Open" });
      const resource = createBountyResource(mockClient);
      await resource.create({
        title: "Train classifier",
        description: ">95% accuracy",
        reward: 1000,
        token: "nCompute",
      });
      expect(execute).toHaveBeenCalledWith([
        "bounty", "create",
        "--title", "Train classifier",
        "--description", ">95% accuracy",
        "--reward", "1000",
        "--token", "nCompute",
      ]);
    });

    it("should pass --claim-deadline and --work-deadline when provided", async () => {
      execute.mockResolvedValue({ id: "bnty_123" });
      const resource = createBountyResource(mockClient);
      await resource.create({
        title: "test", description: "desc", reward: 500, token: "nTrain",
        claimDeadline: 86400, workDeadline: 259200,
      });
      expect(execute).toHaveBeenCalledWith([
        "bounty", "create",
        "--title", "test", "--description", "desc",
        "--reward", "500", "--token", "nTrain",
        "--claim-deadline", "86400", "--work-deadline", "259200",
      ]);
    });
  });

  describe("claim", () => {
    it("should call execute with bounty claim --id --stake", async () => {
      execute.mockResolvedValue({ bounty_id: "bnty_123", claimant: "did:neunode:abc", state: "Claimed" });
      const resource = createBountyResource(mockClient);
      await resource.claim({ id: "bnty_123", stake: 50 });
      expect(execute).toHaveBeenCalledWith([
        "bounty", "claim", "--id", "bnty_123", "--stake", "50",
      ]);
    });
  });

  describe("submit", () => {
    it("should call execute with bounty submit --id --artifact", async () => {
      execute.mockResolvedValue({ bounty_id: "bnty_123", state: "Submitted" });
      const resource = createBountyResource(mockClient);
      await resource.submit({ id: "bnty_123", artifact: "ipfs://QmX7b" });
      expect(execute).toHaveBeenCalledWith([
        "bounty", "submit", "--id", "bnty_123", "--artifact", "ipfs://QmX7b",
      ]);
    });

    it("should pass --evidence when provided", async () => {
      execute.mockResolvedValue({ bounty_id: "bnty_123" });
      const resource = createBountyResource(mockClient);
      await resource.submit({ id: "bnty_123", artifact: "ipfs://QmX7b", evidence: '{"acc":0.96}' });
      expect(execute).toHaveBeenCalledWith([
        "bounty", "submit", "--id", "bnty_123", "--artifact", "ipfs://QmX7b",
        "--evidence", '{"acc":0.96}',
      ]);
    });
  });

  describe("review", () => {
    it("should call execute with bounty review --id --score --feedback", async () => {
      execute.mockResolvedValue({ bounty_id: "bnty_123", score: 9, state: "UnderReview" });
      const resource = createBountyResource(mockClient);
      await resource.review({ id: "bnty_123", score: 9, feedback: "Great work" });
      expect(execute).toHaveBeenCalledWith([
        "bounty", "review", "--id", "bnty_123", "--score", "9", "--feedback", "Great work",
      ]);
    });
  });

  describe("list", () => {
    it("should call execute with bounty list (no params)", async () => {
      execute.mockResolvedValue([]);
      const resource = createBountyResource(mockClient);
      const result = await resource.list();
      expect(execute).toHaveBeenCalledWith(["bounty", "list"]);
      expect(result).toEqual([]);
    });

    it("should pass optional filter params", async () => {
      const mockItems = [{ ID: "bnty_1", State: "Open", Creator: "did:neunode:abc", Claimant: "", Reward: "1000", Deadline: "", Created: "", Escrow: "" }];
      execute.mockResolvedValue(mockItems);
      const resource = createBountyResource(mockClient);
      const result = await resource.list({ state: "Open", creator: "did:neunode:abc", limit: 10 });
      expect(execute).toHaveBeenCalledWith([
        "bounty", "list", "--state", "Open", "--creator", "did:neunode:abc", "--limit", "10",
      ]);
      expect(result).toEqual(mockItems);
    });
  });

  describe("show", () => {
    it("should call execute with bounty show --id", async () => {
      execute.mockResolvedValue({ ID: "bnty_123", State: "Open", Creator: "did:neunode:abc" });
      const resource = createBountyResource(mockClient);
      await resource.show("bnty_123");
      expect(execute).toHaveBeenCalledWith(["bounty", "show", "--id", "bnty_123"]);
    });
  });

  describe("cancel", () => {
    it("should call execute with bounty cancel --id (no reason)", async () => {
      execute.mockResolvedValue({ bounty_id: "bnty_123", state: "Cancelled" });
      const resource = createBountyResource(mockClient);
      await resource.cancel("bnty_123");
      expect(execute).toHaveBeenCalledWith(["bounty", "cancel", "--id", "bnty_123"]);
    });

    it("should pass --reason when provided", async () => {
      execute.mockResolvedValue({ bounty_id: "bnty_123", state: "Cancelled" });
      const resource = createBountyResource(mockClient);
      await resource.cancel("bnty_123", "no longer needed");
      expect(execute).toHaveBeenCalledWith([
        "bounty", "cancel", "--id", "bnty_123", "--reason", "no longer needed",
      ]);
    });
  });
});
