use neunode_core::constants::token::{
    DECAY_BURN_PCT, DECAY_DEV_FUND_PCT, DECAY_STAKING_REWARDS_PCT, DECAY_TREASURY_PCT,
};
use neunode_core::types::{ActivityLevel, TokenAmount};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DecayDistribution {
    pub treasury: TokenAmount,
    pub staking_rewards: TokenAmount,
    pub burned: TokenAmount,
    pub dev_fund: TokenAmount,
}

impl DecayDistribution {
    pub fn total(&self) -> TokenAmount {
        let total: u128 = self
            .treasury
            .0
            .saturating_add(self.staking_rewards.0)
            .saturating_add(self.burned.0)
            .saturating_add(self.dev_fund.0);
        TokenAmount(total)
    }
}

#[derive(Debug, Clone)]
pub struct DecayCalculator;

impl DecayCalculator {
    pub fn effective_decay_rate(activity_level: ActivityLevel) -> f64 {
        activity_level.decay_rate()
    }

    pub fn calculate_decay(
        balance: TokenAmount,
        activity_level: ActivityLevel,
        periods: u32,
    ) -> TokenAmount {
        if balance == TokenAmount::ZERO || periods == 0 {
            return balance;
        }

        let rate = Self::effective_decay_rate(activity_level);
        if rate == 0.0 {
            return balance;
        }

        let mut current = balance.0 as f64;
        let multiplier = 1.0 - (rate / 100.0);
        for _ in 0..periods {
            current *= multiplier;
            if current < 1.0 {
                current = 0.0;
                break;
            }
        }

        TokenAmount(current as u128)
    }

    pub fn apply_decay(
        balance: TokenAmount,
        activity_level: ActivityLevel,
    ) -> (TokenAmount, DecayDistribution) {
        let new_balance = Self::calculate_decay(balance, activity_level, 1);

        let decayed_amount = balance.0.saturating_sub(new_balance.0);

        if decayed_amount == 0 {
            return (
                balance,
                DecayDistribution {
                    treasury: TokenAmount::ZERO,
                    staking_rewards: TokenAmount::ZERO,
                    burned: TokenAmount::ZERO,
                    dev_fund: TokenAmount::ZERO,
                },
            );
        }

        let treasury = Self::distribute_share(decayed_amount, DECAY_TREASURY_PCT);
        let staking_rewards = Self::distribute_share(decayed_amount, DECAY_STAKING_REWARDS_PCT);
        let burned = Self::distribute_share(decayed_amount, DECAY_BURN_PCT);
        let dev_fund = Self::distribute_share(decayed_amount, DECAY_DEV_FUND_PCT);

        // Remainder from rounding goes to treasury (largest share)
        let distributed: u128 = treasury.0 + staking_rewards.0 + burned.0 + dev_fund.0;
        let treasury = TokenAmount(treasury.0 + (decayed_amount - distributed));

        (new_balance, DecayDistribution { treasury, staking_rewards, burned, dev_fund })
    }

    fn distribute_share(total: u128, percentage: f64) -> TokenAmount {
        TokenAmount((total as f64 * percentage / 100.0) as u128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_decay_for_active() {
        let result = DecayCalculator::calculate_decay(TokenAmount(1000), ActivityLevel::Active, 1);
        assert_eq!(result, TokenAmount(1000));
    }

    #[test]
    fn zero_decay_for_active_multiple_periods() {
        let result =
            DecayCalculator::calculate_decay(TokenAmount(1000), ActivityLevel::Active, 100);
        assert_eq!(result, TokenAmount(1000));
    }

    #[test]
    fn decay_moderate_one_period() {
        // 2% decay: 1000 * 0.98 = 980
        let result =
            DecayCalculator::calculate_decay(TokenAmount(1000), ActivityLevel::Moderate, 1);
        assert_eq!(result, TokenAmount(980));
    }

    #[test]
    fn decay_low_one_period() {
        // 5% decay: 1000 * 0.95 = 950
        let result = DecayCalculator::calculate_decay(TokenAmount(1000), ActivityLevel::Low, 1);
        assert_eq!(result, TokenAmount(950));
    }

    #[test]
    fn decay_inactive_one_period() {
        // 15% decay: 1000 * 0.85 = 850
        let result =
            DecayCalculator::calculate_decay(TokenAmount(1000), ActivityLevel::Inactive, 1);
        assert_eq!(result, TokenAmount(850));
    }

    #[test]
    fn decay_dead_one_period() {
        // 50% decay: 1000 * 0.50 = 500
        let result = DecayCalculator::calculate_decay(TokenAmount(1000), ActivityLevel::Dead, 1);
        assert_eq!(result, TokenAmount(500));
    }

    #[test]
    fn decay_multiple_periods() {
        // 2% for 3 periods: 1000 * 0.98^3 = 941.192 → 941
        let result =
            DecayCalculator::calculate_decay(TokenAmount(1000), ActivityLevel::Moderate, 3);
        assert_eq!(result, TokenAmount(941));
    }

    #[test]
    fn decay_zero_periods() {
        let result = DecayCalculator::calculate_decay(TokenAmount(1000), ActivityLevel::Dead, 0);
        assert_eq!(result, TokenAmount(1000));
    }

    #[test]
    fn decay_zero_balance() {
        let result = DecayCalculator::calculate_decay(TokenAmount::ZERO, ActivityLevel::Dead, 10);
        assert_eq!(result, TokenAmount::ZERO);
    }

    #[test]
    fn apply_decay_active_no_distribution() {
        let (new_bal, dist) =
            DecayCalculator::apply_decay(TokenAmount(1000), ActivityLevel::Active);
        assert_eq!(new_bal, TokenAmount(1000));
        assert_eq!(dist.total(), TokenAmount::ZERO);
    }

    #[test]
    fn apply_decay_moderate_distribution() {
        // 2% of 1000 = 20 decayed
        // treasury: 40% of 20 = 8
        // staking: 30% of 20 = 6
        // burned: 20% of 20 = 4
        // dev_fund: 10% of 20 = 2
        let (new_bal, dist) =
            DecayCalculator::apply_decay(TokenAmount(1000), ActivityLevel::Moderate);
        assert_eq!(new_bal, TokenAmount(980));
        assert_eq!(dist.treasury, TokenAmount(8));
        assert_eq!(dist.staking_rewards, TokenAmount(6));
        assert_eq!(dist.burned, TokenAmount(4));
        assert_eq!(dist.dev_fund, TokenAmount(2));
        assert_eq!(dist.total(), TokenAmount(20));
    }

    #[test]
    fn apply_decay_dead_distribution() {
        // 50% of 1000 = 500 decayed
        // treasury: 40% of 500 = 200
        // staking: 30% of 500 = 150
        // burned: 20% of 500 = 100
        // dev_fund: 10% of 500 = 50
        let (new_bal, dist) = DecayCalculator::apply_decay(TokenAmount(1000), ActivityLevel::Dead);
        assert_eq!(new_bal, TokenAmount(500));
        assert_eq!(dist.treasury, TokenAmount(200));
        assert_eq!(dist.staking_rewards, TokenAmount(150));
        assert_eq!(dist.burned, TokenAmount(100));
        assert_eq!(dist.dev_fund, TokenAmount(50));
        assert_eq!(dist.total(), TokenAmount(500));
    }

    #[test]
    fn apply_decay_distribution_sums_to_decayed() {
        // With odd numbers, remainder goes to treasury
        let (_, dist) = DecayCalculator::apply_decay(TokenAmount(999), ActivityLevel::Inactive);
        let decayed = 999u64 - (999_f64 * 0.85) as u64;
        assert_eq!(dist.total(), TokenAmount(decayed as u128));
    }

    #[test]
    fn effective_decay_rates() {
        assert_eq!(DecayCalculator::effective_decay_rate(ActivityLevel::Active), 0.0);
        assert_eq!(DecayCalculator::effective_decay_rate(ActivityLevel::Moderate), 2.0);
        assert_eq!(DecayCalculator::effective_decay_rate(ActivityLevel::Low), 5.0);
        assert_eq!(DecayCalculator::effective_decay_rate(ActivityLevel::Inactive), 15.0);
        assert_eq!(DecayCalculator::effective_decay_rate(ActivityLevel::Dead), 50.0);
    }

    #[test]
    fn large_balance_decay() {
        let result = DecayCalculator::calculate_decay(
            TokenAmount(1_000_000_000),
            ActivityLevel::Inactive,
            1,
        );
        // 15% decay: 1_000_000_000 * 0.85 = 850_000_000
        assert_eq!(result, TokenAmount(850_000_000));
    }

    #[test]
    fn decay_approaches_zero_over_many_periods() {
        let result = DecayCalculator::calculate_decay(TokenAmount(1000), ActivityLevel::Dead, 100);
        assert!(result.0 < 1);
    }

    #[test]
    fn decay_distribution_zero_balance() {
        let (new_bal, dist) = DecayCalculator::apply_decay(TokenAmount::ZERO, ActivityLevel::Dead);
        assert_eq!(new_bal, TokenAmount::ZERO);
        assert_eq!(dist.total(), TokenAmount::ZERO);
    }

    #[test]
    fn decay_distribution_serde_roundtrip() {
        let dist = DecayDistribution {
            treasury: TokenAmount(40),
            staking_rewards: TokenAmount(30),
            burned: TokenAmount(20),
            dev_fund: TokenAmount(10),
        };
        let json = serde_json::to_string(&dist).unwrap();
        let back: DecayDistribution = serde_json::from_str(&json).unwrap();
        assert_eq!(dist, back);
    }

    #[test]
    fn decay_distribution_total() {
        let dist = DecayDistribution {
            treasury: TokenAmount(40),
            staking_rewards: TokenAmount(30),
            burned: TokenAmount(20),
            dev_fund: TokenAmount(10),
        };
        assert_eq!(dist.total(), TokenAmount(100));
    }
}
