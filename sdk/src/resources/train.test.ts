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
});
