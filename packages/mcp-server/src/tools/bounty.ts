/**
 * Bounty tools for the Neunode MCP server.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { AgnetdClient } from "../client.js";

export function registerBountyTools(
  server: McpServer,
  client: AgnetdClient,
): void {
  server.tool(
    "neunode_create_bounty",
    "Create a new bounty on the Neunode network",
    {
      title: z.string().min(1).describe("Bounty title"),
      description: z.string().min(1).describe("Detailed description of the work required"),
      reward: z.number().int().positive().describe("Reward amount in token units"),
      token: z
        .enum(["compute", "train", "bandwidth", "storage"])
        .optional()
        .describe("Token type for reward (default: compute)"),
      claim_deadline: z
        .number()
        .int()
        .positive()
        .optional()
        .describe("Hours until claim deadline (default: 72)"),
      work_deadline: z
        .number()
        .int()
        .positive()
        .optional()
        .describe("Hours until work submission deadline (default: 168)"),
    },
    async ({ title, description, reward, token, claim_deadline, work_deadline }) => {
      const result = await client.createBounty({
        title,
        description,
        reward,
        token,
        claim_deadline,
        work_deadline,
      });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_list_bounties",
    "List bounties on the Neunode network, optionally filtered by state or creator",
    {
      state: z
        .enum(["Open", "Claimed", "Submitted", "UnderReview", "Accepted", "Rejected", "Paid", "Cancelled"])
        .optional()
        .describe("Filter by bounty state"),
      creator: z
        .string()
        .optional()
        .describe("Filter by creator DID"),
      limit: z
        .number()
        .int()
        .min(1)
        .max(200)
        .optional()
        .describe("Maximum results (default: 50)"),
    },
    async ({ state, creator, limit }) => {
      const result = await client.listBounties({ state, creator, limit });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_claim_bounty",
    "Claim an open bounty by staking tokens",
    {
      bounty_id: z.string().min(1).describe("ID of the bounty to claim"),
      stake: z.number().int().positive().describe("Amount to stake as bond"),
    },
    async ({ bounty_id, stake }) => {
      const result = await client.claimBounty(bounty_id, stake);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_submit_bounty",
    "Submit work artifact for a claimed bounty",
    {
      bounty_id: z.string().min(1).describe("ID of the bounty"),
      artifact: z
        .string()
        .min(1)
        .describe("CID or URI of the submitted artifact"),
      evidence: z
        .string()
        .optional()
        .describe("Optional supporting evidence"),
    },
    async ({ bounty_id, artifact, evidence }) => {
      const result = await client.submitBounty(bounty_id, { artifact, evidence });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_review_bounty",
    "Review a submitted bounty with a score and feedback",
    {
      bounty_id: z.string().min(1).describe("ID of the bounty to review"),
      score: z
        .number()
        .int()
        .min(0)
        .max(100)
        .describe("Review score 0-100 (>=60 accepted, <40 rejected)"),
      feedback: z.string().min(1).describe("Review feedback text"),
    },
    async ({ bounty_id, score, feedback }) => {
      const result = await client.reviewBounty(bounty_id, { score, feedback });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );
}
