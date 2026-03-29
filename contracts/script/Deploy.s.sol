// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../src/tokens/ComputeToken.sol";
import "../src/tokens/TrainingToken.sol";
import "../src/tokens/BandwidthToken.sol";
import "../src/tokens/StorageToken.sol";
import "../src/NeunodeIdentity.sol";
import "../src/NeunodeRegistry.sol";
import "../src/NeunodeEscrow.sol";
import "../src/NeunodeBounty.sol";

/// @title Deploy — Phase 1 deployment script for Neunode contracts
contract Deploy is Script {
    // Deployed contract addresses
    ComputeToken public computeToken;
    TrainingToken public trainingToken;
    BandwidthToken public bandwidthToken;
    StorageToken public storageToken;
    NeunodeIdentity public identity;
    NeunodeRegistry public registry;
    NeunodeEscrow public escrow;
    NeunodeBounty public bounty;

    function run() external {
        uint256 pk = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(pk);

        // Deploy 4 resource-backed tokens
        computeToken = new ComputeToken();
        trainingToken = new TrainingToken();
        bandwidthToken = new BandwidthToken();
        storageToken = new StorageToken();

        // Deploy identity (no constructor args)
        identity = new NeunodeIdentity();

        // Deploy registry (needs identity address)
        registry = new NeunodeRegistry(address(identity));

        // Deploy escrow (standalone)
        escrow = new NeunodeEscrow();

        // Deploy bounty (standalone)
        bounty = new NeunodeBounty();

        vm.stopBroadcast();

        // Log deployed addresses
        console.log("ComputeToken:", address(computeToken));
        console.log("TrainingToken:", address(trainingToken));
        console.log("BandwidthToken:", address(bandwidthToken));
        console.log("StorageToken:", address(storageToken));
        console.log("NeunodeIdentity:", address(identity));
        console.log("NeunodeRegistry:", address(registry));
        console.log("NeunodeEscrow:", address(escrow));
        console.log("NeunodeBounty:", address(bounty));
    }
}
