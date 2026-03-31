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
}

export function createFeedResource(client: NeunodeClient): FeedResource {
  const cli = client.cli;
  if (!cli) throw new Error("CLI transport required for feed operations");

  return {
    async post(params: FeedPostParams): Promise<FeedPostResult> {
      const args = ["feed", "post", "--kind", String(params.kind), "--content", params.content];
      if (params.tags) {
        for (const tag of params.tags) {
          args.push("--tags", tag);
        }
      }
      return cli.execute<FeedPostResult>(args);
    },

    async list(params?: FeedListParams): Promise<FeedListItem[]> {
      const args = ["feed", "list"];
      if (params?.kind) args.push("--kind", String(params.kind));
      if (params?.author) args.push("--author", params.author);
      if (params?.limit) args.push("--limit", String(params.limit));
      return cli.execute<FeedListItem[]>(args);
    },

    async show(eventId: string): Promise<FeedShowResult> {
      return cli.execute<FeedShowResult>(["feed", "show", "--event-id", eventId]);
    },

    async subscribe(kind?: number): Promise<FeedSubscribeResult> {
      const args = ["feed", "subscribe"];
      if (kind) args.push("--kind", String(kind));
      return cli.execute<FeedSubscribeResult>(args);
    },
  };
}
