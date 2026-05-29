import { describe, it, expect, vi, beforeEach } from "vitest";
import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { registerBountyTools } from "./bounty";
import type { AgnetdClient } from "../client.js";

type SchemaMap = Record<string, z.ZodTypeAny>;
type Handler = (args: Record<string, unknown>) => Promise<unknown>;

function createMockServer(): McpServer {
  return { tool: vi.fn() } as unknown as McpServer;
}

function getMockToolFn(server: McpServer): ReturnType<typeof vi.fn> {
  return (server as unknown as { tool: ReturnType<typeof vi.fn> }).tool;
}

function requireToolCall(
  server: McpServer,
  name: string,
): [SchemaMap, Handler] {
  const calls = getMockToolFn(server).mock.calls as Array<
    [string, string, SchemaMap, Handler]
  >;
  const call = calls.find((c) => c[0] === name);
  if (!call) throw new Error(`tool '${name}' not registered`);
  return [call[2], call[3]];
}

function parseText(result: unknown): unknown {
  return JSON.parse(
    (result as { content: [{ text: string }] }).content[0]!.text,
  );
}

describe("bounty tools", () => {
  let server: McpServer;
  let clientMethods: Record<string, ReturnType<typeof vi.fn>>;

  const sampleBounty = {
    id: "bounty-1",
    title: "Test Bounty",
    description: "A test bounty",
    state: "Open",
    creator: "did:key:creator",
    claimant: null,
    reward: 100,
    reward_token_type: 0,
    escrow_deposited: 100,
    created_at: 1000000,
    claim_deadline: 1000720,
    work_deadline: 1001680,
    review_deadline: 1002400,
    artifact_hash: null,
    bond: null,
  };

  beforeEach(() => {
    server = createMockServer();
    clientMethods = {
      createBounty: vi.fn().mockResolvedValue(sampleBounty),
      listBounties: vi.fn().mockResolvedValue([sampleBounty]),
      getBounty: vi.fn().mockResolvedValue(sampleBounty),
      claimBounty: vi.fn().mockResolvedValue({ bounty_id: "bounty-1", state: "Claimed" }),
      submitBounty: vi.fn().mockResolvedValue({ bounty_id: "bounty-1", state: "Submitted" }),
      reviewBounty: vi.fn().mockResolvedValue({ bounty_id: "bounty-1", state: "UnderReview" }),
    };
  });

  const client = () => clientMethods as unknown as AgnetdClient;

  it("registers all five bounty tools", () => {
    registerBountyTools(server, client());
    const calls = getMockToolFn(server).mock.calls as Array<[string, unknown]>;
    const names = calls.map((c) => c[0]);
    expect(names).toContain("neunode_create_bounty");
    expect(names).toContain("neunode_list_bounties");
    expect(names).toContain("neunode_claim_bounty");
    expect(names).toContain("neunode_submit_bounty");
    expect(names).toContain("neunode_review_bounty");
    expect(names).toHaveLength(5);
  });

  describe("neunode_create_bounty", () => {
    it("validates required fields: title, description, reward", () => {
      registerBountyTools(server, client());
      const [schema] = requireToolCall(server, "neunode_create_bounty");

      expect(schema["title"]!.parse("My Bounty")).toBe("My Bounty");
      expect(() => schema["title"]!.parse("")).toThrow();

      expect(schema["description"]!.parse("Do work")).toBe("Do work");
      expect(() => schema["description"]!.parse("")).toThrow();

      expect(schema["reward"]!.parse(100)).toBe(100);
      expect(() => schema["reward"]!.parse(0)).toThrow();
      expect(() => schema["reward"]!.parse(-5)).toThrow();
      expect(() => schema["reward"]!.parse(1.5)).toThrow();

      expect(schema["token"]!.parse("compute")).toBe("compute");
      expect(schema["token"]!.parse(undefined)).toBe(undefined);
      expect(() => schema["token"]!.parse("gold")).toThrow();
    });

    it("handler calls client.createBounty with all params", async () => {
      registerBountyTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_create_bounty");
      await handler({
        title: "Bounty Title",
        description: "Do the thing",
        reward: 500,
        token: "train",
        claim_deadline: 48,
        work_deadline: 96,
      });

      expect(clientMethods.createBounty).toHaveBeenCalledWith({
        title: "Bounty Title",
        description: "Do the thing",
        reward: 500,
        token: "train",
        claim_deadline: 48,
        work_deadline: 96,
      });
    });
  });

  describe("neunode_list_bounties", () => {
    it("validates optional filter params", () => {
      registerBountyTools(server, client());
      const [schema] = requireToolCall(server, "neunode_list_bounties");

      expect(schema["state"]!.parse("Open")).toBe("Open");
      expect(schema["state"]!.parse(undefined)).toBe(undefined);
      expect(() => schema["state"]!.parse("Invalid")).toThrow();

      expect(schema["limit"]!.parse(50)).toBe(50);
      expect(schema["limit"]!.parse(undefined)).toBe(undefined);
      expect(() => schema["limit"]!.parse(0)).toThrow();
      expect(() => schema["limit"]!.parse(201)).toThrow();
    });

    it("handler calls client.listBounties with filters", async () => {
      registerBountyTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_list_bounties");
      const result = await handler({ state: "Open", limit: 10 });

      expect(clientMethods.listBounties).toHaveBeenCalledWith({ state: "Open", limit: 10 });
      expect((parseText(result) as unknown[]).length).toBe(1);
    });
  });

  describe("neunode_claim_bounty", () => {
    it("validates bounty_id and stake", () => {
      registerBountyTools(server, client());
      const [schema] = requireToolCall(server, "neunode_claim_bounty");

      expect(schema["bounty_id"]!.parse("abc")).toBe("abc");
      expect(() => schema["bounty_id"]!.parse("")).toThrow();
      expect(schema["stake"]!.parse(50)).toBe(50);
      expect(() => schema["stake"]!.parse(0)).toThrow();
    });

    it("handler calls client.claimBounty", async () => {
      registerBountyTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_claim_bounty");
      await handler({ bounty_id: "bounty-1", stake: 50 });

      expect(clientMethods.claimBounty).toHaveBeenCalledWith("bounty-1", 50);
    });
  });

  describe("neunode_submit_bounty", () => {
    it("validates bounty_id and artifact", () => {
      registerBountyTools(server, client());
      const [schema] = requireToolCall(server, "neunode_submit_bounty");

      expect(schema["artifact"]!.parse("QmX")).toBe("QmX");
      expect(() => schema["artifact"]!.parse("")).toThrow();
    });

    it("handler calls client.submitBounty", async () => {
      registerBountyTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_submit_bounty");
      await handler({ bounty_id: "bounty-1", artifact: "QmX", evidence: "proof" });

      expect(clientMethods.submitBounty).toHaveBeenCalledWith("bounty-1", {
        artifact: "QmX",
        evidence: "proof",
      });
    });
  });

  describe("neunode_review_bounty", () => {
    it("validates score range 0-100", () => {
      registerBountyTools(server, client());
      const [schema] = requireToolCall(server, "neunode_review_bounty");

      expect(schema["score"]!.parse(80)).toBe(80);
      expect(schema["score"]!.parse(0)).toBe(0);
      expect(schema["score"]!.parse(100)).toBe(100);
      expect(() => schema["score"]!.parse(-1)).toThrow();
      expect(() => schema["score"]!.parse(101)).toThrow();
    });

    it("handler calls client.reviewBounty", async () => {
      registerBountyTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_review_bounty");
      await handler({ bounty_id: "bounty-1", score: 85, feedback: "Great work!" });

      expect(clientMethods.reviewBounty).toHaveBeenCalledWith("bounty-1", {
        score: 85,
        feedback: "Great work!",
      });
    });
  });
});
