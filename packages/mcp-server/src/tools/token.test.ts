import { describe, it, expect, vi, beforeEach } from "vitest";
import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { registerTokenTools } from "./token";
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

describe("token tools", () => {
  let server: McpServer;
  let clientMethods: Record<string, ReturnType<typeof vi.fn>>;

  beforeEach(() => {
    server = createMockServer();
    clientMethods = {
      getBalance: vi.fn().mockResolvedValue({
        balances: [
          { token: "compute", balance: 1000, staked: 200 },
          { token: "train", balance: 500, staked: 50 },
        ],
      }),
      transfer: vi.fn().mockResolvedValue({
        from: "did:key:sender",
        to: "did:key:receiver",
        amount: 100,
        token: "compute",
        state: "completed",
      }),
      stake: vi.fn().mockResolvedValue({
        amount: 100,
        token: "compute",
        state: "staked",
        unbonding_period_secs: 86400,
      }),
      unstake: vi.fn().mockResolvedValue({
        amount: 50,
        token: "compute",
        unbond_at: 1700000000,
        state: "unbonding",
      }),
      getStakingInfo: vi.fn().mockResolvedValue({
        total_staked: 250,
        entries: [
          { amount: 200, token: "compute", available: 800 },
          { amount: 50, token: "train", available: 450 },
        ],
      }),
    };
  });

  const client = () => clientMethods as unknown as AgnetdClient;

  it("registers all five token tools", () => {
    registerTokenTools(server, client());
    const calls = getMockToolFn(server).mock.calls as Array<[string, unknown]>;
    const names = calls.map((c) => c[0]);
    expect(names).toContain("neunode_get_balance");
    expect(names).toContain("neunode_transfer");
    expect(names).toContain("neunode_stake");
    expect(names).toContain("neunode_unstake");
    expect(names).toContain("neunode_get_staking_info");
    expect(names).toHaveLength(5);
  });

  describe("neunode_get_balance", () => {
    it("validates optional token enum", () => {
      registerTokenTools(server, client());
      const [schema] = requireToolCall(server, "neunode_get_balance");

      expect(schema["token"]!.parse("compute")).toBe("compute");
      expect(schema["token"]!.parse("train")).toBe("train");
      expect(schema["token"]!.parse("bandwidth")).toBe("bandwidth");
      expect(schema["token"]!.parse("storage")).toBe("storage");
      expect(schema["token"]!.parse(undefined)).toBe(undefined);
      expect(() => schema["token"]!.parse("gold")).toThrow();
    });

    it("handler calls client.getBalance with token", async () => {
      registerTokenTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_get_balance");
      const result = await handler({ token: "compute" });

      expect(clientMethods.getBalance).toHaveBeenCalledWith("compute");
      const parsed = parseText(result) as { balances: unknown[] };
      expect(parsed.balances).toHaveLength(2);
    });

    it("handler calls client.getBalance without token for all balances", async () => {
      registerTokenTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_get_balance");
      await handler({});

      expect(clientMethods.getBalance).toHaveBeenCalledWith(undefined);
    });
  });

  describe("neunode_transfer", () => {
    it("validates required to and amount fields", () => {
      registerTokenTools(server, client());
      const [schema] = requireToolCall(server, "neunode_transfer");

      expect(schema["to"]!.parse("did:key:abc")).toBe("did:key:abc");
      expect(() => schema["to"]!.parse("")).toThrow();

      expect(schema["amount"]!.parse(100)).toBe(100);
      expect(() => schema["amount"]!.parse(0)).toThrow();
      expect(() => schema["amount"]!.parse(-10)).toThrow();
      expect(() => schema["amount"]!.parse(1.5)).toThrow();

      expect(schema["token"]!.parse("compute")).toBe("compute");
      expect(schema["token"]!.parse(undefined)).toBe(undefined);
    });

    it("handler calls client.transfer with params", async () => {
      registerTokenTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_transfer");
      await handler({ to: "did:key:receiver", amount: 100, token: "compute" });

      expect(clientMethods.transfer).toHaveBeenCalledWith({
        to: "did:key:receiver",
        amount: 100,
        token: "compute",
      });
    });
  });

  describe("neunode_stake", () => {
    it("validates amount as positive integer", () => {
      registerTokenTools(server, client());
      const [schema] = requireToolCall(server, "neunode_stake");

      expect(schema["amount"]!.parse(100)).toBe(100);
      expect(() => schema["amount"]!.parse(0)).toThrow();
      expect(() => schema["amount"]!.parse(-1)).toThrow();
    });

    it("handler calls client.stake", async () => {
      registerTokenTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_stake");
      const result = await handler({ amount: 100, token: "train" });

      expect(clientMethods.stake).toHaveBeenCalledWith({ amount: 100, token: "train" });
      expect((parseText(result) as { state: string }).state).toBe("staked");
    });
  });

  describe("neunode_unstake", () => {
    it("validates amount as positive integer", () => {
      registerTokenTools(server, client());
      const [schema] = requireToolCall(server, "neunode_unstake");

      expect(schema["amount"]!.parse(50)).toBe(50);
      expect(() => schema["amount"]!.parse(0)).toThrow();
    });

    it("handler calls client.unstake", async () => {
      registerTokenTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_unstake");
      await handler({ amount: 50 });

      expect(clientMethods.unstake).toHaveBeenCalledWith(50);
    });
  });

  describe("neunode_get_staking_info", () => {
    it("handler calls client.getStakingInfo", async () => {
      registerTokenTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_get_staking_info");
      const result = await handler({});

      expect(clientMethods.getStakingInfo).toHaveBeenCalled();
      const parsed = parseText(result) as { total_staked: number; entries: unknown[] };
      expect(parsed.total_staked).toBe(250);
      expect(parsed.entries).toHaveLength(2);
    });
  });
});
