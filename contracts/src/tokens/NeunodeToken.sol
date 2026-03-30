// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";
import "../interfaces/INeunodeToken.sol";

/// @title NeunodeToken — Base ERC-20 for resource-backed tokens
/// @notice Abstract base with mint/burn, AccessControl roles, staking,
///         activity tracking, token decay, and seed tokens. Owner is the protocol.
abstract contract NeunodeToken is ERC20, Ownable, AccessControl, INeunodeToken {
    uint8 private immutable _tokenDecimals;

    // ─── Roles ────────────────────────────────────────────────────────────

    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 public constant BURNER_ROLE = keccak256("BURNER_ROLE");
    bytes32 public constant GOVERNANCE_ROLE = keccak256("GOVERNANCE_ROLE");

    // ─── Staking Storage ──────────────────────────────────────────────────

    mapping(address => uint256) private _stakedBalances;
    mapping(address => uint256) private _seedBalances;

    // ─── Activity Storage ─────────────────────────────────────────────────

    mapping(address => uint256) private _lastActivity;

    // ─── Decay Storage ────────────────────────────────────────────────────

    struct DecayConfig {
        address treasury;
        address stakingRewards;
        address devFund;
    }

    DecayConfig public decayConfig;
    mapping(address => uint256) private _lastDecayTimestamp;

    // Decay rates in basis points per day per activity level.
    // Index: 0=Active, 1=Moderate, 2=Low, 3=Inactive, 4=Dead
    // Active=0, Moderate=5 (~2%/yr), Low=14 (~5%/yr), Inactive=41 (~15%/yr), Dead=137 (~50%/yr)
    uint256[5] public decayRatesBps = [uint256(0), 5, 14, 41, 137];

    // ─── Constructor ──────────────────────────────────────────────────────

    constructor(string memory name, string memory symbol, uint8 decimals_)
        ERC20(name, symbol)
        Ownable(msg.sender)
    {
        _tokenDecimals = decimals_;
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(MINTER_ROLE, msg.sender);
        _grantRole(BURNER_ROLE, msg.sender);
        _grantRole(GOVERNANCE_ROLE, msg.sender);
    }

    function decimals() public view override returns (uint8) {
        return _tokenDecimals;
    }

    // ─── Modifiers ────────────────────────────────────────────────────────

    /// @dev Allows owner OR role-holder. Reverts with Ownable error for backward compat.
    modifier onlyOwnerOrRole(bytes32 role) {
        if (msg.sender != owner()) {
            if (!hasRole(role, msg.sender)) {
                revert OwnableUnauthorizedAccount(msg.sender);
            }
        }
        _;
    }

    // ─── Mint / Burn ──────────────────────────────────────────────────────

    /// @notice Mint tokens (owner or MINTER_ROLE)
    function mint(address to, uint256 amount) external onlyOwnerOrRole(MINTER_ROLE) {
        _mint(to, amount);
    }

    /// @notice Burn tokens (owner or BURNER_ROLE)
    function burn(address from, uint256 amount) external onlyOwnerOrRole(BURNER_ROLE) {
        _burn(from, amount);
    }

    // ─── Staking ──────────────────────────────────────────────────────────

    /// @notice Stake tokens (transferred from caller to this contract)
    function stake(uint256 amount) external {
        require(balanceOf(msg.sender) >= amount, "insufficient balance");
        _transfer(msg.sender, address(this), amount);
        _stakedBalances[msg.sender] += amount;
        emit Staked(msg.sender, amount);
    }

    /// @notice Unstake tokens (returned from contract to caller)
    function unstake(uint256 amount) external {
        require(_stakedBalances[msg.sender] >= amount, "insufficient staked");
        require(
            _seedBalances[msg.sender] == 0
                || _stakedBalances[msg.sender] - amount >= _seedBalances[msg.sender],
            "cannot unstake seed"
        );
        _stakedBalances[msg.sender] -= amount;
        _transfer(address(this), msg.sender, amount);
        emit Unstaked(msg.sender, amount);
    }

    /// @notice Get staked balance for an account
    function stakedBalanceOf(address account) public view returns (uint256) {
        return _stakedBalances[account];
    }

    // ─── Activity Tracking ────────────────────────────────────────────────

    /// @notice Update activity timestamp for an account
    function updateActivity(address account) public {
        _lastActivity[account] = block.timestamp;
        emit ActivityUpdated(account, block.timestamp);
    }

    /// @notice Get last activity timestamp for an account
    function lastActivity(address account) public view returns (uint256) {
        return _lastActivity[account];
    }

    /// @notice Get activity level: 0=Active, 1=Moderate, 2=Low, 3=Inactive, 4=Dead
    function getActivityLevel(address account) public view returns (uint8) {
        uint256 last = _lastActivity[account];
        if (last == 0) return 4; // Never active = Dead
        uint256 daysSince = (block.timestamp - last) / 1 days;
        if (daysSince <= 1) return 0; // Active
        if (daysSince <= 7) return 1; // Moderate
        if (daysSince <= 30) return 2; // Low
        if (daysSince <= 90) return 3; // Inactive
        return 4; // Dead
    }

    // ─── Decay ────────────────────────────────────────────────────────────

    /// @notice Set decay distribution addresses (GOVERNANCE_ROLE only)
    function setDecayConfig(address treasury, address stakingRewards, address devFund)
        external
        onlyRole(GOVERNANCE_ROLE)
    {
        decayConfig = DecayConfig(treasury, stakingRewards, devFund);
    }

    /// @notice Compute decay amount for an account (view-only)
    function computeDecay(address account) public view returns (uint256) {
        uint8 level = getActivityLevel(account);
        if (level == 0) return 0; // Active = no decay
        uint256 bal = balanceOf(account);
        if (bal == 0) return 0;
        uint256 rate = decayRatesBps[level];
        return (bal * rate) / 10000;
    }

    /// @notice Execute decay on an account. Anyone can call (keeper pattern).
    function executeDecay(address account) external {
        require(block.timestamp >= _lastDecayTimestamp[account] + 1 days, "too soon");
        uint256 decayAmount = computeDecay(account);
        if (decayAmount == 0) return;

        uint256 treasuryPortion = (decayAmount * 40) / 100;
        uint256 stakingPortion = (decayAmount * 30) / 100;
        uint256 burnPortion = (decayAmount * 20) / 100;
        uint256 devPortion = decayAmount - treasuryPortion - stakingPortion - burnPortion;

        _transfer(account, decayConfig.treasury, treasuryPortion);
        _transfer(account, decayConfig.stakingRewards, stakingPortion);
        _burn(account, burnPortion);
        _transfer(account, decayConfig.devFund, devPortion);

        _lastDecayTimestamp[account] = block.timestamp;
        emit DecayExecuted(account, decayAmount, treasuryPortion, stakingPortion, burnPortion, devPortion);
    }

    // ─── Seed Tokens ──────────────────────────────────────────────────────

    /// @notice Mint seed tokens to an agent. Tokens are staked and locked.
    function mintSeed(address to, uint256 amount) external onlyRole(MINTER_ROLE) {
        _mint(address(this), amount);
        _stakedBalances[to] += amount;
        _seedBalances[to] += amount;
        emit Staked(to, amount);
        emit SeedMinted(to, amount);
    }

    /// @notice Activate seed tokens — removes the lock so they can be unstaked
    function activateSeed(address account) external onlyRole(GOVERNANCE_ROLE) {
        _seedBalances[account] = 0;
        emit SeedActivated(account);
    }

    /// @notice Get seed token balance for an account
    function seedBalanceOf(address account) public view returns (uint256) {
        return _seedBalances[account];
    }

    // ─── Transfer Override (auto-track activity) ─────────────────────────

    function _update(address from, address to, uint256 amount) internal override {
        if (from != address(0)) updateActivity(from);
        if (to != address(0)) updateActivity(to);
        super._update(from, to, amount);
    }
}
