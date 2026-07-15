// @neunode/sdk — Core types mirroring neunode-core/src/types.rs + kind.rs

// ─── Brand Helper ────────────────────────────────────────────────────────────────

/** Nominal type branding helper. Creates a distinct type from a primitive without runtime overhead. */
export type Brand<T, B extends string> = T & { readonly __brand: B };

// ─── Branded Primitive Types ─────────────────────────────────────────────────────
// Rust newtype wrappers that serialize as plain strings or numbers in JSON.

/** Decentralized Identifier string (did:key, did:ethr, or did:neunode). */
export type Did = Brand<string, "Did">;
/** Content-addressed identifier (IPFS multihash, e.g. "Qm..." or "bafy..."). */
export type CID = Brand<string, "CID">;
/** libp2p Peer ID string. */
export type PeerId = Brand<string, "PeerId">;
/** Unique bounty identifier. */
export type BountyId = Brand<string, "BountyId">;
/** Content-hashed feed event identifier. */
export type EventId = Brand<string, "EventId">;
/** Registered model identifier. */
export type ModelId = Brand<string, "ModelId">;
/** Training job identifier. */
export type JobId = Brand<string, "JobId">;
/** SHA-256 hash digest as a hex string. */
export type Hash256 = Brand<string, "Hash256">;
/** Ed25519 or secp256k1 signature as a base58/base64 string. */
export type Signature = Brand<string, "Signature">;

// ─── Numeric Aliases ──────────────────────────────────────────────────────────────

/** Unix timestamp in seconds (u64 in Rust). */
export type Timestamp = Brand<number, "Timestamp">;

/** Monotonically increasing sequence number per-agent (u64 in Rust). */
export type Sequence = Brand<number, "Sequence">;

// ─── Token Amount ─────────────────────────────────────────────────────────────────
// Rust: TokenAmount(u128) — too large for JS number, use bigint.

export type TokenAmount = Brand<bigint, "TokenAmount">;

// ─── Token Type ───────────────────────────────────────────────────────────────────
// Rust: enum TokenType { Compute, Train, Bandwidth, Storage }

/** Resource token types: nCompute, nTrain, nBandwidth, nStorage. */
export const TokenType = {
	Compute: "Compute",
	Train: "Train",
	Bandwidth: "Bandwidth",
	Storage: "Storage",
} as const;

export type TokenType = (typeof TokenType)[keyof typeof TokenType];

// ─── Agent Lifecycle ──────────────────────────────────────────────────────────────
// Rust: enum AgentLifecycle { Created, Active, Idle, Zombie, Dead }

/** Agent lifecycle stages in the Neunode network. */
export const AgentLifecycle = {
	Created: "Created",
	Active: "Active",
	Idle: "Idle",
	Zombie: "Zombie",
	Dead: "Dead",
} as const;

export type AgentLifecycle =
	(typeof AgentLifecycle)[keyof typeof AgentLifecycle];

// ─── Bounty State ─────────────────────────────────────────────────────────────────
// Rust: enum BountyState { Open, Claimed, Submitted, UnderReview, Revision, Accepted,
//        Rejected, Disputed, Paid, Expired, Cancelled }

/** Bounty lifecycle states from creation through payout or cancellation. */
export const BountyState = {
	Open: "Open",
	Claimed: "Claimed",
	Submitted: "Submitted",
	UnderReview: "UnderReview",
	Revision: "Revision",
	Accepted: "Accepted",
	Rejected: "Rejected",
	Disputed: "Disputed",
	Paid: "Paid",
	Expired: "Expired",
	Cancelled: "Cancelled",
} as const;

export type BountyState = (typeof BountyState)[keyof typeof BountyState];

// ─── Activity Level ───────────────────────────────────────────────────────────────
// Rust: enum ActivityLevel { Active, Moderate, Low, Inactive, Dead }

/** Agent activity levels used to determine token decay rates. */
export const ActivityLevel = {
	Active: "Active",
	Moderate: "Moderate",
	Low: "Low",
	Inactive: "Inactive",
	Dead: "Dead",
} as const;

export type ActivityLevel = (typeof ActivityLevel)[keyof typeof ActivityLevel];

export { Kind, KindCategory } from "./protocol.generated.js";
