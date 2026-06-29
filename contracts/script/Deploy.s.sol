// SPDX-License-Identifier: AGPL-3.0-or-later
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
import "../src/bounty/BountyReview.sol";
import "../src/royalty/ModelRegistry.sol";
import "../src/royalty/RoyaltySplitter.sol";
import "../src/governance/NeunodeGovernance.sol";
import "../src/diamond/Diamond.sol";
import "../src/diamond/DiamondCutFacet.sol";
import "../src/diamond/DiamondLoupeFacet.sol";

/// @title Deploy — Phase 1 deployment script for Neunode contracts
/// @notice Deploys all contracts: tokens, identity, registry, bounty+escrow+review,
///         royalty system, governance, and diamond proxy with loupe+cut facets.
contract Deploy is Script {
    // ─── Tokens ──────────────────────────────────────────────────────────
    ComputeToken public computeToken;
    TrainingToken public trainingToken;
    BandwidthToken public bandwidthToken;
    StorageToken public storageToken;

    // ─── Core ────────────────────────────────────────────────────────────
    NeunodeIdentity public identity;
    NeunodeRegistry public registry;

    // ─── Bounty System ───────────────────────────────────────────────────
    NeunodeBounty public bounty;
    NeunodeEscrow public escrow;
    BountyReview public review;

    // ─── Royalty System ──────────────────────────────────────────────────
    ModelRegistry public modelRegistry;
    RoyaltySplitter public royaltySplitter;

    // ─── Governance ──────────────────────────────────────────────────────
    NeunodeGovernance public governance;

    // ─── Diamond Proxy ───────────────────────────────────────────────────
    DiamondCutFacet public diamondCutFacet;
    DiamondLoupeFacet public diamondLoupeFacet;
    Diamond public diamond;

    // ─── Governance defaults ─────────────────────────────────────────────
    uint256 constant VOTING_DELAY = 1 days;
    uint256 constant VOTING_PERIOD = 7 days;
    uint256 constant PROPOSAL_THRESHOLD = 100e18; // 100 nCompute
    uint256 constant QUORUM_BPS = 400; // 4%
    uint256 constant TIMELOCK = 2 days;
    uint256 constant EXECUTION_WINDOW = 14 days;

    function run() external {
        uint256 pk = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(pk);

        // ── 1. Deploy 4 resource-backed tokens ──────────────────────────
        computeToken = new ComputeToken();
        trainingToken = new TrainingToken();
        bandwidthToken = new BandwidthToken();
        storageToken = new StorageToken();

        // ── 2. Deploy identity (no constructor args) ────────────────────
        identity = new NeunodeIdentity();

        // ── 3. Deploy registry (needs identity address) ─────────────────
        registry = new NeunodeRegistry(address(identity));

        // ── 4. Deploy bounty system ─────────────────────────────────────
        bounty = new NeunodeBounty();
        escrow = new NeunodeEscrow();
        review = new BountyReview();

        // Wire bounty ↔ escrow + review
        bounty.setEscrow(address(escrow));
        bounty.setReviewContract(address(review));
        escrow.registerBountyContract(address(bounty));

        // ── 5. Deploy royalty system ────────────────────────────────────
        modelRegistry = new ModelRegistry();
        royaltySplitter = new RoyaltySplitter(address(modelRegistry));

        // ── 6. Deploy governance (uses nCompute for voting) ─────────────
        governance = new NeunodeGovernance(
            address(computeToken), // voting token
            VOTING_DELAY,
            VOTING_PERIOD,
            PROPOSAL_THRESHOLD,
            QUORUM_BPS,
            TIMELOCK,
            EXECUTION_WINDOW
        );

        // ── 7. Deploy diamond proxy with loupe + cut facets ─────────────
        diamondCutFacet = new DiamondCutFacet();
        diamondLoupeFacet = new DiamondLoupeFacet();

        IDiamondCut.FacetCut[] memory cuts = new IDiamondCut.FacetCut[](2);

        // Add DiamondCutFacet selectors
        bytes4[] memory cutSelectors = new bytes4[](1);
        cutSelectors[0] = IDiamondCut.diamondCut.selector;
        cuts[0] = IDiamondCut.FacetCut({
            facetAddress: address(diamondCutFacet),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: cutSelectors
        });

        // Add DiamondLoupeFacet selectors
        bytes4[] memory loupeSelectors = new bytes4[](4);
        loupeSelectors[0] = IDiamondLoupe.facets.selector;
        loupeSelectors[1] = IDiamondLoupe.facetFunctionSelectors.selector;
        loupeSelectors[2] = IDiamondLoupe.facetAddresses.selector;
        loupeSelectors[3] = IDiamondLoupe.facetAddress.selector;
        cuts[1] = IDiamondCut.FacetCut({
            facetAddress: address(diamondLoupeFacet),
            action: IDiamondCut.FacetCutAction.Add,
            functionSelectors: loupeSelectors
        });

        diamond = new Diamond(cuts, address(0), "", msg.sender);

        vm.stopBroadcast();

        // ── Log deployed addresses ──────────────────────────────────────
        console.log("=== Tokens ===");
        console.log("ComputeToken:", address(computeToken));
        console.log("TrainingToken:", address(trainingToken));
        console.log("BandwidthToken:", address(bandwidthToken));
        console.log("StorageToken:", address(storageToken));
        console.log("");
        console.log("=== Core ===");
        console.log("NeunodeIdentity:", address(identity));
        console.log("NeunodeRegistry:", address(registry));
        console.log("");
        console.log("=== Bounty System ===");
        console.log("NeunodeBounty:", address(bounty));
        console.log("NeunodeEscrow:", address(escrow));
        console.log("BountyReview:", address(review));
        console.log("");
        console.log("=== Royalty System ===");
        console.log("ModelRegistry:", address(modelRegistry));
        console.log("RoyaltySplitter:", address(royaltySplitter));
        console.log("");
        console.log("=== Governance ===");
        console.log("NeunodeGovernance:", address(governance));
        console.log("");
        console.log("=== Diamond Proxy ===");
        console.log("Diamond:", address(diamond));
        console.log("DiamondCutFacet:", address(diamondCutFacet));
        console.log("DiamondLoupeFacet:", address(diamondLoupeFacet));
    }
}
