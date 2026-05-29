/**
 * Identity tools for the Neunode MCP server.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { AgnetdClient } from "../client.js";

export function registerIdentityTools(
  server: McpServer,
  client: AgnetdClient,
): void {
  server.tool(
    "neunode_create_identity",
    "Create a new agent identity on the Neunode network",
    {
      name: z.string().min(1).describe("Human-readable name for the agent"),
      method: z
        .enum(["key", "neunode"])
        .optional()
        .describe("DID method to use (default: key)"),
    },
    async ({ name, method }) => {
      const result = await client.createIdentity({ name, method });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_list_identities",
    "List all identities stored in the local agnetd daemon",
    {},
    async () => {
      const result = await client.listIdentities();
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_whoami",
    "Get the currently active agent identity (the one set as default)",
    {},
    async () => {
      const result = await client.whoami();
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_get_identity",
    "Get details of a specific identity by DID",
    {
      did: z.string().describe("DID of the identity to look up"),
    },
    async ({ did }) => {
      const result = await client.getIdentity(did);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );
}
