import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createMeshResource } from "./mesh.js";

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

describe("createMeshResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both transports are missing", async () => {
		const resource = createMeshResource({
			...mockClient,
			cli: undefined,
			http: undefined,
		});
		await expect(resource.status()).rejects.toThrow(
			"HTTP or CLI transport required",
		);
	});

	describe("status", () => {
		it("should use HTTP transport when available", async () => {
			const expected = { Status: "Connected", "Peer ID": "12D3KooW..." };
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue(expected);
			const resource = createMeshResource(dualClient);
			const result = await resource.status();
			expect(http.get).toHaveBeenCalledWith("/api/v1/mesh/status");
			expect(result).toEqual(expected);
		});

		it("should call execute with mesh status via CLI", async () => {
			const expected = {
				Status: "Connected",
				"Peer ID": "12D3KooW...",
				Listeners: "2",
				"Connected Peers": "5",
				"Subscribed Topics": "3",
			};
			execute.mockResolvedValue(expected);
			const resource = createMeshResource(mockClient);
			const result = await resource.status();
			expect(result).toEqual(expected);
			expect(execute).toHaveBeenCalledWith(["mesh", "status"]);
		});
	});

	describe("peers", () => {
		it("should use HTTP transport with verbose param", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createMeshResource(dualClient);
			await resource.peers(true);
			expect(http.get).toHaveBeenCalledWith("/api/v1/mesh/peers?verbose=true");
		});

		it("should call execute with mesh peers (no verbose) via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createMeshResource(mockClient);
			await resource.peers();
			expect(execute).toHaveBeenCalledWith(["mesh", "peers"]);
		});

		it("should pass --verbose when verbose=true via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createMeshResource(mockClient);
			await resource.peers(true);
			expect(execute).toHaveBeenCalledWith(["mesh", "peers", "--verbose"]);
		});

		it("should not pass --verbose when verbose=false via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createMeshResource(mockClient);
			await resource.peers(false);
			expect(execute).toHaveBeenCalledWith(["mesh", "peers"]);
		});
	});

	describe("connect", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({ action: "connect", status: "ok" });
			const resource = createMeshResource(dualClient);
			await resource.connect("/ip4/1.2.3.4/tcp/4001/p2p/QmX7b");
			expect(http.post).toHaveBeenCalledWith("/api/v1/mesh/connect", {
				addr: "/ip4/1.2.3.4/tcp/4001/p2p/QmX7b",
			});
		});

		it("should call execute with mesh connect --addr via CLI", async () => {
			execute.mockResolvedValue({
				action: "connect",
				address: "/ip4/1.2.3.4/tcp/4001",
				status: "ok",
			});
			const resource = createMeshResource(mockClient);
			await resource.connect("/ip4/1.2.3.4/tcp/4001/p2p/QmX7b");
			expect(execute).toHaveBeenCalledWith([
				"mesh",
				"connect",
				"--addr",
				"/ip4/1.2.3.4/tcp/4001/p2p/QmX7b",
			]);
		});
	});

	describe("disconnect", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({ action: "disconnect", status: "ok" });
			const resource = createMeshResource(dualClient);
			await resource.disconnect("12D3KooW");
			expect(http.post).toHaveBeenCalledWith("/api/v1/mesh/disconnect", {
				peerId: "12D3KooW",
			});
		});

		it("should call execute with mesh disconnect --peer-id via CLI", async () => {
			execute.mockResolvedValue({
				action: "disconnect",
				peer_id: "12D3KooW",
				status: "ok",
			});
			const resource = createMeshResource(mockClient);
			await resource.disconnect("12D3KooW");
			expect(execute).toHaveBeenCalledWith([
				"mesh",
				"disconnect",
				"--peer-id",
				"12D3KooW",
			]);
		});
	});
});
