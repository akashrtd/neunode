/**
 * Inference tools for the Neunode MCP server.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { AgnetdClient } from "../client.js";

export function registerInferenceTools(
  server: McpServer,
  client: AgnetdClient,
): void {
  server.tool(
    "neunode_list_inference_models",
    "List available AI models for inference on the Neunode network",
    {
      provider: z
        .string()
        .optional()
        .describe("Filter by provider name substring"),
    },
    async ({ provider }) => {
      const result = await client.listModels(provider);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_list_providers",
    "List inference providers on the Neunode network",
    {
      model: z
        .string()
        .optional()
        .describe("Filter providers that serve a specific model"),
    },
    async ({ model }) => {
      const result = await client.listProviders(model);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_request_inference",
    "Submit an inference request to the Neunode network",
    {
      model: z.string().min(1).describe("Model ID to use for inference"),
      prompt: z.string().min(1).describe("The prompt text to send"),
      max_tokens: z
        .number()
        .int()
        .min(1)
        .optional()
        .describe("Maximum tokens to generate (default: 256)"),
      temperature: z
        .number()
        .min(0)
        .max(2)
        .optional()
        .describe("Sampling temperature 0.0-2.0 (default: 0.7)"),
    },
    async ({ model, prompt, max_tokens, temperature }) => {
      const result = await client.requestInference({
        model,
        prompt,
        max_tokens,
        temperature,
      });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

}
