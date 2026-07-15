// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

/// @title INeunodeToken — Interface for Neunode resource-backed tokens
/// @notice Defines roles, staking, activity tracking, decay, and seed token operations.
interface INeunodeToken {
    // ─── Roles ────────────────────────────────────────────────────────────

    function MINTER_ROLE() external view returns (bytes32);
    function BURNER_ROLE() external view returns (bytes32);
    function GOVERNANCE_ROLE() external view returns (bytes32);

    // ─── Supply ──────────────────────────────────────────────────────────

    function supplyCap() external view returns (uint256);
    function maxSupplyCap() external view returns (uint256);
    function setSupplyCap(uint256 newCap) external;

    // ─── Staking ──────────────────────────────────────────────────────────

    function stake(uint256 amount) external;
    function unstake(uint256 amount) external;
    function stakedBalanceOf(address account) external view returns (uint256);
    function slashStake(address account, uint256 amount) external;

    // ─── Activity ─────────────────────────────────────────────────────────

    function updateActivity(address account) external;
    function lastActivity(address account) external view returns (uint256);
    function getActivityLevel(address account) external view returns (uint8);

    // Decay logic has been extracted to StakingEscrow.sol

    // ─── Seed Tokens ──────────────────────────────────────────────────────

    function mintSeed(address to, uint256 amount) external;
    function activateSeed(address account) external;

    // ─── Events ───────────────────────────────────────────────────────────

    event Staked(address indexed account, uint256 amount);
    event Unstaked(address indexed account, uint256 amount);
    event StakeSlashed(address indexed account, uint256 amount);

    event SeedMinted(address indexed to, uint256 amount);
    event SeedActivated(address indexed account);
    event ActivityUpdated(address indexed account, uint256 timestamp);
    event SupplyCapUpdated(uint256 previousCap, uint256 newCap);
}
