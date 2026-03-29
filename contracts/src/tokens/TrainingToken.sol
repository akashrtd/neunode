// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "./NeunodeToken.sol";

/// @title TrainingToken — nTrain, backed by training units
contract TrainingToken is NeunodeToken {
    constructor() NeunodeToken("Neunode Training", "nTrain", 18) {}
}
