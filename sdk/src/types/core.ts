// @neunode/sdk — Core types mirroring neunode-core/src/types.rs + kind.rs

// ─── Brand Helper ────────────────────────────────────────────────────────────────

/** Nominal type branding helper. Creates a distinct type from a primitive without runtime overhead. */
export type Brand<T, B extends string> = T & { readonly __brand: B };

// ─── Branded Primitive Types ─────────────────────────────────────────────────────
// Rust newtype wrappers that serialize as plain strings or numbers in JSON.

/** Decentralized Identifier string (did:key, did:ethr, or did:neunode). */
export type Did = Brand<string, 'Did'>;
/** Content-addressed identifier (IPFS multihash, e.g. "Qm..." or "bafy..."). */
export type CID = Brand<string, 'CID'>;
/** libp2p Peer ID string. */
export type PeerId = Brand<string, 'PeerId'>;
/** Unique bounty identifier. */
export type BountyId = Brand<string, 'BountyId'>;
/** Content-hashed feed event identifier. */
export type EventId = Brand<string, 'EventId'>;
/** Registered model identifier. */
export type ModelId = Brand<string, 'ModelId'>;
/** Training job identifier. */
export type JobId = Brand<string, 'JobId'>;
/** SHA-256 hash digest as a hex string. */
export type Hash256 = Brand<string, 'Hash256'>;
/** Ed25519 or secp256k1 signature as a base58/base64 string. */
export type Signature = Brand<string, 'Signature'>;

// ─── Numeric Aliases ──────────────────────────────────────────────────────────────

/** Millisecond-precision unix timestamp (u64 in Rust). */
export type Timestamp = Brand<number, 'Timestamp'>;

/** Monotonically increasing sequence number per-agent (u64 in Rust). */
export type Sequence = Brand<number, 'Sequence'>;

// ─── Token Amount ─────────────────────────────────────────────────────────────────
// Rust: TokenAmount(u64) — too large for JS number, use bigint.

export type TokenAmount = Brand<bigint, 'TokenAmount'>;

// ─── Token Type ───────────────────────────────────────────────────────────────────
// Rust: enum TokenType { Compute, Train, Bandwidth, Storage }

/** Resource token types: nCompute, nTrain, nBandwidth, nStorage. */
export const TokenType = {
  Compute: 'Compute',
  Train: 'Train',
  Bandwidth: 'Bandwidth',
  Storage: 'Storage',
} as const;

export type TokenType = (typeof TokenType)[keyof typeof TokenType];

// ─── Agent Lifecycle ──────────────────────────────────────────────────────────────
// Rust: enum AgentLifecycle { Created, Active, Idle, Zombie, Dead }

/** Agent lifecycle stages in the Neunode network. */
export const AgentLifecycle = {
  Created: 'Created',
  Active: 'Active',
  Idle: 'Idle',
  Zombie: 'Zombie',
  Dead: 'Dead',
} as const;

export type AgentLifecycle = (typeof AgentLifecycle)[keyof typeof AgentLifecycle];

// ─── Bounty State ─────────────────────────────────────────────────────────────────
// Rust: enum BountyState { Open, Claimed, Submitted, UnderReview, Revision, Accepted,
//        Rejected, Disputed, Paid, Expired, Cancelled }

/** Bounty lifecycle states from creation through payout or cancellation. */
export const BountyState = {
  Open: 'Open',
  Claimed: 'Claimed',
  Submitted: 'Submitted',
  UnderReview: 'UnderReview',
  Revision: 'Revision',
  Accepted: 'Accepted',
  Rejected: 'Rejected',
  Disputed: 'Disputed',
  Paid: 'Paid',
  Expired: 'Expired',
  Cancelled: 'Cancelled',
} as const;

export type BountyState = (typeof BountyState)[keyof typeof BountyState];

// ─── Activity Level ───────────────────────────────────────────────────────────────
// Rust: enum ActivityLevel { Active, Moderate, Low, Inactive, Dead }

/** Agent activity levels used to determine token decay rates. */
export const ActivityLevel = {
  Active: 'Active',
  Moderate: 'Moderate',
  Low: 'Low',
  Inactive: 'Inactive',
  Dead: 'Dead',
} as const;

export type ActivityLevel = (typeof ActivityLevel)[keyof typeof ActivityLevel];

// ─── Kind Taxonomy ────────────────────────────────────────────────────────────────
// Rust: enum Kind — 27 variants, #[repr(u16)], serde serializes as string name.
// Source: neunode-core/src/kind.rs

/** Feed event kind taxonomy: 27 variants across System, Bounty, Training, Attestation, Inference, and Governance. */
export const Kind = {
  // System (0–5)
  AgentAnnounce: 'AgentAnnounce',
  AgentUpdate: 'AgentUpdate',
  Ping: 'Ping',
  Pong: 'Pong',
  Error: 'Error',
  Metadata: 'Metadata',
  // Bounty (1000–1102)
  BountyPost: 'BountyPost',
  BountyClaim: 'BountyClaim',
  BountySubmit: 'BountySubmit',
  BountyReview: 'BountyReview',
  BountyAccept: 'BountyAccept',
  BountyReject: 'BountyReject',
  BountyDispute: 'BountyDispute',
  BountyResolve: 'BountyResolve',
  BountyPay: 'BountyPay',
  BountyCancel: 'BountyCancel',
  BountyExpire: 'BountyExpire',
  // Training (2000–2020)
  TrainingStart: 'TrainingStart',
  TrainingProgress: 'TrainingProgress',
  TrainingComplete: 'TrainingComplete',
  TrainingFailed: 'TrainingFailed',
  // Attestation (3000–3010)
  Attestation: 'Attestation',
  AttestationRevoke: 'AttestationRevoke',
  // Inference (4000–4010)
  InferenceRequest: 'InferenceRequest',
  InferenceResult: 'InferenceResult',
  // Governance (5000–5010)
  GovernanceProposal: 'GovernanceProposal',
  GovernanceVote: 'GovernanceVote',
} as const;

export type Kind = (typeof Kind)[keyof typeof Kind];

// ─── Kind Category ────────────────────────────────────────────────────────────────
// Rust: enum KindCategory { System, Bounty, Training, Attestation, Inference,
//        Governance, Custom, Unknown }

/** Broad category a Kind variant belongs to. */
export const KindCategory = {
  System: 'System',
  Bounty: 'Bounty',
  Training: 'Training',
  Attestation: 'Attestation',
  Inference: 'Inference',
  Governance: 'Governance',
  Custom: 'Custom',
  Unknown: 'Unknown',
} as const;

export type KindCategory = (typeof KindCategory)[keyof typeof KindCategory];
