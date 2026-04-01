// @neunode/sdk — Config types mirroring neunode-core/src/config.rs

export interface AgentConfig {
	readonly name: string;
	readonly did_method: string;
	readonly data_dir: string;
	readonly log_level: string;
}

export interface NetworkConfig {
	readonly listen_addr: string;
	readonly bootstrap_peers: readonly string[];
	readonly mesh_degree: number;
	readonly enable_mdns: boolean;
	readonly enable_relay: boolean;
}

export interface StorageConfig {
	readonly db_path: string;
	readonly cache_size: number;
	readonly cache_ttl_secs: number;
}

export interface TokenConfig {
	readonly decay_check_interval_secs: number;
	readonly unbonding_period_secs: number;
}

export interface AppConfig {
	readonly agent: AgentConfig;
	readonly network: NetworkConfig;
	readonly storage: StorageConfig;
	readonly tokens: TokenConfig;
	readonly active_identity?: string;
}
