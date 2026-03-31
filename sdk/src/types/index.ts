// @neunode/sdk — Barrel re-exports for all type modules

// core — runtime consts + types
export { TokenType, AgentLifecycle, BountyState, ActivityLevel, Kind, KindCategory } from './core.js';
export type { Brand, Did, CID, PeerId, BountyId, EventId, ModelId, JobId, Hash256, Signature, Timestamp, Sequence, TokenAmount } from './core.js';

// identity — types only
export type { PublicKeyBundle, AgentCard, SignedAgentCard, VerificationMethod, ServiceEndpoint, DidDocument } from './identity.js';

// feed — types only
export type { EventTag, EventRef, FeedEvent, FeedFilter, BountyPost, BountyClaim, FeedAttestation } from './feed.js';

// token — types only
export type { StakeEntry, DecayDistribution, BalanceInfo } from './token.js';

// reputation — runtime const + types
export { ReputationGrade } from './reputation.js';
export type { FactorInputs, ReputationScore, FactorWeights, ReputationAttestation } from './reputation.js';

// bounty — runtime consts + types
export { EscrowState, ReviewOutcome, VerificationLayer } from './bounty.js';
export type { Deadlines, BountyData, BountyEvent, Escrow, FeeBreakdown, Review, ReviewCommittee, VerificationResult, VerificationPipeline, BountyDeadlines, BountyRecord } from './bounty.js';

// inference — runtime consts + types
export { MessageRole, FinishReason, ProviderStatus, RoutingStrategy } from './inference.js';
export type { ChatMessage, ChatCompletionRequest, Usage, Choice, ChatCompletionResponse, ChunkChoice, ChatCompletionChunk, ModelInfo, InferenceProvider } from './inference.js';

// p2p — types only
export type { PeerScoreParams, NodeEvent } from './p2p.js';

// model — runtime consts + types
export { TrainingStatus, ContributionType } from './model.js';
export type { LoRAConfig, TrainingConfig, ModelMetadata, TrainingJob, ModelLineage, RoyaltyRecipient, RoyaltyDistribution } from './model.js';

// config — types only
export type { AgentConfig, NetworkConfig, StorageConfig, TokenConfig, AppConfig } from './config.js';

// cli-output — runtime const + types
export { OutputFormat } from './cli-output.js';
export type { SuccessEnvelope, ErrorEnvelope, CliOutput } from './cli-output.js';

// training — runtime consts + types (Phase 2)
export { WorkerStatus, CoordinatorStatus, HealthState, SettlementStatusValues, AggregationMode, TrainingProviderStatus, GradientWireFormat } from './training.js';
export type { DiLoCoConfig, WorkerInfo, MilestoneInfo, TrainingSettlementInfo, HealthInfo, FaultEventInfo, SettlementStatus } from './training.js';

// errors — runtime const + types
export { ExitCode } from './errors.js';
export type { NeunodeError, NeunodeErrorDetails } from './errors.js';
