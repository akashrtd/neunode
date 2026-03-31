// @neunode/sdk — Token types mirroring neunode-token (staking, decay, balance)

import type { TokenAmount, TokenType, Timestamp } from './core.js';

export interface StakeEntry {
  readonly amount: TokenAmount;
  readonly token_type: TokenType;
  readonly staked_at: Timestamp;
  readonly unbonding_at?: Timestamp;
}

export interface DecayDistribution {
  readonly treasury: TokenAmount;
  readonly staking_rewards: TokenAmount;
  readonly burned: TokenAmount;
  readonly dev_fund: TokenAmount;
}

export interface BalanceInfo {
  readonly token_type: TokenType;
  readonly available: TokenAmount;
  readonly staked: TokenAmount;
  readonly unbonding: TokenAmount;
  readonly total: TokenAmount;
}
