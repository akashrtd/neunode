import type { NeunodeClient } from "../client/client.js";

export interface FeedPostParams {
	kind: number;
	content: string;
	tags?: string[];
}

export interface FeedPostResult {
	readonly event_id: string;
	readonly sequence: number;
	readonly kind: number;
	readonly topic: string;
}

export interface FeedListParams {
	kind?: number;
	author?: string;
	limit?: number;
}

export interface FeedListItem {
	readonly sequence: number;
	readonly kind: number;
	readonly timestamp: number;
	readonly author_did: string;
	readonly content: string;
	readonly signature: string;
}

export type FeedShowResult = FeedListItem;

/** A single event from the WebSocket feed stream. */
export interface FeedStreamEvent {
	readonly kind: number;
	readonly author_did: string;
	readonly author_short: string;
	readonly kind_label: string;
	readonly preview: string;
	readonly time_ago: string;
}

/** Feed operations for posting, listing, and subscribing to events. */
export interface FeedResource {
	/** Post a new event to the feed. */
	post(params: FeedPostParams): Promise<FeedPostResult>;
	/** List feed events, optionally filtered by kind, author, or limit. */
	list(params?: FeedListParams): Promise<FeedListItem[]>;
	/** Show the full content and signature of a single event. */
	show(eventId: string): Promise<FeedShowResult>;
	/** Stream feed events in real-time via WebSocket. */
	stream(callback: (event: FeedStreamEvent) => void, kind?: number): () => void;
}

export function createFeedResource(client: NeunodeClient): FeedResource {
	const http = () => {
		if (!client.http)
			throw new Error("HTTP transport required for feed operations");
		return client.http;
	};
	return {
		async post(params: FeedPostParams): Promise<FeedPostResult> {
			return http().post<FeedPostResult>("/api/v1/feed", params);
		},

		async list(params?: FeedListParams): Promise<FeedListItem[]> {
			const qs = new URLSearchParams();
			if (params?.kind !== undefined) qs.set("kind", String(params.kind));
			if (params?.author) qs.set("author", params.author);
			if (params?.limit !== undefined) qs.set("limit", String(params.limit));
			const query = qs.toString();
			return http().get<FeedListItem[]>(
				query ? `/api/v1/feed?${query}` : "/api/v1/feed",
			);
		},

		async show(eventId: string): Promise<FeedShowResult> {
			return http().get<FeedShowResult>(
				`/api/v1/feed/${encodeURIComponent(eventId)}`,
			);
		},

		stream(
			callback: (event: FeedStreamEvent) => void,
			kind?: number,
		): () => void {
			// Extract host from HTTP baseUrl for WebSocket connection
			const base = http().getBaseUrl();
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
					if (kind === undefined || parsed.kind === kind) callback(parsed);
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
