/**
 * Shared types for the Neunode MCP server.
 */

// ---------------------------------------------------------------------------
// API envelope (matches agnetd's JSON envelope format)
// ---------------------------------------------------------------------------

export interface SuccessEnvelope<T> {
  readonly data: T;
  readonly success: true;
}

export interface ErrorEnvelope {
  readonly error: {
    readonly code: string;
    readonly message: string;
  };
  readonly success: false;
}

export type ApiEnvelope<T> = SuccessEnvelope<T> | ErrorEnvelope;

// ---------------------------------------------------------------------------
// Transport config
// ---------------------------------------------------------------------------

export interface McpServerConfig {
  /** URL of the agnetd daemon. Default: http://127.0.0.1:41000 */
  readonly agnetdUrl: string;
  /** Transport mode: "stdio" or "http". Default: "stdio" */
  readonly transport: "stdio" | "http";
  /** Port for HTTP transport. Default: 3100 */
  readonly port: number;
}

// ---------------------------------------------------------------------------
// MCP tool result
// ---------------------------------------------------------------------------

export interface ToolResult {
  readonly content: ReadonlyArray<{
    readonly type: "text";
    readonly text: string;
  }>;
}

// ---------------------------------------------------------------------------
// Identity types
// ---------------------------------------------------------------------------

export interface IdentityResponse {
  readonly did: string;
  readonly method: string;
  readonly name: string;
  readonly ethereum: string;
  readonly peer_id: string;
}

export interface IdentityListItem {
  readonly did: string;
  readonly status: string;
}

export interface CreateIdentityResponse {
  readonly identity: IdentityResponse;
  readonly card_cid: string;
}

// ---------------------------------------------------------------------------
// Feed types
// ---------------------------------------------------------------------------

export interface FeedEventResponse {
  readonly sequence: number;
  readonly kind: number;
  readonly timestamp: number;
  readonly author_did: string;
  readonly content: string;
  readonly signature: string;
}

export interface PostFeedResponse {
  readonly event_id: string;
  readonly sequence: number;
  readonly kind: number;
  readonly topic: string;
}

// ---------------------------------------------------------------------------
// Bounty types
// ---------------------------------------------------------------------------

export interface BountyResponse {
  readonly id: string;
  readonly title: string;
  readonly description: string;
  readonly state: string;
  readonly creator: string;
  readonly claimant: string | null;
  readonly reward: number;
  readonly reward_token_type: number;
  readonly escrow_deposited: number;
  readonly created_at: number;
  readonly claim_deadline: number;
  readonly work_deadline: number;
  readonly review_deadline: number;
  readonly artifact_hash: string | null;
  readonly bond: number | null;
}

export interface BountyListItem {
  readonly id: string;
  readonly title: string;
  readonly state: string;
  readonly reward: number;
  readonly creator: string;
  readonly created_at: number;
}

export interface BountyActionResponse {
  readonly bounty_id: string;
  readonly state: string;
}

// ---------------------------------------------------------------------------
// Token types
// ---------------------------------------------------------------------------

export interface BalanceResponse {
  readonly token: string;
  readonly balance: number;
  readonly staked: number;
}

export interface AllBalancesResponse {
  readonly balances: ReadonlyArray<BalanceResponse>;
}

export interface TransferResponse {
  readonly from: string;
  readonly to: string;
  readonly amount: number;
  readonly token: string;
  readonly state: string;
}

export interface StakeResponse {
  readonly amount: number;
  readonly token: string;
  readonly state: string;
  readonly unbonding_period_secs: number;
}

export interface UnstakeResponse {
  readonly amount: number;
  readonly token: string;
  readonly unbond_at: number;
  readonly state: string;
}

export interface StakeStatusResponse {
  readonly total_staked: number;
  readonly entries: ReadonlyArray<{
    readonly amount: number;
    readonly token: string;
    readonly available: number;
  }>;
}

// ---------------------------------------------------------------------------
// Inference types
// ---------------------------------------------------------------------------

export interface InferenceRequestResponse {
  readonly model: string;
  readonly prompt: string;
  readonly max_tokens: number;
  readonly temperature: number;
  readonly estimated_input_tokens: number;
  readonly status: string;
  readonly pricing?: {
    readonly input_price_per_mtok: number;
    readonly output_price_per_mtok: number;
    readonly estimated_cost: number;
  };
}

export interface ModelEntry {
  readonly id: string;
  readonly input_price_per_million: number;
  readonly output_price_per_million: number;
  readonly context_length: number;
}

export interface ProviderEntry {
  readonly name: string;
  readonly did: string;
  readonly status: string;
  readonly reputation_score: number;
  readonly avg_latency_ms: number;
  readonly model_count: number;
}

// ---------------------------------------------------------------------------
// Model registry types
// ---------------------------------------------------------------------------

export interface ModelResponse {
  readonly id: string;
  readonly base_model: string | null;
  readonly context_length: number;
  readonly input_price_per_million: number;
  readonly output_price_per_million: number;
  readonly total_price_per_million: number;
  readonly capabilities: ReadonlyArray<string>;
}

// ---------------------------------------------------------------------------
// Mesh types
// ---------------------------------------------------------------------------

export interface MeshStatusResponse {
  readonly running: boolean;
  readonly local_peer_id: string | null;
  readonly listeners: ReadonlyArray<string>;
  readonly connected_peers: ReadonlyArray<string>;
  readonly subscribed_topics: ReadonlyArray<string>;
}

export interface PeersResponse {
  readonly peers: ReadonlyArray<string>;
  readonly count: number;
}
