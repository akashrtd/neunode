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

export interface WorkerRegisterParams {
	gpuCount: number;
	gpuMemoryGb: number;
	maxModelParams: number;
	supportsBf16?: boolean;
}

export interface WorkerRegisterResult {
	worker_id: string;
	gpu_count: number;
	gpu_memory_gb: number;
	max_model_params: number;
	supports_bf16: boolean;
	status: string;
	registered_at: number;
}

export interface WorkerListParams {
	minGpu?: number;
	minMemory?: number;
}

export interface WorkerListResult {
	data: Array<{
		worker_id: string;
		gpu_count: number;
		gpu_memory_gb: number;
		supports_bf16: boolean;
		status: string;
	}>;
}

export interface CoordinatorStatusParams {
	jobId: string;
}

export interface CoordinatorStatusResult {
	job_id: string;
	status: string;
	workers_count: number;
	current_outer_step: number;
	created_at: number;
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
	/** Register this agent as a training compute provider. */
	registerWorker(params: WorkerRegisterParams): Promise<WorkerRegisterResult>;
	/** List available training workers. */
	listWorkers(params?: WorkerListParams): Promise<WorkerListResult>;
	/** Show training coordinator status for a job. */
	coordinatorStatus(
		params: CoordinatorStatusParams,
	): Promise<CoordinatorStatusResult>;
}

export function createTrainResource(client: NeunodeClient): TrainResource {
	const cli = client.cli;
	if (!cli) throw new Error("CLI transport required for train operations");

	return {
		async start(params: TrainStartParams): Promise<TrainStartResult> {
			const args = [
				"train",
				"start",
				"--model",
				params.model,
				"--dataset",
				params.dataset,
			];
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

		async registerWorker(
			params: WorkerRegisterParams,
		): Promise<WorkerRegisterResult> {
			const args = [
				"train",
				"worker-register",
				"--gpu-count",
				String(params.gpuCount),
				"--gpu-memory",
				String(params.gpuMemoryGb),
				"--max-params",
				String(params.maxModelParams),
			];
			if (params.supportsBf16) args.push("--bf16");
			return cli.execute<WorkerRegisterResult>(args);
		},

		async listWorkers(params?: WorkerListParams): Promise<WorkerListResult> {
			const args = ["train", "worker-list"];
			if (params?.minGpu) args.push("--min-gpu", String(params.minGpu));
			if (params?.minMemory)
				args.push("--min-memory", String(params.minMemory));
			return cli.execute<WorkerListResult>(args);
		},

		async coordinatorStatus(
			params: CoordinatorStatusParams,
		): Promise<CoordinatorStatusResult> {
			return cli.execute<CoordinatorStatusResult>([
				"train",
				"coordinator-status",
				"--job-id",
				params.jobId,
			]);
		},
	};
}
