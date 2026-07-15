//! NeunodeToken base contract + 4 resource token bindings.
//!
//! Base ERC-20 with mint/burn, AccessControl roles, staking, activity tracking,
//! and seed tokens. Four concrete implementations: nCompute, nTrain, nBandwidth, nStorage.

use alloy::sol;

sol! {
    // ─── Events ───────────────────────────────────────────────────────────

    #[derive(Debug)]
    event Staked(address indexed account, uint256 amount);

    #[derive(Debug)]
    event Unstaked(address indexed account, uint256 amount);

    #[derive(Debug)]
    event StakeSlashed(address indexed account, uint256 amount);

    #[derive(Debug)]
    event SeedMinted(address indexed to, uint256 amount);

    #[derive(Debug)]
    event SeedActivated(address indexed account);

    #[derive(Debug)]
    event ActivityUpdated(address indexed account, uint256 timestamp);

    #[derive(Debug)]
    event SupplyCapUpdated(uint256 previousCap, uint256 newCap);

    // ─── Errors ───────────────────────────────────────────────────────────

    error UnauthorizedActivityUpdate(address caller, address account);
    error InsufficientBalance(address account, uint256 required);
    error InsufficientStake(address account, uint256 required);
    error CannotUnstakeSeed();
    error SupplyCapExceeded(uint256 requestedSupply, uint256 cap);
    error SupplyCapBelowCurrentSupply(uint256 requestedCap, uint256 currentSupply);
    error SupplyCapAboveMaximum(uint256 requestedCap, uint256 maximumCap);

    // ─── Functions (INeunodeToken interface) ───────────────────────────────

    // Roles
    function MINTER_ROLE() external view returns (bytes32);
    function BURNER_ROLE() external view returns (bytes32);
    function GOVERNANCE_ROLE() external view returns (bytes32);

    // Supply
    function supplyCap() external view returns (uint256);
    function maxSupplyCap() external view returns (uint256);
    function setSupplyCap(uint256 newCap) external;

    // Staking
    function stake(uint256 amount) external;
    function unstake(uint256 amount) external;
    function stakedBalanceOf(address account) external view returns (uint256);
    function slashStake(address account, uint256 amount) external;

    // Activity
    function updateActivity(address account) external;
    function lastActivity(address account) external view returns (uint256);
    function getActivityLevel(address account) external view returns (uint8);

    // Seed tokens
    function mintSeed(address to, uint256 amount) external;
    function activateSeed(address account) external;
    function seedBalanceOf(address account) external view returns (uint256);

    // Mint / Burn
    function mint(address to, uint256 amount) external;
    function burn(address from, uint256 amount) external;

    // Standard ERC-20 (subset used by NeunodeToken)
    function decimals() external view returns (uint8);
}

sol! {
    /// StakingEscrow — Inactivity Decay Manager
    /// Moves decay logic out of the ERC-20 token for standard composability.

    #[derive(Debug)]
    event DecayExecuted(address indexed account, uint256 slashedAmount);

    error DecayTooSoon(address account);

    function computeDecay(address account) external view returns (uint256);
    function executeDecay(address account) external;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::U256;
    use alloy::primitives::address;
    use alloy::sol_types::{SolError, SolEvent};

    // ─── Event signature tests ──────────────────────────────────────────────

    #[test]
    fn staking_event_signatures_non_empty() {
        assert!(!Staked::SIGNATURE.is_empty());
        assert!(!Unstaked::SIGNATURE.is_empty());
        assert!(!StakeSlashed::SIGNATURE.is_empty());
        assert!(!SeedMinted::SIGNATURE.is_empty());
        assert!(!SeedActivated::SIGNATURE.is_empty());
        assert!(!ActivityUpdated::SIGNATURE.is_empty());
        assert!(!SupplyCapUpdated::SIGNATURE.is_empty());
    }

    #[test]
    fn staking_event_signatures_expected_format() {
        assert!(Staked::SIGNATURE.starts_with("Staked("));
        assert!(Unstaked::SIGNATURE.starts_with("Unstaked("));
        assert!(StakeSlashed::SIGNATURE.starts_with("StakeSlashed("));
        assert!(SeedMinted::SIGNATURE.starts_with("SeedMinted("));
        assert!(SeedActivated::SIGNATURE.starts_with("SeedActivated("));
        assert!(ActivityUpdated::SIGNATURE.starts_with("ActivityUpdated("));
        assert!(SupplyCapUpdated::SIGNATURE.starts_with("SupplyCapUpdated("));
    }

    #[test]
    fn staking_event_selectors_are_32_bytes() {
        assert_eq!(Staked::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(Unstaked::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(StakeSlashed::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(SeedMinted::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(SeedActivated::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(ActivityUpdated::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(SupplyCapUpdated::SIGNATURE_HASH.as_slice().len(), 32);
    }

    #[test]
    fn staking_event_selectors_unique() {
        let selectors = [
            Staked::SIGNATURE_HASH,
            Unstaked::SIGNATURE_HASH,
            StakeSlashed::SIGNATURE_HASH,
            SeedMinted::SIGNATURE_HASH,
            SeedActivated::SIGNATURE_HASH,
            ActivityUpdated::SIGNATURE_HASH,
            SupplyCapUpdated::SIGNATURE_HASH,
        ];
        for i in 0..selectors.len() {
            for j in (i + 1)..selectors.len() {
                assert_ne!(
                    selectors[i], selectors[j],
                    "Token staking event selectors must be unique"
                );
            }
        }
    }

    // ─── Decay event tests ──────────────────────────────────────────────────

    #[test]
    fn decay_event_signature() {
        assert!(!DecayExecuted::SIGNATURE.is_empty());
        assert!(DecayExecuted::SIGNATURE.starts_with("DecayExecuted("));
        assert_eq!(DecayExecuted::SIGNATURE_HASH.as_slice().len(), 32);
    }

    // ─── Error construction tests ───────────────────────────────────────────

    #[test]
    fn token_error_types_constructible() {
        let acct = address!("0000000000000000000000000000000000000001");
        let caller = address!("0000000000000000000000000000000000000002");

        let _ = UnauthorizedActivityUpdate { caller, account: acct };
        let _ = InsufficientBalance { account: acct, required: U256::from(1000) };
        let _ = InsufficientStake { account: acct, required: U256::from(500) };
        let _ = CannotUnstakeSeed {};
        let _ = SupplyCapExceeded { requestedSupply: U256::from(1001), cap: U256::from(1000) };
        let _ = SupplyCapBelowCurrentSupply {
            requestedCap: U256::from(999),
            currentSupply: U256::from(1000),
        };
        let _ =
            SupplyCapAboveMaximum { requestedCap: U256::from(1001), maximumCap: U256::from(1000) };
    }

    #[test]
    fn token_error_selectors_are_4_bytes() {
        assert_eq!(UnauthorizedActivityUpdate::SELECTOR.len(), 4);
        assert_eq!(InsufficientBalance::SELECTOR.len(), 4);
        assert_eq!(InsufficientStake::SELECTOR.len(), 4);
        assert_eq!(CannotUnstakeSeed::SELECTOR.len(), 4);
        assert_eq!(SupplyCapExceeded::SELECTOR.len(), 4);
        assert_eq!(SupplyCapBelowCurrentSupply::SELECTOR.len(), 4);
        assert_eq!(SupplyCapAboveMaximum::SELECTOR.len(), 4);
    }

    #[test]
    fn decay_error_constructible() {
        let acct = address!("0000000000000000000000000000000000000001");
        let _ = DecayTooSoon { account: acct };
        assert_eq!(DecayTooSoon::SELECTOR.len(), 4);
    }

    // ─── Decay parameter tests ──────────────────────────────────────────────

    #[test]
    fn decay_calculation_range() {
        // Decay ranges from 0% (fully active) to 50% (dead)
        // Verify the range logic is sensible at the parameter level
        let max_decay_bps = 5000u64; // 50%
        let activity_levels = [
            (100, 0),   // 100% active -> 0% decay
            (50, 2500), // 50% active -> 25% decay
            (0, 5000),  // 0% active -> 50% decay
        ];
        for (activity, expected_bps) in activity_levels {
            let decay_bps = max_decay_bps * (100 - activity) / 100;
            assert_eq!(decay_bps, expected_bps);
        }
    }

    #[test]
    fn decay_redistribution_split() {
        // 40% treasury / 30% staking / 20% burn / 10% dev fund
        let total = U256::from(1000);
        let treasury = total * U256::from(40) / U256::from(100);
        let staking = total * U256::from(30) / U256::from(100);
        let burn = total * U256::from(20) / U256::from(100);
        let dev = total * U256::from(10) / U256::from(100);

        assert_eq!(treasury, U256::from(400));
        assert_eq!(staking, U256::from(300));
        assert_eq!(burn, U256::from(200));
        assert_eq!(dev, U256::from(100));
        assert_eq!(treasury + staking + burn + dev, total);
    }
}
