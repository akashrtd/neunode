// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import "./NeunodeToken.sol";

/// @title TrainingToken — nTrain, backed by training units
contract TrainingToken is NeunodeToken {
    constructor()
        NeunodeToken("Neunode Training", "nTrain", 18, 1_000_000_000e18, 10_000_000_000e18)
    {}
}
