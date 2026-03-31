// @neunode/sdk — Inference types mirroring neunode-inference (openai, provider, router)

import type { Did, TokenAmount, Timestamp } from './core.js';

export const MessageRole = {
  System: 'system',
  User: 'user',
  Assistant: 'assistant',
  Tool: 'tool',
} as const;

export type MessageRole = (typeof MessageRole)[keyof typeof MessageRole];

export interface ChatMessage {
  readonly role: MessageRole;
  readonly content: string;
  readonly name?: string;
}

/** OpenAI-compatible chat completion request. */
export interface ChatCompletionRequest {
  readonly model: string;
  readonly messages: readonly ChatMessage[];
  readonly temperature?: number;
  readonly max_tokens?: number;
  readonly top_p?: number;
  readonly stream?: boolean;
  readonly stop?: readonly string[];
  readonly frequency_penalty?: number;
  readonly presence_penalty?: number;
}

export const FinishReason = {
  Stop: 'stop',
  Length: 'length',
  ContentFilter: 'content_filter',
  ToolCalls: 'tool_calls',
} as const;

export type FinishReason = (typeof FinishReason)[keyof typeof FinishReason];

export interface Usage {
  readonly prompt_tokens: number;
  readonly completion_tokens: number;
  readonly total_tokens: number;
}

export interface Choice {
  readonly index: number;
  readonly message: ChatMessage;
  readonly finish_reason: FinishReason;
}

/** OpenAI-compatible chat completion response. */
export interface ChatCompletionResponse {
  readonly id: string;
  readonly object: 'chat.completion';
  readonly created: Timestamp;
  readonly model: string;
  readonly choices: readonly Choice[];
  readonly usage: Usage;
}

export interface ChunkChoice {
  readonly index: number;
  readonly delta: ChatMessage;
  readonly finish_reason?: FinishReason;
}

export interface ChatCompletionChunk {
  readonly id: string;
  readonly object: 'chat.completion.chunk';
  readonly created: Timestamp;
  readonly model: string;
  readonly choices: readonly ChunkChoice[];
}

export interface ModelInfo {
  readonly id: string;
  readonly base_model?: string;
  readonly context_length: number;
  readonly input_price_per_million: TokenAmount;
  readonly output_price_per_million: TokenAmount;
  readonly capabilities: readonly string[];
}

export const ProviderStatus = {
  Online: 'online',
  Degraded: 'degraded',
  Offline: 'offline',
} as const;

export type ProviderStatus = (typeof ProviderStatus)[keyof typeof ProviderStatus];

/** An inference provider registered on the network. */
export interface InferenceProvider {
  readonly did: Did;
  readonly name: string;
  readonly endpoint: string;
  readonly models: readonly ModelInfo[];
  readonly reputation_score: number;
  readonly stake_amount: TokenAmount;
  readonly status: ProviderStatus;
  readonly last_heartbeat: Timestamp;
  readonly total_requests_served: number;
  readonly avg_latency_ms: number;
}

export const RoutingStrategy = {
  Cheapest: 'Cheapest',
  Fastest: 'Fastest',
  HighestReputation: 'HighestReputation',
  Random: 'Random',
  RoundRobin: 'RoundRobin',
} as const;

export type RoutingStrategy = (typeof RoutingStrategy)[keyof typeof RoutingStrategy];
