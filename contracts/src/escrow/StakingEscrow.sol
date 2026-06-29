// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/AccessControl.sol";
import "../interfaces/INeunodeToken.sol";

/// @title StakingEscrow — Inactivity Decay Manager
/// @notice Moves the inactivity decay logic out of the ERC-20 token interface
///         so that standard composability works. Calculates decay on the staked
///         balance and slashes it via the token's slashStake method.
contract StakingEscrow is AccessControl {
    error DecayTooSoon(address account);

    bytes32 public constant DECAY_ADMIN_ROLE = keccak256("DECAY_ADMIN_ROLE");

    INeunodeToken public immutable neunodeToken;
    mapping(address => uint256) private _lastDecayTimestamp;

    // Decay rates in basis points per day per activity level.
    // Index: 0=Active, 1=Moderate, 2=Low, 3=Inactive, 4=Dead
    // Active=0, Moderate=5 (~2%/yr), Low=14 (~5%/yr), Inactive=41 (~15%/yr), Dead=137 (~50%/yr)
    uint256[5] public decayRatesBps = [uint256(0), 5, 14, 41, 137];

    event DecayExecuted(address indexed account, uint256 slashedAmount);

    constructor(address tokenAddress) {
        neunodeToken = INeunodeToken(tokenAddress);
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(DECAY_ADMIN_ROLE, msg.sender);
    }

    /// @notice Compute the decay amount (slashing amount) for a given account's staked balance.
    /// @param account The address whose staked balance to calculate decay for.
    /// @return The amount of tokens to slash, in the token's smallest unit.
    function computeDecay(address account) public view returns (uint256) {
        uint8 level = neunodeToken.getActivityLevel(account);
        if (level == 0) return 0; // Active = no decay

        uint256 stakedBal = neunodeToken.stakedBalanceOf(account);
        if (stakedBal == 0) return 0;

        uint256 rate = decayRatesBps[level];
        return (stakedBal * rate) / 10000;
    }

    /// @notice Apply decay to an account. Anyone can trigger this on inactive nodes.
    /// @param account The address to apply inactivity decay to.
    /// @dev Reverts if less than 1 day since last decay. Slashes staked balance via the token contract.
    function executeDecay(address account) external {
        if (block.timestamp < _lastDecayTimestamp[account] + 1 days) revert DecayTooSoon(account);

        uint256 decayAmount = computeDecay(account);
        if (decayAmount == 0) {
            _lastDecayTimestamp[account] = block.timestamp;
            return;
        }

        // Apply penalty by slashing their stake
        neunodeToken.slashStake(account, decayAmount);
        _lastDecayTimestamp[account] = block.timestamp;

        emit DecayExecuted(account, decayAmount);
    }
}
