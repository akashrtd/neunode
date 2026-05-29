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
	return {
		async start(params: TrainStartParams): Promise<TrainStartResult> {
			if (client.http) {
				return client.http.post<TrainStartResult>(
					"/api/v1/train/start",
					params,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for train operations");
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
			if (client.http) {
				const qs = new URLSearchParams();
				if (jobId) qs.set("jobId", jobId);
				const query = qs.toString();
				return client.http.get<TrainStatusResult>(
					query ? `/api/v1/train/status?${query}` : "/api/v1/train/status",
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for train operations");
			const args = ["train", "status"];
			if (jobId) args.push("--job-id", jobId);
			return cli.execute<TrainStatusResult>(args);
		},

		async stop(jobId: string): Promise<TrainStopResult> {
			if (client.http) {
				return client.http.post<TrainStopResult>(`/api/v1/train/stop`, {
					jobId,
				});
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for train operations");
			return cli.execute<TrainStopResult>(["train", "stop", "--job-id", jobId]);
		},

		async list(): Promise<TrainListResult> {
			if (client.http) {
				return client.http.get<TrainListResult>("/api/v1/train/list");
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for train operations");
			return cli.execute<TrainListResult>(["train", "list"]);
		},

		async registerWorker(
			params: WorkerRegisterParams,
		): Promise<WorkerRegisterResult> {
			if (client.http) {
				return client.http.post<WorkerRegisterResult>(
					"/api/v1/train/worker-register",
					params,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for train operations");
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
			if (client.http) {
				const qs = new URLSearchParams();
				if (params?.minGpu) qs.set("minGpu", String(params.minGpu));
				if (params?.minMemory) qs.set("minMemory", String(params.minMemory));
				const query = qs.toString();
				return client.http.get<WorkerListResult>(
					query ? `/api/v1/train/workers?${query}` : "/api/v1/train/workers",
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for train operations");
			const args = ["train", "worker-list"];
			if (params?.minGpu) args.push("--min-gpu", String(params.minGpu));
			if (params?.minMemory)
				args.push("--min-memory", String(params.minMemory));
			return cli.execute<WorkerListResult>(args);
		},

		async coordinatorStatus(
			params: CoordinatorStatusParams,
		): Promise<CoordinatorStatusResult> {
			if (client.http) {
				return client.http.get<CoordinatorStatusResult>(
					`/api/v1/train/coordinator-status?jobId=${encodeURIComponent(params.jobId)}`,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for train operations");
			return cli.execute<CoordinatorStatusResult>([
				"train",
				"coordinator-status",
				"--job-id",
				params.jobId,
			]);
		},
	};
}
