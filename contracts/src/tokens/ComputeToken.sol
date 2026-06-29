// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import "./NeunodeToken.sol";

/// @title ComputeToken — nCompute, backed by GPU/CPU hours
contract ComputeToken is NeunodeToken {
    constructor() NeunodeToken("Neunode Compute", "nCompute", 18) {}
}
