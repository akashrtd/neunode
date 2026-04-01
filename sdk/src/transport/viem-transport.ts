/**
 * Viem (Ethereum) transport for @neunode/sdk.
 *
 * Provides direct contract interaction via Viem.
 * This is an OPTIONAL transport — viem is a peer dependency.
 */

import type { Chain, PublicClient, WalletClient } from "viem";

// ---------------------------------------------------------------------------
// Viem transport config
// ---------------------------------------------------------------------------

export interface ViemTransportConfig {
	/** Viem public client for read operations. */
	readonly publicClient: PublicClient;
	/** Optional wallet client for write operations. */
	readonly walletClient?: WalletClient;
	/** Chain configuration. */
	readonly chain: Chain;
}

// ---------------------------------------------------------------------------
// Viem transport wrapper
// ---------------------------------------------------------------------------

export class ViemTransport {
	public readonly publicClient: PublicClient;
	public readonly walletClient: WalletClient | undefined;
	public readonly chain: Chain;

	constructor(config: ViemTransportConfig) {
		this.publicClient = config.publicClient;
		this.walletClient = config.walletClient;
		this.chain = config.chain;
	}

	/** Whether a wallet client is available for write operations. */
	get canWrite(): boolean {
		return this.walletClient !== undefined;
	}

	/** Get the chain ID. */
	get chainId(): number {
		return this.chain.id;
	}
}
