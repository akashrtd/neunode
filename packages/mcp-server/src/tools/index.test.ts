import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { describe, expect, it, vi } from "vitest";
import type { AgnetdClient } from "../client.js";
import { registerAllTools } from "./index.js";

describe("complete MCP tool surface", () => {
  it("registers all ten Neunode resource modules", () => {
    const tool = vi.fn();
    const server = { tool } as unknown as McpServer;
    registerAllTools(server, {} as AgnetdClient);
    const names = tool.mock.calls.map((call) => call[0] as string);

    for (const expected of [
      "neunode_create_identity",
      "neunode_post_feed",
      "neunode_request_inference",
      "neunode_create_bounty",
      "neunode_get_balance",
      "neunode_register_model",
      "neunode_get_peers",
      "neunode_start_training",
      "neunode_get_reputation",
      "neunode_query_knowledge",
    ]) {
      expect(names).toContain(expected);
    }
  });
});
