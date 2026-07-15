import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createFeedResource } from "./feed.js";

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
		getBaseUrl: () => "http://127.0.0.1:41000",
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

describe("createFeedResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both transports are missing", async () => {
		const resource = createFeedResource({
			...mockClient,
			cli: undefined,
			http: undefined,
		});
		await expect(resource.list()).rejects.toThrow(
			"HTTP or CLI transport required",
		);
	});

	describe("post", () => {
		it("should use HTTP transport when available", async () => {
			const expected = { "Event ID": "evt_http" };
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue(expected);
			const resource = createFeedResource(dualClient);
			const result = await resource.post({ kind: 1000, content: "test" });
			expect(http.post).toHaveBeenCalledWith("/api/v1/feed", {
				kind: 1000,
				content: "test",
			});
			expect(result).toEqual(expected);
		});

		it("should call execute with feed post --kind and --content via CLI", async () => {
			execute.mockResolvedValue({ "Event ID": "evt_123" });
			const resource = createFeedResource(mockClient);
			await resource.post({ kind: 1000, content: '{"title":"hello"}' });
			expect(execute).toHaveBeenCalledWith([
				"feed",
				"post",
				"--kind",
				"1000",
				"--content",
				'{"title":"hello"}',
			]);
		});

		it("should pass --tags for each tag via CLI", async () => {
			execute.mockResolvedValue({ "Event ID": "evt_123" });
			const resource = createFeedResource(mockClient);
			await resource.post({
				kind: 1000,
				content: "test",
				tags: ["ai", "training"],
			});
			expect(execute).toHaveBeenCalledWith([
				"feed",
				"post",
				"--kind",
				"1000",
				"--content",
				"test",
				"--tags",
				"ai",
				"--tags",
				"training",
			]);
		});

		it("should not pass --tags when tags is undefined", async () => {
			execute.mockResolvedValue({ "Event ID": "evt_123" });
			const resource = createFeedResource(mockClient);
			await resource.post({ kind: 2000, content: "no tags" });
			const args = execute.mock.calls[0]?.[0] as string[] | undefined;
			expect(args).toBeDefined();
			expect(args).not.toContain("--tags");
		});
	});

	describe("list", () => {
		it("should use HTTP transport with query params", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue([]);
			const resource = createFeedResource(dualClient);
			await resource.list({ kind: 1000, author: "did:neunode:abc", limit: 10 });
			expect(http.get).toHaveBeenCalled();
			const callUrl = http.get.mock.calls[0]?.[0] as string;
			expect(callUrl).toContain("/api/v1/feed?");
			expect(callUrl).toContain("kind=1000");
			expect(callUrl).toContain("author=did%3Aneunode%3Aabc");
			expect(callUrl).toContain("limit=10");
		});

		it("should call execute with feed list (no params) via CLI", async () => {
			execute.mockResolvedValue([]);
			const resource = createFeedResource(mockClient);
			await resource.list();
			expect(execute).toHaveBeenCalledWith(["feed", "list"]);
		});

		it("should pass --kind, --author, --limit when provided via CLI", async () => {
			execute.mockResolvedValue([]);
			const resource = createFeedResource(mockClient);
			await resource.list({ kind: 1000, author: "did:neunode:abc", limit: 10 });
			expect(execute).toHaveBeenCalledWith([
				"feed",
				"list",
				"--kind",
				"1000",
				"--author",
				"did:neunode:abc",
				"--limit",
				"10",
			]);
		});

		it("should pass only provided optional params via CLI", async () => {
			execute.mockResolvedValue([]);
			const resource = createFeedResource(mockClient);
			await resource.list({ limit: 5 });
			expect(execute).toHaveBeenCalledWith(["feed", "list", "--limit", "5"]);
		});
	});

	describe("show", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ Sequence: "1", Content: "hello" });
			const resource = createFeedResource(dualClient);
			await resource.show("evt_abc123");
			expect(http.get).toHaveBeenCalledWith("/api/v1/feed/evt_abc123");
		});

		it("should call execute with feed show --event-id via CLI", async () => {
			execute.mockResolvedValue({ Sequence: "1", Content: "hello" });
			const resource = createFeedResource(mockClient);
			await resource.show("evt_abc123");
			expect(execute).toHaveBeenCalledWith([
				"feed",
				"show",
				"--event-id",
				"evt_abc123",
			]);
		});
	});

	describe("subscribe", () => {
		it("should call execute with feed subscribe (no kind) via CLI", async () => {
			execute.mockResolvedValue({
				topic: "feed",
				status: "active",
				streaming: true,
			});
			const resource = createFeedResource(mockClient);
			await resource.subscribe();
			expect(execute).toHaveBeenCalledWith(["feed", "subscribe"]);
		});

		it("should pass --kind when provided via CLI", async () => {
			execute.mockResolvedValue({
				topic: "bounty",
				status: "active",
				streaming: true,
			});
			const resource = createFeedResource(mockClient);
			await resource.subscribe(1000);
			expect(execute).toHaveBeenCalledWith([
				"feed",
				"subscribe",
				"--kind",
				"1000",
			]);
		});
	});

	describe("stream", () => {
		it("should throw if HTTP transport is not configured", () => {
			const resource = createFeedResource(mockClient);
			expect(() => resource.stream(() => {})).toThrow(
				"HTTP transport required for feed streaming",
			);
		});

		it("should return an unsubscribe function when HTTP is available", () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const resource = createFeedResource(dualClient);
			const unsubscribe = resource.stream(() => {});
			expect(typeof unsubscribe).toBe("function");
			unsubscribe();
		});
	});
});
