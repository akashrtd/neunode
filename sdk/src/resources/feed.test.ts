import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createFeedResource } from "./feed.js";

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
		getBaseUrl: () => "http://127.0.0.1:41000",
	} as unknown as HttpTransport;
	return {
		client: {
			http,
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

describe("createFeedResource", () => {
	let client: NeunodeClient;
	let get: ReturnType<typeof vi.fn>;
	let post: ReturnType<typeof vi.fn>;

	beforeEach(() => ({ client, get, post } = makeClient()));

	it("posts feed events over HTTP", async () => {
		post.mockResolvedValue({ event_id: "evt_1", sequence: 1 });
		await createFeedResource(client).post({
			kind: 1000,
			content: "test",
			tags: ["ai"],
		});
		expect(post).toHaveBeenCalledWith("/api/v1/feed", {
			kind: 1000,
			content: "test",
			tags: ["ai"],
		});
	});

	it("lists feed events with encoded filters", async () => {
		get.mockResolvedValue([]);
		await createFeedResource(client).list({
			kind: 0,
			author: "did:neunode:abc",
			limit: 0,
		});
		expect(get).toHaveBeenCalledWith(
			"/api/v1/feed?kind=0&author=did%3Aneunode%3Aabc&limit=0",
		);
	});

	it("shows events by their canonical event ID", async () => {
		get.mockResolvedValue({ sequence: 1, content: "hello" });
		await createFeedResource(client).show("evt:with/slash");
		expect(get).toHaveBeenCalledWith("/api/v1/feed/evt%3Awith%2Fslash");
	});

	it("opens a filtered WebSocket and returns a cancellation function", () => {
		const close = vi.fn();
		const sockets: Array<{
			url: string;
			close: () => void;
			onmessage?: (event: MessageEvent) => void;
		}> = [];
		class FakeWebSocket {
			onmessage?: (event: MessageEvent) => void;
			constructor(readonly url: string) {
				sockets.push(this);
			}
			close = close;
		}
		vi.stubGlobal("WebSocket", FakeWebSocket);
		const callback = vi.fn();
		const cancel = createFeedResource(client).stream(callback, 1000);
		expect(sockets[0]?.url).toBe("ws://127.0.0.1:41000/ws/feed?kind=1000");
		sockets[0]?.onmessage?.({
			data: JSON.stringify({ kind: 1000, preview: "hello" }),
		} as MessageEvent);
		expect(callback).toHaveBeenCalledWith({ kind: 1000, preview: "hello" });
		cancel();
		expect(close).toHaveBeenCalled();
		vi.unstubAllGlobals();
	});
});
