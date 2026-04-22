import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import { createTurboquantResource } from "./turboquant.js";

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

describe("createTurboquantResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if cli transport is missing", () => {
		expect(() =>
			createTurboquantResource({ ...mockClient, cli: undefined }),
		).toThrow("CLI transport required");
	});

	describe("compress", () => {
		it("should call execute with gradient profile", async () => {
			execute.mockResolvedValue({ strategy: "int8" });
			const resource = createTurboquantResource(mockClient);
			await resource.compress({
				profile: "gradient",
				dimension: 4096,
				workers: 4,
				bandwidthMbps: 100,
			});
			expect(execute).toHaveBeenCalledWith([
				"turboquant",
				"compress",
				"--profile",
				"gradient",
				"--dimension",
				"4096",
				"--workers",
				"4",
				"--bandwidth-mbps",
				"100",
			]);
		});

		it("should call execute with kv_cache profile", async () => {
			execute.mockResolvedValue({ strategy: "mse", bits: 3.5 });
			const resource = createTurboquantResource(mockClient);
			await resource.compress({
				profile: "kv_cache",
				dimension: 8192,
				targetBits: 3.5,
			});
			expect(execute).toHaveBeenCalledWith([
				"turboquant",
				"compress",
				"--profile",
				"kv_cache",
				"--dimension",
				"8192",
				"--target-bits",
				"3.5",
			]);
		});

		it("should call execute with custom profile", async () => {
			execute.mockResolvedValue({ strategy: "mse", bits: 5 });
			const resource = createTurboquantResource(mockClient);
			await resource.compress({
				profile: "custom",
				dimension: 512,
				bits: 5,
			});
			expect(execute).toHaveBeenCalledWith([
				"turboquant",
				"compress",
				"--profile",
				"custom",
				"--dimension",
				"512",
				"--bits",
				"5",
			]);
		});
	});

	describe("generateCodebook", () => {
		it("should call execute with required params", async () => {
			execute.mockResolvedValue({
				bits: 4,
				levels: [0.1, 0.2, 0.3, 0.4],
				dimension: 4096,
				iterations: 20,
				mse: 0.001,
			});
			const resource = createTurboquantResource(mockClient);
			await resource.generateCodebook({ bits: 4, dimension: 4096 });
			expect(execute).toHaveBeenCalledWith([
				"turboquant",
				"generate-codebook",
				"--bits",
				"4",
				"--dimension",
				"4096",
			]);
		});

		it("should pass optional params when provided", async () => {
			execute.mockResolvedValue({
				bits: 8,
				levels: [],
				dimension: 2048,
				iterations: 50,
				mse: 0.0001,
			});
			const resource = createTurboquantResource(mockClient);
			await resource.generateCodebook({
				bits: 8,
				dimension: 2048,
				maxIterations: 200,
				convergenceThreshold: 1e-10,
				numSamples: 20000,
			});
			expect(execute).toHaveBeenCalledWith([
				"turboquant",
				"generate-codebook",
				"--bits",
				"8",
				"--dimension",
				"2048",
				"--max-iterations",
				"200",
				"--convergence-threshold",
				"1e-10",
				"--num-samples",
				"20000",
			]);
		});
	});
});
