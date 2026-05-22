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
	return {
		async balance(
			token?: string,
		): Promise<TokenBalanceResult | TokenAllBalancesResult> {
			if (client.http) {
				const qs = new URLSearchParams();
				if (token) qs.set("token", token);
				const query = qs.toString();
				return client.http.get<TokenBalanceResult | TokenAllBalancesResult>(
					query ? `/api/v1/tokens/balance?${query}` : "/api/v1/tokens/balance",
				);
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for token operations");
			const args = ["token", "balance"];
			if (token) args.push("--token", token);
			return cli.execute<TokenBalanceResult | TokenAllBalancesResult>(args);
		},

		async transfer(params: TokenTransferParams): Promise<TokenTransferResult> {
			if (client.http) {
				return client.http.post<TokenTransferResult>(
					"/api/v1/tokens/transfer",
					params,
				);
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for token operations");
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
			if (client.http) {
				return client.http.post<TokenStakeResult>(
					"/api/v1/tokens/stake",
					params,
				);
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for token operations");
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
			if (client.http) {
				return client.http.post<TokenUnstakeResult>(
					"/api/v1/tokens/unstake",
					{ amount },
				);
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for token operations");
			return cli.execute<TokenUnstakeResult>([
				"token",
				"unstake",
				"--amount",
				String(amount),
			]);
		},

		async stakeStatus(): Promise<TokenStakeStatusResult> {
			if (client.http) {
				return client.http.get<TokenStakeStatusResult>(
					"/api/v1/tokens/stake-status",
				);
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for token operations");
			return cli.execute<TokenStakeStatusResult>(["token", "stake-status"]);
		},

		async decayInfo(): Promise<TokenDecayInfoResult> {
			if (client.http) {
				return client.http.get<TokenDecayInfoResult>(
					"/api/v1/tokens/decay-info",
				);
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for token operations");
			return cli.execute<TokenDecayInfoResult>(["token", "decay-info"]);
		},
	};
}
