import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createConfigResource } from "./config.js";

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
	const httpPut = vi.fn();
	const httpTransport = {
		get: httpGet,
		post: vi.fn(),
		put: httpPut,
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
		extend: vi.fn(),
	};
}

describe("createConfigResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both transports are missing", async () => {
		const noTransports = { ...mockClient, cli: undefined, http: undefined };
		const resource = createConfigResource(noTransports);
		await expect(resource.list()).rejects.toThrow(
			"HTTP or CLI transport required",
		);
	});

	describe("set", () => {
		it("should use HTTP transport when available", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				put: ReturnType<typeof vi.fn>;
			};
			http.put.mockResolvedValue(undefined);
			const resource = createConfigResource(dualClient);
			await resource.set({ key: "network", value: "testnet" });
			expect(http.put).toHaveBeenCalledWith("/api/v1/config", {
				key: "network",
				value: "testnet",
			});
		});

		it("should call execute with config set key value via CLI", async () => {
			execute.mockResolvedValue(undefined);
			const resource = createConfigResource(mockClient);
			await resource.set({ key: "network", value: "testnet" });
			expect(execute).toHaveBeenCalledWith([
				"config",
				"set",
				"network",
				"testnet",
			]);
		});
	});

	describe("get", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ network: "testnet" });
			const resource = createConfigResource(dualClient);
			const result = await resource.get("network");
			expect(http.get).toHaveBeenCalledWith("/api/v1/config?key=network");
			expect(result).toBe("testnet");
		});

		it("should call execute with config get key via CLI", async () => {
			execute.mockResolvedValue({ network: "testnet" });
			const resource = createConfigResource(mockClient);
			const result = await resource.get("network");
			expect(result).toBe("testnet");
			expect(execute).toHaveBeenCalledWith(["config", "get", "network"]);
		});

		it("should return empty string for missing key via CLI", async () => {
			execute.mockResolvedValue({});
			const resource = createConfigResource(mockClient);
			const result = await resource.get("nonexistent");
			expect(result).toBe("");
		});
	});

	describe("list", () => {
		it("should use HTTP transport", async () => {
			const expected = { network: "testnet", identity: "did:neunode:abc" };
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue(expected);
			const resource = createConfigResource(dualClient);
			const result = await resource.list();
			expect(http.get).toHaveBeenCalledWith("/api/v1/config");
			expect(result).toEqual(expected);
		});

		it("should call execute with config list via CLI", async () => {
			const expected = { network: "testnet", identity: "did:neunode:abc" };
			execute.mockResolvedValue(expected);
			const resource = createConfigResource(mockClient);
			const result = await resource.list();
			expect(result).toEqual(expected);
			expect(execute).toHaveBeenCalledWith(["config", "list"]);
		});
	});

	describe("path", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ path: "/home/user/.agnetd/config.toml" });
			const resource = createConfigResource(dualClient);
			const result = await resource.path();
			expect(http.get).toHaveBeenCalledWith("/api/v1/config/path");
			expect(result).toBe("/home/user/.agnetd/config.toml");
		});

		it("should call execute with config path and extract 'Config path' via CLI", async () => {
			execute.mockResolvedValue({
				"Config path": "/home/user/.agnetd/config.toml",
			});
			const resource = createConfigResource(mockClient);
			const result = await resource.path();
			expect(result).toBe("/home/user/.agnetd/config.toml");
			expect(execute).toHaveBeenCalledWith(["config", "path"]);
		});

		it("should return empty string if Config path not in response via CLI", async () => {
			execute.mockResolvedValue({});
			const resource = createConfigResource(mockClient);
			const result = await resource.path();
			expect(result).toBe("");
		});
	});
});
