import type { NeunodeClient } from "../client/client.js";

export interface FeedPostParams {
	kind: number;
	content: string;
	tags?: string[];
}

export interface FeedPostResult {
	"Event ID": string;
	Kind: string;
	Author: string;
	Sequence: string;
	Topic: string;
	Schema: string;
}

export interface FeedListParams {
	kind?: number;
	author?: string;
	limit?: number;
}

export interface FeedListItem {
	Seq: string;
	Kind: string;
	Timestamp: string;
	Author: string;
}

export interface FeedShowResult {
	Sequence: string;
	Kind: string;
	Timestamp: string;
	Author: string;
	Content: string;
	Signature: string;
}

export interface FeedSubscribeResult {
	topic: string;
	status: string;
	streaming: boolean;
}

/** A single event from the WebSocket feed stream. */
export interface FeedStreamEvent {
	kind: number;
	author_did: string;
	content: string;
	timestamp: number;
}

/** Feed operations for posting, listing, and subscribing to events. */
export interface FeedResource {
	/** Post a new event to the feed. */
	post(params: FeedPostParams): Promise<FeedPostResult>;
	/** List feed events, optionally filtered by kind, author, or limit. */
	list(params?: FeedListParams): Promise<FeedListItem[]>;
	/** Show the full content and signature of a single event. */
	show(eventId: string): Promise<FeedShowResult>;
	/** Subscribe to feed events, optionally filtered by kind. */
	subscribe(kind?: number): Promise<FeedSubscribeResult>;
	/** Stream feed events in real-time via WebSocket. */
	stream(callback: (event: FeedStreamEvent) => void, kind?: number): () => void;
}

export function createFeedResource(client: NeunodeClient): FeedResource {
	return {
		async post(params: FeedPostParams): Promise<FeedPostResult> {
			if (client.http) {
				return client.http.post<FeedPostResult>("/api/v1/feed", params);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for feed operations");
			const args = [
				"feed",
				"post",
				"--kind",
				String(params.kind),
				"--content",
				params.content,
			];
			if (params.tags) {
				for (const tag of params.tags) {
					args.push("--tags", tag);
				}
			}
			return cli.execute<FeedPostResult>(args);
		},

		async list(params?: FeedListParams): Promise<FeedListItem[]> {
			if (client.http) {
				const qs = new URLSearchParams();
				if (params?.kind) qs.set("kind", String(params.kind));
				if (params?.author) qs.set("author", params.author);
				if (params?.limit) qs.set("limit", String(params.limit));
				const query = qs.toString();
				return client.http.get<FeedListItem[]>(
					query ? `/api/v1/feed?${query}` : "/api/v1/feed",
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for feed operations");
			const args = ["feed", "list"];
			if (params?.kind) args.push("--kind", String(params.kind));
			if (params?.author) args.push("--author", params.author);
			if (params?.limit) args.push("--limit", String(params.limit));
			return cli.execute<FeedListItem[]>(args);
		},

		async show(eventId: string): Promise<FeedShowResult> {
			if (client.http) {
				return client.http.get<FeedShowResult>(
					`/api/v1/feed/${encodeURIComponent(eventId)}`,
				);
			}
			const cli = client.cli;
			if (!cli)
				throw new Error("HTTP or CLI transport required for feed operations");
			return cli.execute<FeedShowResult>([
				"feed",
				"show",
				"--event-id",
				eventId,
			]);
		},

		async subscribe(kind?: number): Promise<FeedSubscribeResult> {
			// subscribe uses special streaming — CLI fallback for now
			const cli = client.cli;
			if (!cli)
				throw new Error(
					"CLI transport required for feed subscribe (streaming operation)",
				);
			const args = ["feed", "subscribe"];
			if (kind) args.push("--kind", String(kind));
			return cli.execute<FeedSubscribeResult>(args);
		},

		stream(
			callback: (event: FeedStreamEvent) => void,
			kind?: number,
		): () => void {
			if (!client.http) {
				throw new Error(
					"HTTP transport required for feed streaming. " +
						"Configure { http: { baseUrl: '...' } } to use stream().",
				);
			}
			// Extract host from HTTP baseUrl for WebSocket connection
			const base = client.http.getBaseUrl();
			const wsUrl = `${base.replace(/^http/, "ws")}/ws/feed`;
			const params = new URLSearchParams();
			if (kind !== undefined) params.set("kind", String(kind));
			const fullUrl = params.toString()
				? `${wsUrl}?${params.toString()}`
				: wsUrl;

			const socket = new WebSocket(fullUrl);
			socket.onmessage = (event: MessageEvent) => {
				try {
					const parsed = JSON.parse(event.data as string) as FeedStreamEvent;
					callback(parsed);
				} catch {
					// Ignore malformed messages
				}
			};
			return () => {
				socket.close();
			};
		},
	};
}
