// @neunode/sdk — Phase 2 training types mirroring neunode-training crate
// (coordinator, worker, settlement, fault, aggregator, provider, gradient)

import type { Did, JobId, TokenAmount, Timestamp } from './core.js';

// ─── Worker Status ────────────────────────────────────────────────────────────────

/** Current state of a training worker in the distributed job. */
export const WorkerStatus = {
  Idle: 'idle',
  Training: 'training',
  Reporting: 'reporting',
  Failed: 'failed',
} as const;

export type WorkerStatus = (typeof WorkerStatus)[keyof typeof WorkerStatus];

// ─── Coordinator Status ───────────────────────────────────────────────────────────

/** Current state of a DiLoCo training coordinator. */
export const CoordinatorStatus = {
  Idle: 'idle',
  Collecting: 'collecting',
  Aggregating: 'aggregating',
  Distributing: 'distributing',
  Completed: 'completed',
  Failed: 'failed',
} as const;

export type CoordinatorStatus = (typeof CoordinatorStatus)[keyof typeof CoordinatorStatus];

// ─── Health State ──────────────────────────────────────────────────────────────────

/** Liveness state of a worker as determined by the heartbeat monitor. */
export const HealthState = {
  Healthy: 'healthy',
  Suspect: 'suspect',
  Dead: 'dead',
} as const;

export type HealthState = (typeof HealthState)[keyof typeof HealthState];

// ─── Settlement Status ─────────────────────────────────────────────────────────────

/** Payout status for a training job's escrowed funds. */
export const SettlementStatusValues = {
  Pending: 'pending',
  Partial: 'partial',
  Completed: 'completed',
  Refunded: 'refunded',
} as const;

export type SettlementStatus = (typeof SettlementStatusValues)[keyof typeof SettlementStatusValues];

// ─── Aggregation Mode ──────────────────────────────────────────────────────────────

/** How gradient aggregation handles stragglers: wait for all or proceed with subset. */
export const AggregationMode = {
  AllReduce: 'all_reduce',
  Partial: 'partial',
} as const;

export type AggregationMode = (typeof AggregationMode)[keyof typeof AggregationMode];

// ─── Provider Status ───────────────────────────────────────────────────────────────

/** Availability state of a GPU compute provider in the training marketplace. */
export const TrainingProviderStatus = {
  Available: 'available',
  Busy: 'busy',
  Offline: 'offline',
} as const;

export type TrainingProviderStatus = (typeof TrainingProviderStatus)[keyof typeof TrainingProviderStatus];

// ─── Gradient Wire Format ──────────────────────────────────────────────────────────

/** Serialization format for gradient tensors transmitted over the wire. */
export const GradientWireFormat = {
  F32: 'f32',
  Int8: 'int8',
} as const;

export type GradientWireFormat = (typeof GradientWireFormat)[keyof typeof GradientWireFormat];

// ─── DiLoCo Configuration ──────────────────────────────────────────────────────────

/** Configuration for Distributed-Local-Composite (DiLoCo) training rounds.
 *  Mirrors TrainingConfig from neunode-training crate. Named DiLoCoConfig to
 *  avoid collision with the existing TrainingConfig in model.ts. */
export interface DiLoCoConfig {
  /** Number of local SGD steps between global synchronizations. */
  readonly local_steps: number;
  /** Learning rate for inner (local) optimization loop. */
  readonly inner_lr: number;
  /** Learning rate for outer (global) model update. */
  readonly outer_lr: number;
  /** Momentum coefficient applied to the outer optimization step. */
  readonly outer_momentum: number;
  /** Per-worker minibatch size for local training steps. */
  readonly batch_size: number;
  /** Bits per gradient element for quantized all-reduce (e.g. 8 for int8). */
  readonly quantization_bits: number;
  /** Maximum number of concurrent workers in a single training round. */
  readonly max_workers: number;
  /** Seconds before a worker is marked suspect due to missed heartbeats. */
  readonly heartbeat_timeout_secs: number;
  /** Number of outer steps between checkpoint saves. */
  readonly checkpoint_interval: number;
}

// ─── Worker Info ───────────────────────────────────────────────────────────────────

/** Runtime information about a worker participating in a training job. */
export interface WorkerInfo {
  /** DID identifying the worker agent. */
  readonly worker_id: Did;
  /** Number of GPUs available on this worker. */
  readonly gpu_count: number;
  /** Total GPU memory in gigabytes. */
  readonly gpu_memory_gb: number;
  /** Maximum model parameters this worker can host in memory. */
  readonly max_model_params: number;
  /** Whether the worker supports bfloat16 arithmetic. */
  readonly supports_bf16: boolean;
  /** Current operational status of the worker. */
  readonly status: WorkerStatus;
  /** Reputation score (0–1) from the 5-factor scoring system. */
  readonly reputation_score: number;
  /** Milliseconds since epoch of the last received heartbeat. */
  readonly last_heartbeat_ms: Timestamp;
}

// ─── Milestone Info ────────────────────────────────────────────────────────────────

/** A single training milestone used for streaming settlement payouts. */
export interface MilestoneInfo {
  /** Outer step number this milestone corresponds to. */
  readonly step: number;
  /** DID of the worker whose contribution earned this payout. */
  readonly worker_id: Did;
  /** Normalized contribution score (0–1) for this milestone. */
  readonly contribution_score: number;
  /** Token amount awarded for this milestone. */
  readonly token_amount: TokenAmount;
}

// ─── Training Settlement Info ──────────────────────────────────────────────────────

/** Settlement details for a completed (or in-progress) training job. */
export interface TrainingSettlementInfo {
  /** The training job this settlement belongs to. */
  readonly job_id: JobId;
  /** DID of the agent that requested the training job. */
  readonly requester: Did;
  /** Total tokens deposited into escrow for this job. */
  readonly total_deposit: TokenAmount;
  /** Milestone-based payout records. */
  readonly milestones: readonly MilestoneInfo[];
  /** Current settlement state. */
  readonly status: SettlementStatus;
  /** Protocol fee percentage (e.g. 0.02 for 2%). */
  readonly protocol_fee_pct: number;
}

// ─── Health Info ───────────────────────────────────────────────────────────────────

/** Heartbeat-based health status for a single worker. */
export interface HealthInfo {
  /** DID of the worker being monitored. */
  readonly worker_id: Did;
  /** Milliseconds since epoch of the last received heartbeat. */
  readonly last_heartbeat_ms: Timestamp;
  /** Current liveness state. */
  readonly state: HealthState;
  /** Number of consecutive heartbeats missed since last response. */
  readonly missed_heartbeats: number;
}

// ─── Fault Event Info ──────────────────────────────────────────────────────────────

/** Discriminated union of fault events emitted by the training fault detector. */
export type FaultEventInfo =
  | { readonly type: 'WorkerSuspect'; readonly worker_id: Did; readonly details: string }
  | { readonly type: 'WorkerDead'; readonly worker_id: Did; readonly details: string }
  | { readonly type: 'WorkerRecovered'; readonly worker_id: Did; readonly details: string }
  | { readonly type: 'RecoveryNeeded'; readonly worker_id: Did; readonly details: string };
