import type { NeunodeClient } from "../client/client.js";

export interface TrainStartParams {
  model: string;
  dataset: string;
  config?: string;
}

export interface TrainStartResult {
  "Job ID": string;
  Model: string;
  Dataset: string;
  Config: string;
  Status: string;
  Method: string;
}

export interface TrainStatusResult {
  job_id: string;
  model: string;
  dataset: string;
  status: string;
  created_at: number;
  method: string;
}

export interface TrainStopResult {
  job_id: string;
  action: string;
  status: string;
}

export interface TrainListResult {
  data: Array<{
    "Job ID": string;
    Model: string;
    Dataset: string;
    Status: string;
    Method: string;
  }>;
}

/** Distributed training job management. */
export interface TrainResource {
  /** Start a new training job. */
  start(params: TrainStartParams): Promise<TrainStartResult>;
  /** Get the status of a training job. */
  status(jobId?: string): Promise<TrainStatusResult>;
  /** Stop a running training job. */
  stop(jobId: string): Promise<TrainStopResult>;
  /** List all training jobs. */
  list(): Promise<TrainListResult>;
}

export function createTrainResource(client: NeunodeClient): TrainResource {
  const cli = client.cli;
  if (!cli) throw new Error("CLI transport required for train operations");

  return {
    async start(params: TrainStartParams): Promise<TrainStartResult> {
      const args = ["train", "start", "--model", params.model, "--dataset", params.dataset];
      if (params.config) args.push("--config", params.config);
      return cli.execute<TrainStartResult>(args);
    },

    async status(jobId?: string): Promise<TrainStatusResult> {
      const args = ["train", "status"];
      if (jobId) args.push("--job-id", jobId);
      return cli.execute<TrainStatusResult>(args);
    },

    async stop(jobId: string): Promise<TrainStopResult> {
      return cli.execute<TrainStopResult>(["train", "stop", "--job-id", jobId]);
    },

    async list(): Promise<TrainListResult> {
      return cli.execute<TrainListResult>(["train", "list"]);
    },
  };
}
