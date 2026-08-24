import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { AgnetdClient } from "../client.js";

const text = (value: unknown) => ({
  content: [{ type: "text" as const, text: JSON.stringify(value, null, 2) }],
});

export function registerKnowledgeTools(server: McpServer, client: AgnetdClient): void {
  server.tool(
    "neunode_query_knowledge",
    "Query the Neunode knowledge graph",
    {
      subject: z.string().optional(),
      predicate: z.string().optional(),
      object: z.string().optional(),
      graph: z.string().optional(),
      limit: z.number().int().positive().max(1000).optional(),
    },
    async (params) => text(await client.queryKnowledge(params)),
  );
  server.tool(
    "neunode_register_knowledge_agent",
    "Register an agent and its capabilities in the knowledge graph",
    { did: z.string().min(1), capabilities: z.string().min(1) },
    async ({ did, capabilities }) => text(await client.registerKnowledgeAgent(did, capabilities)),
  );
}
