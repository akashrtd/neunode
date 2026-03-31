import { describe, it, expect, vi, beforeEach } from "vitest";
import { createTrainResource } from "./train.js";
import type { NeunodeClient } from "../client/client.js";
import { CliTransport } from "../transport/cli-transport.js";

function makeMockClient(): NeunodeClient {
  const execute = vi.fn();
  const transport = { execute, executeMulti: vi.fn(), executeRaw: vi.fn() } as unknown as CliTransport;
  return {
    cli: transport, viem: undefined, transportMode: "cli",
    identity: {} as never, config: {} as never, feed: {} as never, mesh: {} as never,
    model: {} as never, train: {} as never, bounty: {} as never, token: {} as never,
    reputation: {} as never, inference: {} as never, extend: vi.fn(),
  };
}

describe("createTrainResource", () => {
  let mockClient: NeunodeClient;
  let execute: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockClient = makeMockClient();
    execute = mockClient.cli!.execute as ReturnType<typeof vi.fn>;
  });

  it("should throw if cli transport is missing", () => {
    expect(() => createTrainResource({ ...mockClient, cli: undefined })).toThrow("CLI transport required");
  });

  describe("start", () => {
    it("should call execute with train start --model --dataset", async () => {
      execute.mockResolvedValue({ "Job ID": "job_123", Model: "llama-3b", Status: "Running" });
      const resource = createTrainResource(mockClient);
      await resource.start({ model: "llama-3b", dataset: "medical-data" });
      expect(execute).toHaveBeenCalledWith([
        "train", "start", "--model", "llama-3b", "--dataset", "medical-data",
      ]);
    });

    it("should pass --config when provided", async () => {
      execute.mockResolvedValue({ "Job ID": "job_123" });
      const resource = createTrainResource(mockClient);
      await resource.start({ model: "llama-3b", dataset: "data", config: "/tmp/train.toml" });
      expect(execute).toHaveBeenCalledWith([
        "train", "start", "--model", "llama-3b", "--dataset", "data", "--config", "/tmp/train.toml",
      ]);
    });
  });

  describe("status", () => {
    it("should call execute with train status (no jobId)", async () => {
      execute.mockResolvedValue({ job_id: "job_123", status: "Running" });
      const resource = createTrainResource(mockClient);
      await resource.status();
      expect(execute).toHaveBeenCalledWith(["train", "status"]);
    });

    it("should pass --job-id when provided", async () => {
      execute.mockResolvedValue({ job_id: "job_123", status: "Running" });
      const resource = createTrainResource(mockClient);
      await resource.status("job_123");
      expect(execute).toHaveBeenCalledWith(["train", "status", "--job-id", "job_123"]);
    });
  });

  describe("stop", () => {
    it("should call execute with train stop --job-id", async () => {
      execute.mockResolvedValue({ job_id: "job_123", action: "stop", status: "Stopped" });
      const resource = createTrainResource(mockClient);
      await resource.stop("job_123");
      expect(execute).toHaveBeenCalledWith(["train", "stop", "--job-id", "job_123"]);
    });
  });

  describe("list", () => {
    it("should call execute with train list", async () => {
      execute.mockResolvedValue({ data: [] });
      const resource = createTrainResource(mockClient);
      await resource.list();
      expect(execute).toHaveBeenCalledWith(["train", "list"]);
    });
  });

  describe("registerWorker", () => {
    it("should call execute with train worker-register args", async () => {
      execute.mockResolvedValue({
        worker_id: "w_001", gpu_count: 4, gpu_memory_gb: 80,
        max_model_params: 7, supports_bf16: false, status: "available", registered_at: 1700000000,
      });
      const resource = createTrainResource(mockClient);
      await resource.registerWorker({ gpuCount: 4, gpuMemoryGb: 80, maxModelParams: 7 });
      expect(execute).toHaveBeenCalledWith([
        "train", "worker-register", "--gpu-count", "4", "--gpu-memory", "80", "--max-params", "7",
      ]);
    });

    it("should pass --bf16 when supportsBf16 is true", async () => {
      execute.mockResolvedValue({
        worker_id: "w_002", gpu_count: 8, gpu_memory_gb: 80,
        max_model_params: 70, supports_bf16: true, status: "available", registered_at: 1700000000,
      });
      const resource = createTrainResource(mockClient);
      await resource.registerWorker({ gpuCount: 8, gpuMemoryGb: 80, maxModelParams: 70, supportsBf16: true });
      expect(execute).toHaveBeenCalledWith([
        "train", "worker-register", "--gpu-count", "8", "--gpu-memory", "80", "--max-params", "70", "--bf16",
      ]);
    });
  });

  describe("listWorkers", () => {
    it("should call execute with train worker-list (no params)", async () => {
      execute.mockResolvedValue({ data: [] });
      const resource = createTrainResource(mockClient);
      await resource.listWorkers();
      expect(execute).toHaveBeenCalledWith(["train", "worker-list"]);
    });

    it("should pass filter params when provided", async () => {
      execute.mockResolvedValue({ data: [] });
      const resource = createTrainResource(mockClient);
      await resource.listWorkers({ minGpu: 4, minMemory: 40 });
      expect(execute).toHaveBeenCalledWith([
        "train", "worker-list", "--min-gpu", "4", "--min-memory", "40",
      ]);
    });
  });

  describe("coordinatorStatus", () => {
    it("should pass job ID", async () => {
      execute.mockResolvedValue({
        job_id: "job_abc", status: "Running", workers_count: 3,
        current_outer_step: 42, created_at: 1700000000,
      });
      const resource = createTrainResource(mockClient);
      await resource.coordinatorStatus({ jobId: "job_abc" });
      expect(execute).toHaveBeenCalledWith(["train", "coordinator-status", "--job-id", "job_abc"]);
    });
  });
});
