//! Neunode L1 chain constants.

/// Neunode chain ID. Not registered on chainlist.org as of 2026-05-29.
pub const CHAIN_ID: u64 = 9109;

/// Human-readable chain name.
pub const CHAIN_NAME: &str = "neunode";

/// Gas token display name.
pub const NATIVE_CURRENCY_NAME: &str = "Neu";

/// Gas token ticker symbol.
pub const NATIVE_CURRENCY_SYMBOL: &str = "NEU";

/// Gas token decimals (same as ETH).
pub const NATIVE_CURRENCY_DECIMALS: u8 = 18;

/// Block gas limit: 30M (same as Ethereum mainnet).
/// Sufficient for complex AI agent interactions (model registration,
/// bounty creation, multi-step verification).
pub const BLOCK_GAS_LIMIT: u64 = 30_000_000;

/// Initial base fee: 1 Gwei in wei.
pub const INITIAL_BASE_FEE: u64 = 1_000_000_000;

/// Number of initial validators.
pub const INITIAL_VALIDATORS: usize = 4;

/// "neunode" in ASCII hex, used as extra data in genesis block.
pub const GENESIS_EXTRA_DATA: &[u8] = &[0x8e, 0x65, 0x75, 0x6e, 0x6f, 0x64, 0x65];
