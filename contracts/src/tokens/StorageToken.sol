// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "./NeunodeToken.sol";

/// @title StorageToken — nStorage, backed by disk space
contract StorageToken is NeunodeToken {
    constructor() NeunodeToken("Neunode Storage", "nStorage", 18) {}
}
