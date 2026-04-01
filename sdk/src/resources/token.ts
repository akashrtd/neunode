import type { NeunodeClient } from "../client/client.js";

export interface TokenBalanceResult {
	token: string;
	balance: string;
	staked: string;
}

export interface TokenAllBalancesResult {
	data: Array<{ Token: string; Balance: string; Staked: string }>;
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
	/** Get current staking positions and unbonding status. */
	stakeStatus(): Promise<TokenStakeStatusResult>;
	/** Get activity-based decay rates and distribution breakdown. */
	decayInfo(): Promise<TokenDecayInfoResult>;
}

export function createTokenResource(client: NeunodeClient): TokenResource {
	const cli = client.cli;
	if (!cli) throw new Error("CLI transport required for token operations");

	return {
		async balance(
			token?: string,
		): Promise<TokenBalanceResult | TokenAllBalancesResult> {
			const args = ["token", "balance"];
			if (token) args.push("--token", token);
			return cli.execute<TokenBalanceResult | TokenAllBalancesResult>(args);
		},

		async transfer(params: TokenTransferParams): Promise<TokenTransferResult> {
			return cli.execute<TokenTransferResult>([
				"token",
				"transfer",
				"--to",
				params.to,
				"--amount",
				String(params.amount),
				"--token",
				params.token,
			]);
		},

		async stake(params: TokenStakeParams): Promise<TokenStakeResult> {
			return cli.execute<TokenStakeResult>([
				"token",
				"stake",
				"--amount",
				String(params.amount),
				"--token",
				params.token,
			]);
		},

		async unstake(amount: number): Promise<TokenUnstakeResult> {
			return cli.execute<TokenUnstakeResult>([
				"token",
				"unstake",
				"--amount",
				String(amount),
			]);
		},

		async stakeStatus(): Promise<TokenStakeStatusResult> {
			return cli.execute<TokenStakeStatusResult>(["token", "stake-status"]);
		},

		async decayInfo(): Promise<TokenDecayInfoResult> {
			return cli.execute<TokenDecayInfoResult>(["token", "decay-info"]);
		},
	};
}
