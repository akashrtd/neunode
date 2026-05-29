/**
 * MCP resource templates for the Neunode MCP server.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { ResourceTemplate } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { Variables } from "@modelcontextprotocol/sdk/shared/uriTemplate.js";
import type { AgnetdClient } from "./client.js";

function varStr(variables: Variables, key: string): string {
  const val = variables[key];
  if (val === undefined) {
    throw new Error(`missing template variable '${key}'`);
  }
  return typeof val === "string" ? val : val[0]!;
}

export function registerResources(
  server: McpServer,
  client: AgnetdClient,
): void {
  // Agent profile resource
  server.resource(
    "agent-profile",
    new ResourceTemplate("neunode://agent/{did}", { list: undefined }),
    async (uri: URL, variables: Variables) => {
      try {
        const result = await client.getIdentity(decodeURIComponent(varStr(variables, "did")));
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: "application/json",
              text: JSON.stringify(result, null, 2),
            },
          ],
        };
      } catch (err) {
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: "application/json",
              text: JSON.stringify({
                error: err instanceof Error ? err.message : String(err),
              }),
            },
          ],
        };
      }
    },
  );

  // Feed entry resource
  server.resource(
    "feed-entry",
    new ResourceTemplate("neunode://feed/{did}/{sequence}", { list: undefined }),
    async (uri: URL, variables: Variables) => {
      try {
        const result = await client.readFeed({
          author: decodeURIComponent(varStr(variables, "did")),
          limit: Number.parseInt(varStr(variables, "sequence"), 10) || 10,
        });
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: "application/json",
              text: JSON.stringify(result, null, 2),
            },
          ],
        };
      } catch (err) {
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: "application/json",
              text: JSON.stringify({
                error: err instanceof Error ? err.message : String(err),
              }),
            },
          ],
        };
      }
    },
  );

  // Model resource
  server.resource(
    "model",
    new ResourceTemplate("neunode://model/{model_id}", { list: undefined }),
    async (uri: URL, variables: Variables) => {
      try {
        const result = await client.getModel(decodeURIComponent(varStr(variables, "model_id")));
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: "application/json",
              text: JSON.stringify(result, null, 2),
            },
          ],
        };
      } catch (err) {
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: "application/json",
              text: JSON.stringify({
                error: err instanceof Error ? err.message : String(err),
              }),
            },
          ],
        };
      }
    },
  );

  // Bounty resource
  server.resource(
    "bounty",
    new ResourceTemplate("neunode://bounty/{bounty_id}", { list: undefined }),
    async (uri: URL, variables: Variables) => {
      try {
        const result = await client.getBounty(decodeURIComponent(varStr(variables, "bounty_id")));
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: "application/json",
              text: JSON.stringify(result, null, 2),
            },
          ],
        };
      } catch (err) {
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: "application/json",
              text: JSON.stringify({
                error: err instanceof Error ? err.message : String(err),
              }),
            },
          ],
        };
      }
    },
  );
}
