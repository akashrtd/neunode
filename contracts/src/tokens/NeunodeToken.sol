// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/// @title NeunodeToken — Base ERC-20 for resource-backed tokens
/// @notice Abstract base with mint/burn, custom decimals. Owner is the protocol.
abstract contract NeunodeToken is ERC20, Ownable {
    uint8 private immutable _tokenDecimals;

    constructor(string memory name, string memory symbol, uint8 decimals_)
        ERC20(name, symbol)
        Ownable(msg.sender)
    {
        _tokenDecimals = decimals_;
    }

    function decimals() public view override returns (uint8) {
        return _tokenDecimals;
    }

    /// @notice Mint tokens to an address (protocol only)
    function mint(address to, uint256 amount) external onlyOwner {
        _mint(to, amount);
    }

    /// @notice Burn tokens from an address (protocol only)
    function burn(address from, uint256 amount) external onlyOwner {
        _burn(from, amount);
    }
}
