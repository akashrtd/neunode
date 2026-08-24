import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { AgnetdClient } from "../client.js";

const text = (value: unknown) => ({
  content: [{ type: "text" as const, text: JSON.stringify(value, null, 2) }],
});

export function registerTrainingTools(server: McpServer, client: AgnetdClient): void {
  server.tool(
    "neunode_start_training",
    "Start a distributed training job",
    { model: z.string().min(1), dataset: z.string().min(1), config: z.string().optional() },
    async ({ model, dataset, config }) => text(await client.startTraining(model, dataset, config)),
  );
  server.tool(
    "neunode_training_status",
    "Get one training job or the active job status",
    { job_id: z.string().min(1).optional() },
    async ({ job_id }) => text(await client.getTrainingStatus(job_id)),
  );
  server.tool(
    "neunode_stop_training",
    "Stop a distributed training job",
    { job_id: z.string().min(1) },
    async ({ job_id }) => text(await client.stopTraining(job_id)),
  );
}
