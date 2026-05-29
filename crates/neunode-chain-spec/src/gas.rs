//! EIP-1559 parameters tuned for AI agent microtransactions.
//!
//! Default Ethereum mainnet uses `max_change_denominator=8, elasticity_multiplier=2`.
//! Neunode uses slower fee changes (16) and a wider target band (4) to keep gas
//! prices predictable for agents submitting many small transactions.

/// EIP-1559 configuration for the Neunode chain.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NeunodeEip1559Config {
    /// Maximum base fee change per block is `1/denominator`.
    /// 16 = 6.25% max change (vs 12.5% on mainnet).
    pub max_change_denominator: u64,
    /// Gas target = gas_limit / elasticity_multiplier.
    /// 4 = wider headroom for burst traffic from many agents.
    pub elasticity_multiplier: u64,
    /// Starting base fee in wei (1 Gwei).
    pub initial_base_fee: u64,
    /// Denominator for max per-block base fee increase.
    /// With denominator 8, max increase is parent_fee / 8.
    pub base_fee_max_increase_denominator: u64,
    /// Denominator for max per-block base fee decrease.
    /// With denominator 8, max decrease is parent_fee / 8.
    pub base_fee_max_decrease_denominator: u64,
}

impl Default for NeunodeEip1559Config {
    fn default() -> Self {
        Self {
            max_change_denominator: 16,
            elasticity_multiplier: 4,
            initial_base_fee: 1_000_000_000, // 1 Gwei
            base_fee_max_increase_denominator: 8,
            base_fee_max_decrease_denominator: 8,
        }
    }
}

impl NeunodeEip1559Config {
    /// Returns the Neunode EIP-1559 config.
    pub fn neunode() -> Self {
        Self::default()
    }

    /// Maximum possible base fee increase per block as a fraction (e.g., 0.0625 = 6.25%).
    pub fn max_increase_fraction(&self) -> f64 {
        1.0 / self.max_change_denominator as f64
    }

    /// Maximum possible base fee decrease per block as a fraction.
    pub fn max_decrease_fraction(&self) -> f64 {
        1.0 / self.max_change_denominator as f64
    }
}
