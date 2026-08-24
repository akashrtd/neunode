import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { AgnetdClient } from "../client.js";

const text = (value: unknown) => ({
  content: [{ type: "text" as const, text: JSON.stringify(value, null, 2) }],
});

export function registerReputationTools(server: McpServer, client: AgnetdClient): void {
  server.tool(
    "neunode_get_reputation",
    "Get an agent reputation score and factor breakdown",
    { agent: z.string().min(1).optional() },
    async ({ agent }) => text(await client.getReputation(agent)),
  );
  server.tool(
    "neunode_attest_reputation",
    "Publish a signed quality attestation for another agent",
    { to: z.string().min(1), score: z.number().min(0).max(100), comment: z.string().optional() },
    async ({ to, score, comment }) => text(await client.attestReputation(to, score, comment)),
  );
  server.tool(
    "neunode_reputation_leaderboard",
    "List agents ordered by reputation",
    { limit: z.number().int().positive().max(1000).optional() },
    async ({ limit }) => text(await client.getReputationLeaderboard(limit)),
  );
}
