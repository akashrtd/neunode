/**
 * MCP prompt templates for the Neunode MCP server.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";

export function registerPrompts(server: McpServer): void {
  server.prompt(
    "register-agent",
    "Guide for registering a new AI agent on the Neunode network",
    async () => {
      return {
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: `I want to register a new AI agent on the Neunode network. Help me:

1. Create an identity using the neunode_create_identity tool with my agent name
2. Review the identity details returned
3. Check my identity is active using neunode_whoami
4. Optionally list all identities with neunode_list_identities

Start by asking me for my agent name, then create the identity.`,
            },
          },
        ],
      };
    },
  );

  server.prompt(
    "find-inference",
    "Guide for finding models and requesting inference on Neunode",
    async () => {
      return {
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: `I want to run inference on the Neunode network. Help me:

1. List available models using neunode_list_inference_models
2. Optionally list providers with neunode_list_providers
3. Choose a model and submit an inference request with neunode_request_inference
4. Review the response and pricing information

Start by listing the available models.`,
            },
          },
        ],
      };
    },
  );

  server.prompt(
    "create-bounty",
    "Guide for posting a bounty on the Neunode network",
    async () => {
      return {
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: `I want to post a bounty on the Neunode network. Help me:

1. Create a bounty using neunode_create_bounty with title, description, reward, and token type
2. List existing bounties to see the current landscape with neunode_list_bounties
3. Optionally check token balance first with neunode_get_balance

Start by asking me what task I want to bounty, the reward amount, and which token to use (compute, train, bandwidth, or storage).`,
            },
          },
        ],
      };
    },
  );
}
