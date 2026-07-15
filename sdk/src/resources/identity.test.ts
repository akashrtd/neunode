import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createIdentityResource } from "./identity.js";

function makeClient(): {
	client: NeunodeClient;
	get: ReturnType<typeof vi.fn>;
	post: ReturnType<typeof vi.fn>;
} {
	const get = vi.fn();
	const post = vi.fn();
	const http = {
		get,
		post,
		put: vi.fn(),
		delete: vi.fn(),
	} as unknown as HttpTransport;
	return {
		client: {
			http,
			cli: undefined,
			viem: undefined,
			transportMode: "http",
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
		},
		get,
		post,
	};
}

describe("createIdentityResource", () => {
	let client: NeunodeClient;
	let get: ReturnType<typeof vi.fn>;
	let post: ReturnType<typeof vi.fn>;

	beforeEach(() => ({ client, get, post } = makeClient()));

	it("requires HTTP transport", async () => {
		const resource = createIdentityResource({ ...client, http: undefined });
		await expect(resource.list()).rejects.toThrow("HTTP transport required");
	});

	it("creates identities over HTTP", async () => {
		const expected = {
			identity: { did: "did:neunode:http" },
			card_cid: "bafycard",
		};
		post.mockResolvedValue(expected);
		await expect(
			createIdentityResource(client).create({ name: "test", method: "key" }),
		).resolves.toEqual(expected);
		expect(post).toHaveBeenCalledWith("/api/v1/identity/create", {
			name: "test",
			method: "key",
		});
	});

	it("shows the active or selected identity", async () => {
		get.mockResolvedValue({ did: "did:neunode:http" });
		const resource = createIdentityResource(client);
		await resource.show();
		expect(get).toHaveBeenLastCalledWith("/api/v1/identity");
		await resource.show("did:neunode:http");
		expect(get).toHaveBeenLastCalledWith(
			"/api/v1/identity?did=did%3Aneunode%3Ahttp",
		);
	});

	it("lists identities over HTTP", async () => {
		get.mockResolvedValue([{ did: "did:neunode:1", status: "stored" }]);
		await createIdentityResource(client).list();
		expect(get).toHaveBeenCalledWith("/api/v1/identity/list");
	});

	it("exports the active or selected identity as JSON", async () => {
		get.mockResolvedValue({ did: "did:neunode:abc" });
		const resource = createIdentityResource(client);
		await resource.export();
		expect(get).toHaveBeenLastCalledWith("/api/v1/identity/export");
		await resource.export({ did: "did:neunode:abc" });
		expect(get).toHaveBeenLastCalledWith(
			"/api/v1/identity/export?did=did%3Aneunode%3Aabc",
		);
	});
});
