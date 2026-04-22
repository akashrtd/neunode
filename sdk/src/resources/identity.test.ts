import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import { createIdentityResource } from "./identity.js";

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

describe("createIdentityResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if cli transport is missing", () => {
		const noCliClient = { ...mockClient, cli: undefined };
		expect(() => createIdentityResource(noCliClient)).toThrow(
			"CLI transport required",
		);
	});

	describe("create", () => {
		it("should call execute with correct args for name only", async () => {
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

		it("should pass --method when provided", async () => {
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

		it("should pass --output-dir when provided", async () => {
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
	});

	describe("show", () => {
		it("should call execute with identity show (no did)", async () => {
			execute.mockResolvedValue({ did: "did:neunode:abc" });
			const resource = createIdentityResource(mockClient);
			await resource.show();
			expect(execute).toHaveBeenCalledWith(["identity", "show"]);
		});

		it("should pass --did when provided", async () => {
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
		it("should call execute with identity list", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createIdentityResource(mockClient);
			await resource.list();
			expect(execute).toHaveBeenCalledWith(["identity", "list"]);
		});
	});

	describe("export", () => {
		it("should call execute with --file only", async () => {
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
	});
});
