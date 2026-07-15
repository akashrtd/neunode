// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import "./NeunodeToken.sol";

/// @title StorageToken — nStorage, backed by disk space
contract StorageToken is NeunodeToken {
    constructor()
        NeunodeToken("Neunode Storage", "nStorage", 18, 1_000_000_000e18, 10_000_000_000e18)
    {}
}
