import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import { createModelResource } from "./model.js";

function makeMockClient(): NeunodeClient {
	const execute = vi.fn();
	const transport = {
		execute,
		executeMulti: vi.fn(),
		executeRaw: vi.fn(),
	} as unknown as CliTransport;
	return {
		cli: transport,
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
		extend: vi.fn(),
	};
}

describe("createModelResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if cli transport is missing", () => {
		expect(() =>
			createModelResource({ ...mockClient, cli: undefined }),
		).toThrow("CLI transport required");
	});

	describe("list", () => {
		it("should call execute with model list (no provider)", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createModelResource(mockClient);
			await resource.list();
			expect(execute).toHaveBeenCalledWith(["model", "list"]);
		});

		it("should pass --provider when provided", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createModelResource(mockClient);
			await resource.list("provider-abc");
			expect(execute).toHaveBeenCalledWith([
				"model",
				"list",
				"--provider",
				"provider-abc",
			]);
		});
	});

	describe("show", () => {
		it("should call execute with model show --model-id", async () => {
			execute.mockResolvedValue({
				"Model ID": "llama-3b",
				"Base Model": "llama",
			});
			const resource = createModelResource(mockClient);
			await resource.show("llama-3b");
			expect(execute).toHaveBeenCalledWith([
				"model",
				"show",
				"--model-id",
				"llama-3b",
			]);
		});
	});

	describe("push", () => {
		it("should call execute with model push --path --name", async () => {
			execute.mockResolvedValue({ Status: "ok", "Model ID": "my-model" });
			const resource = createModelResource(mockClient);
			await resource.push({ path: "/tmp/model.safetensors", name: "my-model" });
			expect(execute).toHaveBeenCalledWith([
				"model",
				"push",
				"--path",
				"/tmp/model.safetensors",
				"--name",
				"my-model",
			]);
		});
	});

	describe("rm", () => {
		it("should call execute with model rm --model-id", async () => {
			execute.mockResolvedValue({
				action: "rm",
				model_id: "my-model",
				status: "ok",
			});
			const resource = createModelResource(mockClient);
			await resource.rm("my-model");
			expect(execute).toHaveBeenCalledWith([
				"model",
				"rm",
				"--model-id",
				"my-model",
			]);
		});
	});
});
