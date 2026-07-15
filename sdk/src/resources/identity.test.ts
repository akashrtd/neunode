import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createIdentityResource } from "./identity.js";

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
	const httpPut = vi.fn();
	const httpDelete = vi.fn();
	const httpTransport = {
		get: httpGet,
		post: httpPost,
		put: httpPut,
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
		lifecycle: {} as never,
		lineage: {} as never,
		verification: {} as never,
		extend: vi.fn(),
	};
}

describe("createIdentityResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both cli and http transports are missing", async () => {
		const noTransports = { ...mockClient, cli: undefined, http: undefined };
		const resource = createIdentityResource(noTransports);
		await expect(resource.list()).rejects.toThrow(
			"HTTP or CLI transport required",
		);
	});

	describe("create", () => {
		it("should use HTTP transport when available", async () => {
			const expected = { DID: "did:neunode:http", Name: "test" };
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue(expected);
			const resource = createIdentityResource(dualClient);
			const result = await resource.create({ name: "test" });
			expect(http.post).toHaveBeenCalledWith(
				"/api/v1/identity/create",
				expect.objectContaining({ name: "test" }),
			);
			expect(result).toEqual(expected);
		});

		it("should fall back to CLI when HTTP is not available", async () => {
			execute.mockResolvedValue({ DID: "did:neunode:abc", Name: "test" });
			const resource = createIdentityResource(mockClient);
			await resource.create({ name: "test" });
			expect(execute).toHaveBeenCalledWith([
				"identity",
				"create",
				"--name",
				"test",
			]);
		});

		it("should pass --method when provided (CLI fallback)", async () => {
			execute.mockResolvedValue({ DID: "did:neunode:abc" });
			const resource = createIdentityResource(mockClient);
			await resource.create({ name: "test", method: "key" });
			expect(execute).toHaveBeenCalledWith([
				"identity",
				"create",
				"--name",
				"test",
				"--method",
				"key",
			]);
		});

		it("should pass --output-dir when provided (CLI fallback)", async () => {
			execute.mockResolvedValue({ DID: "did:neunode:abc" });
			const resource = createIdentityResource(mockClient);
			await resource.create({ name: "test", outputDir: "/tmp/keys" });
			expect(execute).toHaveBeenCalledWith([
				"identity",
				"create",
				"--name",
				"test",
				"--output-dir",
				"/tmp/keys",
			]);
		});

		it("should return IdentityCreateResult", async () => {
			const expected = {
				DID: "did:neunode:abc",
				"DID (key)": "did:key:xyz",
				"Peer ID": "12D3KooW...",
				Ethereum: "0xabc",
				Name: "test",
				Method: "key",
				Directory: "/tmp/keys",
				"Card CID": "QmX7b...",
			};
			execute.mockResolvedValue(expected);
			const resource = createIdentityResource(mockClient);
			const result = await resource.create({ name: "test" });
			expect(result).toEqual(expected);
		});

		it("should throw when neither transport is available", async () => {
			const noTransports = { ...mockClient, cli: undefined, http: undefined };
			const resource = createIdentityResource(noTransports);
			await expect(resource.create({ name: "test" })).rejects.toThrow(
				"HTTP or CLI transport required",
			);
		});
	});

	describe("show", () => {
		it("should use HTTP transport with query params", async () => {
			const expected = { did: "did:neunode:http" };
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue(expected);
			const resource = createIdentityResource(dualClient);
			const result = await resource.show("did:neunode:http");
			expect(http.get).toHaveBeenCalledWith(
				"/api/v1/identity?did=did%3Aneunode%3Ahttp",
			);
			expect(result).toEqual(expected);
		});

		it("should use HTTP without query params when no did", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ did: "default" });
			const resource = createIdentityResource(dualClient);
			await resource.show();
			expect(http.get).toHaveBeenCalledWith("/api/v1/identity");
		});

		it("should call execute with identity show (no did) via CLI", async () => {
			execute.mockResolvedValue({ did: "did:neunode:abc" });
			const resource = createIdentityResource(mockClient);
			await resource.show();
			expect(execute).toHaveBeenCalledWith(["identity", "show"]);
		});

		it("should pass --did when provided via CLI", async () => {
			execute.mockResolvedValue({ did: "did:neunode:abc" });
			const resource = createIdentityResource(mockClient);
			await resource.show("did:neunode:abc");
			expect(execute).toHaveBeenCalledWith([
				"identity",
				"show",
				"--did",
				"did:neunode:abc",
			]);
		});
	});

	describe("list", () => {
		it("should use HTTP transport", async () => {
			const expected = { data: [{ DID: "did:neunode:1", Status: "Active" }] };
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue(expected);
			const resource = createIdentityResource(dualClient);
			const result = await resource.list();
			expect(http.get).toHaveBeenCalledWith("/api/v1/identity/list");
			expect(result).toEqual(expected);
		});

		it("should call execute with identity list via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createIdentityResource(mockClient);
			await resource.list();
			expect(execute).toHaveBeenCalledWith(["identity", "list"]);
		});
	});

	describe("export", () => {
		it("should always use CLI transport (file operation)", async () => {
			execute.mockResolvedValue({
				did: "did:neunode:abc",
				exported_at: "2026-01-01",
			});
			const resource = createIdentityResource(mockClient);
			await resource.export({ file: "/tmp/export.json" });
			expect(execute).toHaveBeenCalledWith([
				"identity",
				"export",
				"--file",
				"/tmp/export.json",
			]);
		});

		it("should pass --did when provided", async () => {
			execute.mockResolvedValue({ did: "did:neunode:abc" });
			const resource = createIdentityResource(mockClient);
			await resource.export({
				file: "/tmp/export.json",
				did: "did:neunode:abc",
			});
			expect(execute).toHaveBeenCalledWith([
				"identity",
				"export",
				"--file",
				"/tmp/export.json",
				"--did",
				"did:neunode:abc",
			]);
		});

		it("should throw if CLI is not available", async () => {
			const httpOnly = makeMockClient({ withHttp: true, withCli: false });
			const resource = createIdentityResource(httpOnly);
			await expect(
				resource.export({ file: "/tmp/export.json" }),
			).rejects.toThrow("CLI transport required");
		});
	});
});
