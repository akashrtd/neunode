import { describe, it, expect, vi, beforeEach } from "vitest";
import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { registerIdentityTools } from "./identity";
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

describe("identity tools", () => {
  let server: McpServer;
  let clientMethods: Record<string, ReturnType<typeof vi.fn>>;

  beforeEach(() => {
    server = createMockServer();
    clientMethods = {
      createIdentity: vi.fn().mockResolvedValue({
        identity: { did: "did:key:abc", method: "key", name: "agent-1", ethereum: "0x0", peer_id: "peer1" },
        card_cid: "QmX",
      }),
      listIdentities: vi.fn().mockResolvedValue([
        { did: "did:key:abc", status: "active" },
      ]),
      whoami: vi.fn().mockResolvedValue({
        did: "did:key:abc",
        method: "key",
        name: "agent-1",
        ethereum: "0x0",
        peer_id: "peer1",
      }),
      getIdentity: vi.fn().mockResolvedValue({
        did: "did:key:abc",
        method: "key",
        name: "agent-1",
        ethereum: "0x0",
        peer_id: "peer1",
      }),
    };
  });

  const client = () => clientMethods as unknown as AgnetdClient;

  it("registers all four identity tools", () => {
    registerIdentityTools(server, client());
    const calls = getMockToolFn(server).mock.calls as Array<[string, unknown]>;
    const names = calls.map((c) => c[0]);
    expect(names).toContain("neunode_create_identity");
    expect(names).toContain("neunode_list_identities");
    expect(names).toContain("neunode_whoami");
    expect(names).toContain("neunode_get_identity");
    expect(names).toHaveLength(4);
  });

  describe("neunode_create_identity", () => {
    it("validates name and method schema", () => {
      registerIdentityTools(server, client());
      const [schema] = requireToolCall(server, "neunode_create_identity");

      expect(schema["name"]!.parse("agent-1")).toBe("agent-1");
      expect(() => schema["name"]!.parse("")).toThrow();
      expect(() => schema["name"]!.parse(123)).toThrow();

      expect(schema["method"]!.parse("key")).toBe("key");
      expect(schema["method"]!.parse(undefined)).toBe(undefined);
      expect(() => schema["method"]!.parse("invalid")).toThrow();
    });

    it("handler calls client.createIdentity with params", async () => {
      registerIdentityTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_create_identity");
      const result = await handler({ name: "test-agent", method: "key" });

      expect(clientMethods.createIdentity).toHaveBeenCalledWith({
        name: "test-agent",
        method: "key",
      });
      expect((parseText(result) as { identity: { name: string } }).identity.name).toBe("agent-1");
    });

    it("handler defaults method to undefined when omitted", async () => {
      registerIdentityTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_create_identity");
      await handler({ name: "agent-x" });

      expect(clientMethods.createIdentity).toHaveBeenCalledWith({
        name: "agent-x",
        method: undefined,
      });
    });
  });

  describe("neunode_list_identities", () => {
    it("handler calls client.listIdentities", async () => {
      registerIdentityTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_list_identities");
      const result = await handler({});

      expect(clientMethods.listIdentities).toHaveBeenCalled();
      const parsed = parseText(result) as Array<{ did: string }>;
      expect(parsed).toHaveLength(1);
      expect(parsed[0]!.did).toBe("did:key:abc");
    });
  });

  describe("neunode_whoami", () => {
    it("handler calls client.whoami", async () => {
      registerIdentityTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_whoami");
      const result = await handler({});

      expect(clientMethods.whoami).toHaveBeenCalled();
      const parsed = parseText(result) as { did: string; name: string };
      expect(parsed.did).toBe("did:key:abc");
      expect(parsed.name).toBe("agent-1");
    });
  });

  describe("neunode_get_identity", () => {
    it("has a schema accepting did string", () => {
      registerIdentityTools(server, client());
      const [schema] = requireToolCall(server, "neunode_get_identity");
      expect(schema["did"]!.parse("did:key:abc")).toBe("did:key:abc");
      expect(schema["did"]!.parse("any-string")).toBe("any-string");
      expect(schema["did"]!.parse("")).toBe("");
      expect(() => schema["did"]!.parse(123)).toThrow();
    });

    it("handler calls client.getIdentity with did", async () => {
      registerIdentityTools(server, client());
      const [, handler] = requireToolCall(server, "neunode_get_identity");
      const result = await handler({ did: "did:key:abc" });

      expect(clientMethods.getIdentity).toHaveBeenCalledWith("did:key:abc");
      expect((parseText(result) as { did: string }).did).toBe("did:key:abc");
    });
  });
});
