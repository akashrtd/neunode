import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createModelResource } from "./model.js";

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
	const httpDelete = vi.fn();
	const httpTransport = {
		get: httpGet,
		post: httpPost,
		put: vi.fn(),
		delete: httpDelete,
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

describe("createModelResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both transports are missing", async () => {
		const resource = createModelResource({
			...mockClient,
			cli: undefined,
			http: undefined,
		});
		await expect(resource.list()).rejects.toThrow(
			"HTTP or CLI transport required",
		);
	});

	describe("list", () => {
		it("should use HTTP transport when available", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createModelResource(dualClient);
			await resource.list("provider-abc");
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/models?provider=provider-abc",
			);
		});

		it("should call execute with model list (no provider) via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createModelResource(mockClient);
			await resource.list();
			expect(execute).toHaveBeenCalledWith(["model", "list"]);
		});

		it("should pass --provider when provided via CLI", async () => {
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
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({
				"Model ID": "llama-3b",
				"Base Model": "llama",
			});
			const resource = createModelResource(dualClient);
			await resource.show("llama-3b");
			expect(http.get).toHaveBeenCalledWith("/api/v1/models/llama-3b");
		});

		it("should call execute with model show --model-id via CLI", async () => {
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
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({ Status: "ok", "Model ID": "my-model" });
			const resource = createModelResource(dualClient);
			await resource.push({ path: "/tmp/model.safetensors", name: "my-model" });
			expect(http.post).toHaveBeenCalledWith("/api/v1/models", {
				path: "/tmp/model.safetensors",
				name: "my-model",
			});
		});

		it("should call execute with model push --path --name via CLI", async () => {
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
		it("should use HTTP transport with DELETE", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				delete: ReturnType<typeof vi.fn>;
			};
			http.delete.mockResolvedValue({
				action: "rm",
				model_id: "my-model",
				status: "ok",
			});
			const resource = createModelResource(dualClient);
			await resource.rm("my-model");
			expect(http.delete).toHaveBeenCalledWith("/api/v1/models/my-model");
		});

		it("should call execute with model rm --model-id via CLI", async () => {
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
