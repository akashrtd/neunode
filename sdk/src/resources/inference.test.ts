import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import { createInferenceResource } from "./inference.js";

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
		knowledge: {} as never,
		discovery: {} as never,
		turboquant: {} as never,
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

	it("should throw if cli transport is missing", () => {
		expect(() =>
			createInferenceResource({ ...mockClient, cli: undefined }),
		).toThrow("CLI transport required");
	});

	describe("request", () => {
		it("should call execute with inference request (required params)", async () => {
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

		it("should pass --temperature when provided", async () => {
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
		it("should call execute with inference list-models (no provider)", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createInferenceResource(mockClient);
			await resource.listModels();
			expect(execute).toHaveBeenCalledWith(["inference", "list-models"]);
		});

		it("should pass --provider when provided", async () => {
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
		it("should call execute with inference providers (no model)", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createInferenceResource(mockClient);
			await resource.providers();
			expect(execute).toHaveBeenCalledWith(["inference", "providers"]);
		});

		it("should pass --model when provided", async () => {
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
		it("should default strategy to 'cheapest'", async () => {
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

		it("should use provided strategy", async () => {
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
		it("should call execute with inference pricing --model --input-tokens --output-tokens", async () => {
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
