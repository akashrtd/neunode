import type { NeunodeClient } from "../client/client.js";

export interface TokenBalanceResult {
	token: string;
	balance: string;
	staked: string;
}

export interface TokenAllBalancesResult {
	balances: TokenBalanceResult[];
}

export interface TokenTransferParams {
	to: string;
	amount: number;
	token: string;
}

export interface TokenTransferResult {
	from: string;
	to: string;
	amount: number;
	token: string;
	state: string;
}

export interface TokenStakeParams {
	amount: number;
	token: string;
}

export interface TokenStakeResult {
	amount: number;
	token: string;
	state: string;
	unbonding_period_secs: number;
}

export interface TokenUnstakeResult {
	id: string;
	amount: number;
	token: string;
	unbond_at: number;
	state: string;
}

export interface TokenStakeStatusResult {
	total_staked: number;
	entries: Array<{
		amount: number;
		token: string;
		available: number;
	}>;
	unbonding: Array<{
		id: string;
		token: string;
		amount: number;
		created_at: number;
		unlock_at: number;
	}>;
}

export interface TokenClaimUnbondedResult {
	claimed_amount: number;
	claimed_positions: number;
	state: "Claimed" | "NothingMatured";
}

export interface TokenDecayInfoResult {
	data: Array<{
		"Activity Level": string;
		"Decay Rate": string;
		Treasury: string;
		Staking: string;
		Burned: string;
		"Dev Fund": string;
	}>;
}

/** Resource token balances, staking, transfers, and decay info. */
export interface TokenResource {
	/** Get balance for a specific token, or all tokens if unspecified. */
	balance(token?: string): Promise<TokenBalanceResult | TokenAllBalancesResult>;
	/** Transfer tokens to another agent. */
	transfer(params: TokenTransferParams): Promise<TokenTransferResult>;
	/** Stake tokens as reputation collateral. */
	stake(params: TokenStakeParams): Promise<TokenStakeResult>;
	/** Begin unstaking tokens (subject to unbonding period). */
	unstake(amount: number): Promise<TokenUnstakeResult>;
	/** Claim every unbonding position whose maturity time has passed. */
	claimUnbonded(): Promise<TokenClaimUnbondedResult>;
	/** Get current staking positions and unbonding status. */
	stakeStatus(): Promise<TokenStakeStatusResult>;
	/** Get activity-based decay rates and distribution breakdown. */
	decayInfo(): Promise<TokenDecayInfoResult>;
}

export function createTokenResource(client: NeunodeClient): TokenResource {
	return {
		async balance(
			token?: string,
		): Promise<TokenBalanceResult | TokenAllBalancesResult> {
			const qs = new URLSearchParams();
			if (token) qs.set("token", token);
			const query = qs.toString();
			return client.http.get<TokenBalanceResult | TokenAllBalancesResult>(
				query ? `/api/v1/tokens/balance?${query}` : "/api/v1/tokens/balance",
			);
		},

		async transfer(params: TokenTransferParams): Promise<TokenTransferResult> {
			return client.http.post<TokenTransferResult>(
				"/api/v1/tokens/transfer",
				params,
			);
		},

		async stake(params: TokenStakeParams): Promise<TokenStakeResult> {
			return client.http.post<TokenStakeResult>("/api/v1/tokens/stake", params);
		},

		async unstake(amount: number): Promise<TokenUnstakeResult> {
			return client.http.post<TokenUnstakeResult>("/api/v1/tokens/unstake", {
				amount,
			});
		},

		async claimUnbonded(): Promise<TokenClaimUnbondedResult> {
			return client.http.post<TokenClaimUnbondedResult>(
				"/api/v1/tokens/claim-unbonded",
			);
		},

		async stakeStatus(): Promise<TokenStakeStatusResult> {
			return client.http.get<TokenStakeStatusResult>(
				"/api/v1/tokens/stake-status",
			);
		},

		async decayInfo(): Promise<TokenDecayInfoResult> {
			return client.http.get<TokenDecayInfoResult>("/api/v1/tokens/decay-info");
		},
	};
}
