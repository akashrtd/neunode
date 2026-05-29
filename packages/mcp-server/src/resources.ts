/**
 * MCP resource templates for the Neunode MCP server.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { AgnetdClient } from "../client.js";

export function registerResources(
  server: McpServer,
  client: AgnetdClient,
): void {
  // Agent profile resource
  server.resource(
    "agent-profile",
    "neunode://agent/{did}",
    async (uri: URL, { did }: { did: string }) => {
      try {
        const result = await client.getIdentity(decodeURIComponent(did));
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
    "neunode://feed/{did}/{sequence}",
    async (uri: URL, { did, sequence }: { did: string; sequence: string }) => {
      try {
        const result = await client.readFeed({
          author: decodeURIComponent(did),
          limit: Number.parseInt(sequence, 10) || 10,
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
    "neunode://model/{model_id}",
    async (uri: URL, { model_id }: { model_id: string }) => {
      try {
        const result = await client.getModel(decodeURIComponent(model_id));
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
    "neunode://bounty/{bounty_id}",
    async (uri: URL, { bounty_id }: { bounty_id: string }) => {
      try {
        const result = await client.getBounty(decodeURIComponent(bounty_id));
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
