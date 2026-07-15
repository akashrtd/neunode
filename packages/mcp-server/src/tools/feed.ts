/**
 * Feed tools for the Neunode MCP server.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { AgnetdClient } from "../client.js";

export function registerFeedTools(
  server: McpServer,
  client: AgnetdClient,
): void {
  server.tool(
    "neunode_post_feed",
    "Post a message or event to the Neunode feed",
    {
      kind: z
        .number()
        .int()
        .describe(
          "Event kind code (9001=POST, 9002=REPLY, 1=CAPABILITY, 1000=BOUNTY_CREATED, etc.)",
        ),
      content: z.string().min(1).describe("Content of the feed event"),
      tags: z
        .array(z.string())
        .optional()
        .describe("Optional tags for the event"),
    },
    async ({ kind, content, tags }) => {
      const result = await client.postFeed({ kind, content, tags });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_read_feed",
    "Read feed events, optionally filtered by kind, author, or limit",
    {
      kind: z
        .number()
        .int()
        .optional()
        .describe("Filter by event kind code"),
      author: z
        .string()
        .optional()
        .describe("Filter by author DID"),
      limit: z
        .number()
        .int()
        .min(1)
        .max(200)
        .optional()
        .describe("Maximum number of events to return (default: 50)"),
    },
    async ({ kind, author, limit }) => {
      const result = await client.readFeed({ kind, author, limit });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

}
