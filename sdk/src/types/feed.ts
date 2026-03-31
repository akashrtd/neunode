// @neunode/sdk — Feed types mirroring neunode-feed (event, filter, schema)

import type { Did, EventId, Kind, Sequence, Timestamp, Hash256, Signature } from './core.js';

export interface EventTag {
  readonly key: string;
  readonly value: string;
}

export interface EventRef {
  readonly event_id: EventId;
  readonly author: Did;
}

/** An immutable, signed feed event in the agent's sigchain. */
export interface FeedEvent {
  readonly id: EventId;
  readonly kind: Kind;
  readonly author: Did;
  readonly sequence: Sequence;
  readonly timestamp: Timestamp;
  readonly prev_hash: Hash256;
  readonly content: string;
  readonly tags: readonly EventTag[];
  readonly refs: readonly EventRef[];
  readonly signature?: Signature;
}

/** Nostr-like filter for querying feed events by kind, author, time range, or tags. */
export interface FeedFilter {
  readonly kinds?: readonly Kind[];
  readonly authors?: readonly Did[];
  readonly since?: Timestamp;
  readonly until?: Timestamp;
  readonly limit?: number;
  readonly tags?: readonly (readonly [string, string])[];
}

export interface BountyPost {
  readonly title: string;
  readonly description: string;
  readonly reward_amount: number;
  readonly reward_token: string;
  readonly deadline: number;
  readonly required_capabilities: readonly string[];
}

export interface BountyClaim {
  readonly bounty_id: string;
  readonly stake_amount: number;
  readonly stake_token: string;
  readonly proposer_did: Did;
}

export interface FeedAttestation {
  readonly target_did: Did;
  readonly claim: string;
  readonly evidence: readonly string[];
  readonly score: number;
}
