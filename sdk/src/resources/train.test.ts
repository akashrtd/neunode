import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createTrainResource } from "./train.js";

function makeMockClient(
	opts: { withHttp?: boolean; withCli?: boolean } = {},
): NeunodeClient {
	const execute = vi.fn();
	const transport = {
		execute,
		executeMulti: vi.fn(),
		executeRaw: vi.fn(),
	} as unknown as CliTransport;

	const httpGet = vi.fn();
	const httpPost = vi.fn();
	const httpTransport = {
		get: httpGet,
		post: httpPost,
		put: vi.fn(),
		delete: vi.fn(),
	} as unknown as HttpTransport;

	return {
		cli: opts.withHttp && !opts.withCli ? undefined : transport,
		http: opts.withHttp ? httpTransport : undefined,
		viem: undefined,
		transportMode: "cli",
		identity: {} as never,
		config: {} as never,
		feed: {} as never,
		mesh: {} as never,
		model: {} as never,
		train: {} as never,
		bounty: {} as never,
		token: {} as never,
		reputation: {} as never,
		inference: {} as never,
		knowledge: {} as never,
		discovery: {} as never,
		turboquant: {} as never,
		extend: vi.fn(),
	};
}

describe("createTrainResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both transports are missing", async () => {
		const resource = createTrainResource({
			...mockClient,
			cli: undefined,
			http: undefined,
		});
		await expect(resource.list()).rejects.toThrow(
			"HTTP or CLI transport required",
		);
	});

	describe("start", () => {
		it("should use HTTP transport when available", async () => {
			const expected = {
				"Job ID": "job_http",
				Model: "llama-3b",
				Status: "Running",
			};
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue(expected);
			const resource = createTrainResource(dualClient);
			const result = await resource.start({
				model: "llama-3b",
				dataset: "medical-data",
			});
			expect(http.post).toHaveBeenCalledWith("/api/v1/train/start", {
				model: "llama-3b",
				dataset: "medical-data",
			});
			expect(result).toEqual(expected);
		});

		it("should call execute with train start --model --dataset via CLI", async () => {
			execute.mockResolvedValue({
				"Job ID": "job_123",
				Model: "llama-3b",
				Status: "Running",
			});
			const resource = createTrainResource(mockClient);
			await resource.start({ model: "llama-3b", dataset: "medical-data" });
			expect(execute).toHaveBeenCalledWith([
				"train",
				"start",
				"--model",
				"llama-3b",
				"--dataset",
				"medical-data",
			]);
		});

		it("should pass --config when provided via CLI", async () => {
			execute.mockResolvedValue({ "Job ID": "job_123" });
			const resource = createTrainResource(mockClient);
			await resource.start({
				model: "llama-3b",
				dataset: "data",
				config: "/tmp/train.toml",
			});
			expect(execute).toHaveBeenCalledWith([
				"train",
				"start",
				"--model",
				"llama-3b",
				"--dataset",
				"data",
				"--config",
				"/tmp/train.toml",
			]);
		});
	});

	describe("status", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ job_id: "job_123", status: "Running" });
			const resource = createTrainResource(dualClient);
			await resource.status("job_123");
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/train/status?jobId=job_123",
			);
		});

		it("should call execute with train status (no jobId) via CLI", async () => {
			execute.mockResolvedValue({ job_id: "job_123", status: "Running" });
			const resource = createTrainResource(mockClient);
			await resource.status();
			expect(execute).toHaveBeenCalledWith(["train", "status"]);
		});

		it("should pass --job-id when provided via CLI", async () => {
			execute.mockResolvedValue({ job_id: "job_123", status: "Running" });
			const resource = createTrainResource(mockClient);
			await resource.status("job_123");
			expect(execute).toHaveBeenCalledWith([
				"train",
				"status",
				"--job-id",
				"job_123",
			]);
		});
	});

	describe("stop", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({ job_id: "job_123", status: "Stopped" });
			const resource = createTrainResource(dualClient);
			await resource.stop("job_123");
			expect(http.post).toHaveBeenCalledWith("/api/v1/train/stop", {
				jobId: "job_123",
			});
		});

		it("should call execute with train stop --job-id via CLI", async () => {
			execute.mockResolvedValue({
				job_id: "job_123",
				action: "stop",
				status: "Stopped",
			});
			const resource = createTrainResource(mockClient);
			await resource.stop("job_123");
			expect(execute).toHaveBeenCalledWith([
				"train",
				"stop",
				"--job-id",
				"job_123",
			]);
		});
	});

	describe("list", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createTrainResource(dualClient);
			await resource.list();
			expect(http.get).toHaveBeenCalledWith("/api/v1/train/list");
		});

		it("should call execute with train list via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createTrainResource(mockClient);
			await resource.list();
			expect(execute).toHaveBeenCalledWith(["train", "list"]);
		});
	});

	describe("registerWorker", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({ worker_id: "w_001", status: "available" });
			const resource = createTrainResource(dualClient);
			await resource.registerWorker({
				gpuCount: 4,
				gpuMemoryGb: 80,
				maxModelParams: 7,
			});
			expect(http.post).toHaveBeenCalledWith("/api/v1/train/worker-register", {
				gpuCount: 4,
				gpuMemoryGb: 80,
				maxModelParams: 7,
			});
		});

		it("should call execute with train worker-register args via CLI", async () => {
			execute.mockResolvedValue({
				worker_id: "w_001",
				gpu_count: 4,
				gpu_memory_gb: 80,
				max_model_params: 7,
				supports_bf16: false,
				status: "available",
				registered_at: 1700000000,
			});
			const resource = createTrainResource(mockClient);
			await resource.registerWorker({
				gpuCount: 4,
				gpuMemoryGb: 80,
				maxModelParams: 7,
			});
			expect(execute).toHaveBeenCalledWith([
				"train",
				"worker-register",
				"--gpu-count",
				"4",
				"--gpu-memory",
				"80",
				"--max-params",
				"7",
			]);
		});

		it("should pass --bf16 when supportsBf16 is true via CLI", async () => {
			execute.mockResolvedValue({
				worker_id: "w_002",
				gpu_count: 8,
				gpu_memory_gb: 80,
				max_model_params: 70,
				supports_bf16: true,
				status: "available",
				registered_at: 1700000000,
			});
			const resource = createTrainResource(mockClient);
			await resource.registerWorker({
				gpuCount: 8,
				gpuMemoryGb: 80,
				maxModelParams: 70,
				supportsBf16: true,
			});
			expect(execute).toHaveBeenCalledWith([
				"train",
				"worker-register",
				"--gpu-count",
				"8",
				"--gpu-memory",
				"80",
				"--max-params",
				"70",
				"--bf16",
			]);
		});
	});

	describe("listWorkers", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createTrainResource(dualClient);
			await resource.listWorkers({ minGpu: 4, minMemory: 40 });
			expect(http.get).toHaveBeenCalled();
			const callUrl = http.get.mock.calls[0]?.[0] as string;
			expect(callUrl).toContain("/api/v1/train/workers?");
			expect(callUrl).toContain("minGpu=4");
			expect(callUrl).toContain("minMemory=40");
		});

		it("should call execute with train worker-list (no params) via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createTrainResource(mockClient);
			await resource.listWorkers();
			expect(execute).toHaveBeenCalledWith(["train", "worker-list"]);
		});

		it("should pass filter params when provided via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createTrainResource(mockClient);
			await resource.listWorkers({ minGpu: 4, minMemory: 40 });
			expect(execute).toHaveBeenCalledWith([
				"train",
				"worker-list",
				"--min-gpu",
				"4",
				"--min-memory",
				"40",
			]);
		});
	});

	describe("coordinatorStatus", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ job_id: "job_abc", status: "Running" });
			const resource = createTrainResource(dualClient);
			await resource.coordinatorStatus({ jobId: "job_abc" });
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/train/coordinator-status?jobId=job_abc",
			);
		});

		it("should pass job ID via CLI", async () => {
			execute.mockResolvedValue({
				job_id: "job_abc",
				status: "Running",
				workers_count: 3,
				current_outer_step: 42,
				created_at: 1700000000,
			});
			const resource = createTrainResource(mockClient);
			await resource.coordinatorStatus({ jobId: "job_abc" });
			expect(execute).toHaveBeenCalledWith([
				"train",
				"coordinator-status",
				"--job-id",
				"job_abc",
			]);
		});
	});
});
