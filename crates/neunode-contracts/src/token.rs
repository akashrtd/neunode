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

    // ─── Errors ───────────────────────────────────────────────────────────

    error UnauthorizedActivityUpdate(address caller, address account);
    error InsufficientBalance(address account, uint256 required);
    error InsufficientStake(address account, uint256 required);
    error CannotUnstakeSeed();

    // ─── Functions (INeunodeToken interface) ───────────────────────────────

    // Roles
    function MINTER_ROLE() external view returns (bytes32);
    function BURNER_ROLE() external view returns (bytes32);
    function GOVERNANCE_ROLE() external view returns (bytes32);

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
