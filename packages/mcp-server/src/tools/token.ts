/**
 * Token tools for the Neunode MCP server.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { AgnetdClient } from "../client.js";

export function registerTokenTools(
  server: McpServer,
  client: AgnetdClient,
): void {
  server.tool(
    "neunode_get_balance",
    "Get token balance for the active agent. Optionally filter by token type.",
    {
      token: z
        .enum(["compute", "train", "bandwidth", "storage"])
        .optional()
        .describe("Token type (omit to get all balances)"),
    },
    async ({ token }) => {
      const result = await client.getBalance(token);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_transfer",
    "Transfer tokens to another agent",
    {
      to: z
        .string()
        .min(1)
        .describe("Recipient DID (must start with 'did:')"),
      amount: z
        .number()
        .int()
        .positive()
        .describe("Amount to transfer"),
      token: z
        .enum(["compute", "train", "bandwidth", "storage"])
        .optional()
        .describe("Token type (default: compute)"),
    },
    async ({ to, amount, token }) => {
      const result = await client.transfer({ to, amount, token });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_stake",
    "Stake tokens to increase reputation and earn rewards",
    {
      amount: z
        .number()
        .int()
        .positive()
        .describe("Amount to stake"),
      token: z
        .enum(["compute", "train", "bandwidth", "storage"])
        .optional()
        .describe("Token type (default: compute)"),
    },
    async ({ amount, token }) => {
      const result = await client.stake({ amount, token });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_unstake",
    "Unstake tokens (subject to unbonding period)",
    {
      amount: z
        .number()
        .int()
        .positive()
        .describe("Amount to unstake"),
    },
    async ({ amount }) => {
      const result = await client.unstake(amount);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_get_staking_info",
    "Get current staking status for the active agent across all token types",
    {},
    async () => {
      const result = await client.getStakingInfo();
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );
}
