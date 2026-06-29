// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import "./NeunodeToken.sol";

/// @title BandwidthToken — nBandwidth, backed by transfer volume
contract BandwidthToken is NeunodeToken {
    constructor() NeunodeToken("Neunode Bandwidth", "nBandwidth", 18) {}
}
