/**
 * Model registry tools for the Neunode MCP server.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { AgnetdClient } from "../client.js";

export function registerModelTools(
  server: McpServer,
  client: AgnetdClient,
): void {
  server.tool(
    "neunode_register_model",
    "Register a new AI model on the Neunode network",
    {
      name: z.string().min(1).describe("Model name/ID"),
      path: z
        .string()
        .min(1)
        .describe("Path or source of the model"),
    },
    async ({ name, path }) => {
      const result = await client.registerModel({ name, path });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_list_registered_models",
    "List all models registered on the Neunode network",
    {
      provider: z
        .string()
        .optional()
        .describe("Filter by provider substring"),
    },
    async ({ provider }) => {
      const result = await client.listRegisteredModels(provider);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_get_registered_model",
    "Get a registered model by its model ID",
    {
      model_id: z.string().min(1).describe("Registered model ID"),
    },
    async ({ model_id }) => {
      const result = await client.getModel(model_id);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_get_lineage",
    "Get the lineage/provenance chain for a model by its content identifier",
    {
      cid: z.string().min(1).describe("Content identifier (CID) of the model"),
    },
    async ({ cid }) => {
      const result = await client.getLineage(cid);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );
}
