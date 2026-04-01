import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import { createFeedResource } from "./feed.js";

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

	it("should throw if cli transport is missing", () => {
		expect(() => createFeedResource({ ...mockClient, cli: undefined })).toThrow(
			"CLI transport required",
		);
	});

	describe("post", () => {
		it("should call execute with feed post --kind and --content", async () => {
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

		it("should pass --tags for each tag", async () => {
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
		it("should call execute with feed list (no params)", async () => {
			execute.mockResolvedValue([]);
			const resource = createFeedResource(mockClient);
			await resource.list();
			expect(execute).toHaveBeenCalledWith(["feed", "list"]);
		});

		it("should pass --kind, --author, --limit when provided", async () => {
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

		it("should pass only provided optional params", async () => {
			execute.mockResolvedValue([]);
			const resource = createFeedResource(mockClient);
			await resource.list({ limit: 5 });
			expect(execute).toHaveBeenCalledWith(["feed", "list", "--limit", "5"]);
		});
	});

	describe("show", () => {
		it("should call execute with feed show --event-id", async () => {
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
		it("should call execute with feed subscribe (no kind)", async () => {
			execute.mockResolvedValue({
				topic: "feed",
				status: "active",
				streaming: true,
			});
			const resource = createFeedResource(mockClient);
			await resource.subscribe();
			expect(execute).toHaveBeenCalledWith(["feed", "subscribe"]);
		});

		it("should pass --kind when provided", async () => {
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
});
