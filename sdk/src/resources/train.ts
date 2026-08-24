import type { NeunodeClient } from "../client/client.js";

export interface TrainStartParams {
	model: string;
	dataset: string;
	config?: string;
}

export interface TrainStartResult {
	job_id: string;
	model: string;
	dataset: string;
	status: string;
	created_at: number;
	method: string;
	config?: {
		local_steps: number;
		inner_lr: number;
		outer_lr: number;
		batch_size: number;
		max_workers: number;
		async_mode: boolean;
	};
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

export type TrainListResult = TrainStartResult[];

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
}

export interface WorkerListParams {
	minGpu?: number;
	minMemory?: number;
}

export type WorkerListResult = WorkerRegisterResult[];

export interface CoordinatorStatusParams {
	jobId: string;
}

export interface CoordinatorStatusResult {
	job_id: string;
	job_status: string;
	model: string;
	method: string;
	config: unknown;
	coordinator: {
		active_workers: number;
		phase: string;
	};
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
			return client.http.post<TrainStartResult>("/api/v1/train/start", params);
		},

		async status(jobId?: string): Promise<TrainStatusResult> {
			const qs = new URLSearchParams();
			if (jobId) qs.set("job_id", jobId);
			const query = qs.toString();
			return client.http.get<TrainStatusResult>(
				query ? `/api/v1/train/status?${query}` : "/api/v1/train/status",
			);
		},

		async stop(jobId: string): Promise<TrainStopResult> {
			return client.http.post<TrainStopResult>(`/api/v1/train/stop`, {
				job_id: jobId,
			});
		},

		async list(): Promise<TrainListResult> {
			return client.http.get<TrainListResult>("/api/v1/train/jobs");
		},

		async registerWorker(
			params: WorkerRegisterParams,
		): Promise<WorkerRegisterResult> {
			return client.http.post<WorkerRegisterResult>(
				"/api/v1/train/worker-register",
				{
					gpu_count: params.gpuCount,
					gpu_memory: params.gpuMemoryGb,
					max_params: params.maxModelParams,
					bf16: params.supportsBf16 ?? false,
				},
			);
		},

		async listWorkers(params?: WorkerListParams): Promise<WorkerListResult> {
			const qs = new URLSearchParams();
			if (params?.minGpu) qs.set("min_gpu", String(params.minGpu));
			if (params?.minMemory) qs.set("min_memory", String(params.minMemory));
			const query = qs.toString();
			return client.http.get<WorkerListResult>(
				query ? `/api/v1/train/workers?${query}` : "/api/v1/train/workers",
			);
		},

		async coordinatorStatus(
			params: CoordinatorStatusParams,
		): Promise<CoordinatorStatusResult> {
			return client.http.get<CoordinatorStatusResult>(
				`/api/v1/train/coordinator-status?job_id=${encodeURIComponent(params.jobId)}`,
			);
		},
	};
}
