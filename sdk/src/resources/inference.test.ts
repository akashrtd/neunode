import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createInferenceResource } from "./inference.js";

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
		lifecycle: {} as never,
		lineage: {} as never,
		verification: {} as never,
		extend: vi.fn(),
	};
}

describe("createInferenceResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both transports are missing", async () => {
		const resource = createInferenceResource({
			...mockClient,
			cli: undefined,
			http: undefined,
		});
		await expect(resource.listModels()).rejects.toThrow(
			"HTTP or CLI transport required",
		);
	});

	describe("request", () => {
		it("should use HTTP transport when available", async () => {
			const expected = { model: "llama-3b", status: "routed" };
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue(expected);
			const resource = createInferenceResource(dualClient);
			const result = await resource.request({
				model: "llama-3b",
				prompt: "hello",
				maxTokens: 512,
			});
			expect(http.post).toHaveBeenCalledWith("/api/v1/inference/request", {
				model: "llama-3b",
				prompt: "hello",
				maxTokens: 512,
			});
			expect(result).toEqual(expected);
		});

		it("should call execute with inference request (required params) via CLI", async () => {
			execute.mockResolvedValue({ model: "llama-3b", status: "routed" });
			const resource = createInferenceResource(mockClient);
			await resource.request({
				model: "llama-3b",
				prompt: "hello",
				maxTokens: 512,
			});
			expect(execute).toHaveBeenCalledWith([
				"inference",
				"request",
				"--model",
				"llama-3b",
				"--prompt",
				"hello",
				"--max-tokens",
				"512",
			]);
		});

		it("should pass --temperature when provided via CLI", async () => {
			execute.mockResolvedValue({ model: "llama-3b", status: "routed" });
			const resource = createInferenceResource(mockClient);
			await resource.request({
				model: "llama-3b",
				prompt: "hello",
				maxTokens: 512,
				temperature: 0.7,
			});
			expect(execute).toHaveBeenCalledWith([
				"inference",
				"request",
				"--model",
				"llama-3b",
				"--prompt",
				"hello",
				"--max-tokens",
				"512",
				"--temperature",
				"0.7",
			]);
		});

		it("should NOT pass --temperature when undefined", async () => {
			execute.mockResolvedValue({ model: "llama-3b" });
			const resource = createInferenceResource(mockClient);
			await resource.request({
				model: "llama-3b",
				prompt: "hi",
				maxTokens: 100,
			});
			const callArgs = execute.mock.calls[0]?.[0] as string[] | undefined;
			expect(callArgs).toBeDefined();
			expect(callArgs).not.toContain("--temperature");
		});

		it("should pass --temperature when 0 (falsy but defined)", async () => {
			execute.mockResolvedValue({ model: "llama-3b" });
			const resource = createInferenceResource(mockClient);
			await resource.request({
				model: "llama-3b",
				prompt: "hi",
				maxTokens: 100,
				temperature: 0,
			});
			const callArgs = execute.mock.calls[0]?.[0] as string[] | undefined;
			expect(callArgs).toBeDefined();
			expect(callArgs).toContain("--temperature");
			expect(callArgs).toContain("0");
		});
	});

	describe("listModels", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createInferenceResource(dualClient);
			await resource.listModels("provider-abc");
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/inference/models?provider=provider-abc",
			);
		});

		it("should call execute with inference list-models (no provider) via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createInferenceResource(mockClient);
			await resource.listModels();
			expect(execute).toHaveBeenCalledWith(["inference", "list-models"]);
		});

		it("should pass --provider when provided via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createInferenceResource(mockClient);
			await resource.listModels("provider-abc");
			expect(execute).toHaveBeenCalledWith([
				"inference",
				"list-models",
				"--provider",
				"provider-abc",
			]);
		});
	});

	describe("providers", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createInferenceResource(dualClient);
			await resource.providers("llama-3b");
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/inference/providers?model=llama-3b",
			);
		});

		it("should call execute with inference providers (no model) via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createInferenceResource(mockClient);
			await resource.providers();
			expect(execute).toHaveBeenCalledWith(["inference", "providers"]);
		});

		it("should pass --model when provided via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createInferenceResource(mockClient);
			await resource.providers("llama-3b");
			expect(execute).toHaveBeenCalledWith([
				"inference",
				"providers",
				"--model",
				"llama-3b",
			]);
		});
	});

	describe("route", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ model: "llama-3b", strategy: "cheapest" });
			const resource = createInferenceResource(dualClient);
			await resource.route("llama-3b", "Fastest");
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/inference/route?model=llama-3b&strategy=Fastest",
			);
		});

		it("should default strategy to 'cheapest' via CLI", async () => {
			execute.mockResolvedValue({ model: "llama-3b", strategy: "cheapest" });
			const resource = createInferenceResource(mockClient);
			await resource.route("llama-3b");
			expect(execute).toHaveBeenCalledWith([
				"inference",
				"route",
				"--model",
				"llama-3b",
				"--strategy",
				"cheapest",
			]);
		});

		it("should use provided strategy via CLI", async () => {
			execute.mockResolvedValue({ model: "llama-3b", strategy: "Fastest" });
			const resource = createInferenceResource(mockClient);
			await resource.route("llama-3b", "Fastest");
			expect(execute).toHaveBeenCalledWith([
				"inference",
				"route",
				"--model",
				"llama-3b",
				"--strategy",
				"Fastest",
			]);
		});
	});

	describe("pricing", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ model: "llama-3b", total_cost: 0.5 });
			const resource = createInferenceResource(dualClient);
			await resource.pricing("llama-3b", 1000, 500);
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/inference/pricing?model=llama-3b&input_tokens=1000&output_tokens=500",
			);
		});

		it("should call execute with inference pricing --model --input-tokens --output-tokens via CLI", async () => {
			execute.mockResolvedValue({ model: "llama-3b", total_cost: 0.5 });
			const resource = createInferenceResource(mockClient);
			await resource.pricing("llama-3b", 1000, 500);
			expect(execute).toHaveBeenCalledWith([
				"inference",
				"pricing",
				"--model",
				"llama-3b",
				"--input-tokens",
				"1000",
				"--output-tokens",
				"500",
			]);
		});
	});
});
