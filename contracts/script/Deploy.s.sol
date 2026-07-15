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
import "../src/reputation/NeunodeReputation.sol";
import "../src/slashing/NeunodeSlashing.sol";
import "../src/escrow/StakingEscrow.sol";
import "../src/exchange/ResourceAMM.sol";
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
    ResourceAMM public resourceAmm;

    // ─── Core ────────────────────────────────────────────────────────────
    NeunodeIdentity public identity;
    NeunodeRegistry public registry;
    NeunodeReputation public reputation;
    NeunodeSlashing public slashing;
    StakingEscrow public stakingEscrow;

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
    uint256 constant MIN_REGISTRATION_STAKE = 100e18;
    uint256 constant STAKE_FACTOR_TARGET = 10_000e18;
    uint256 constant AMM_PAIR_SEED = 1_000_000e18;

    function run() external {
        uint256 pk = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(pk);
        vm.startBroadcast(pk);

        // ── 1. Deploy 4 resource-backed tokens ──────────────────────────
        computeToken = new ComputeToken();
        trainingToken = new TrainingToken();
        bandwidthToken = new BandwidthToken();
        storageToken = new StorageToken();

        // ── 2. Seed all six resource-token markets from treasury ────────
        address[4] memory resourceTokens = [
            address(computeToken),
            address(trainingToken),
            address(bandwidthToken),
            address(storageToken)
        ];
        resourceAmm = new ResourceAMM(resourceTokens, deployer);
        _seedResourceMarkets(deployer);

        // ── 3. Deploy identity (no constructor args) ────────────────────
        identity = new NeunodeIdentity();

        // ── 4. Deploy registry (needs identity address) ─────────────────
        registry = new NeunodeRegistry(address(identity));

        // ── 5. Deploy reputation, staking, and slashing ─────────────────
        reputation = new NeunodeReputation();
        stakingEscrow = new StakingEscrow(address(computeToken));
        slashing = new NeunodeSlashing(address(computeToken));

        identity.setStakeSource(address(computeToken));
        identity.setMinRegistrationStake(MIN_REGISTRATION_STAKE);
        reputation.setIdentityRegistry(address(identity));
        reputation.setStakeSource(address(computeToken));
        reputation.setStakeFactorTarget(STAKE_FACTOR_TARGET);
        slashing.setReputationContract(address(reputation));

        computeToken.grantRole(computeToken.GOVERNANCE_ROLE(), address(stakingEscrow));
        computeToken.grantRole(computeToken.GOVERNANCE_ROLE(), address(slashing));
        reputation.grantRole(reputation.SLASHING_ROLE(), address(slashing));

        // ── 6. Deploy bounty system ─────────────────────────────────────
        bounty = new NeunodeBounty();
        escrow = new NeunodeEscrow();
        review = new BountyReview();

        // Wire bounty ↔ escrow + review
        bounty.setEscrow(address(escrow));
        bounty.setReviewContract(address(review));
        escrow.registerBountyContract(address(bounty));
        review.grantRole(review.DEFAULT_ADMIN_ROLE(), address(bounty));

        // ── 7. Deploy royalty system ────────────────────────────────────
        modelRegistry = new ModelRegistry();
        royaltySplitter = new RoyaltySplitter(address(modelRegistry));

        // ── 8. Deploy governance (uses nCompute for voting) ─────────────
        governance = new NeunodeGovernance(
            address(computeToken), // voting token
            VOTING_DELAY,
            VOTING_PERIOD,
            PROPOSAL_THRESHOLD,
            QUORUM_BPS,
            TIMELOCK,
            EXECUTION_WINDOW
        );

        _wireGovernance(deployer);

        // ── 9. Deploy diamond proxy with loupe + cut facets ─────────────
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
        governance.setAllowedTarget(address(diamond), true);

        vm.stopBroadcast();

        // ── Log deployed addresses ──────────────────────────────────────
        console.log("=== Tokens ===");
        console.log("ComputeToken:", address(computeToken));
        console.log("TrainingToken:", address(trainingToken));
        console.log("BandwidthToken:", address(bandwidthToken));
        console.log("StorageToken:", address(storageToken));
        console.log("ResourceAMM:", address(resourceAmm));
        console.log("");
        console.log("=== Core ===");
        console.log("NeunodeIdentity:", address(identity));
        console.log("NeunodeRegistry:", address(registry));
        console.log("NeunodeReputation:", address(reputation));
        console.log("NeunodeSlashing:", address(slashing));
        console.log("StakingEscrow:", address(stakingEscrow));
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

    function _wireGovernance(address oracle) internal {
        address governanceAddress = address(governance);

        governance.grantRole(governance.DEFAULT_ADMIN_ROLE(), governanceAddress);
        governance.grantRole(governance.GOVERNANCE_ROLE(), governanceAddress);

        computeToken.grantRole(computeToken.DEFAULT_ADMIN_ROLE(), governanceAddress);
        computeToken.grantRole(computeToken.GOVERNANCE_ROLE(), governanceAddress);
        trainingToken.grantRole(trainingToken.DEFAULT_ADMIN_ROLE(), governanceAddress);
        trainingToken.grantRole(trainingToken.GOVERNANCE_ROLE(), governanceAddress);
        bandwidthToken.grantRole(bandwidthToken.DEFAULT_ADMIN_ROLE(), governanceAddress);
        bandwidthToken.grantRole(bandwidthToken.GOVERNANCE_ROLE(), governanceAddress);
        storageToken.grantRole(storageToken.DEFAULT_ADMIN_ROLE(), governanceAddress);
        storageToken.grantRole(storageToken.GOVERNANCE_ROLE(), governanceAddress);
        resourceAmm.grantRole(resourceAmm.DEFAULT_ADMIN_ROLE(), governanceAddress);
        resourceAmm.grantRole(resourceAmm.TREASURY_ROLE(), governanceAddress);

        bounty.grantRole(bounty.DEFAULT_ADMIN_ROLE(), governanceAddress);
        bounty.grantRole(bounty.ADMIN_ROLE(), governanceAddress);
        escrow.grantRole(escrow.DEFAULT_ADMIN_ROLE(), governanceAddress);
        escrow.grantRole(escrow.ESCROW_ADMIN_ROLE(), governanceAddress);
        review.grantRole(review.DEFAULT_ADMIN_ROLE(), governanceAddress);
        modelRegistry.grantRole(modelRegistry.DEFAULT_ADMIN_ROLE(), governanceAddress);
        modelRegistry.grantRole(modelRegistry.REGISTRAR_ROLE(), governanceAddress);
        royaltySplitter.grantRole(royaltySplitter.DEFAULT_ADMIN_ROLE(), governanceAddress);
        royaltySplitter.grantRole(royaltySplitter.ADMIN_ROLE(), governanceAddress);
        reputation.grantRole(reputation.DEFAULT_ADMIN_ROLE(), governanceAddress);
        reputation.grantRole(reputation.REPUTATION_ADMIN_ROLE(), governanceAddress);
        reputation.grantRole(reputation.EPOCH_FINALIZER_ROLE(), governanceAddress);
        reputation.grantRole(reputation.STAKE_ORACLE_ROLE(), oracle);
        reputation.grantRole(reputation.ATTEST_ORACLE_ROLE(), oracle);
        reputation.grantRole(reputation.ACTIVITY_ORACLE_ROLE(), oracle);
        reputation.grantRole(reputation.VERIFY_ORACLE_ROLE(), oracle);
        reputation.grantRole(reputation.TENURE_ORACLE_ROLE(), oracle);
        slashing.grantRole(slashing.DEFAULT_ADMIN_ROLE(), governanceAddress);
        slashing.grantRole(slashing.ADMIN_ROLE(), governanceAddress);
        slashing.grantRole(slashing.REPORTER_ROLE(), oracle);
        stakingEscrow.grantRole(stakingEscrow.DEFAULT_ADMIN_ROLE(), governanceAddress);
        stakingEscrow.grantRole(stakingEscrow.DECAY_ADMIN_ROLE(), governanceAddress);

        identity.transferOwnership(governanceAddress);

        governance.setAllowedTarget(governanceAddress, true);
        governance.setAllowedTarget(address(computeToken), true);
        governance.setAllowedTarget(address(trainingToken), true);
        governance.setAllowedTarget(address(bandwidthToken), true);
        governance.setAllowedTarget(address(storageToken), true);
        governance.setAllowedTarget(address(resourceAmm), true);
        governance.setAllowedTarget(address(identity), true);
        governance.setAllowedTarget(address(bounty), true);
        governance.setAllowedTarget(address(escrow), true);
        governance.setAllowedTarget(address(review), true);
        governance.setAllowedTarget(address(modelRegistry), true);
        governance.setAllowedTarget(address(royaltySplitter), true);
        governance.setAllowedTarget(address(reputation), true);
        governance.setAllowedTarget(address(slashing), true);
        governance.setAllowedTarget(address(stakingEscrow), true);
    }

    function _seedResourceMarkets(address treasury) internal {
        uint256 treasuryAmount = AMM_PAIR_SEED * 3;
        computeToken.mint(treasury, treasuryAmount);
        trainingToken.mint(treasury, treasuryAmount);
        bandwidthToken.mint(treasury, treasuryAmount);
        storageToken.mint(treasury, treasuryAmount);

        computeToken.approve(address(resourceAmm), treasuryAmount);
        trainingToken.approve(address(resourceAmm), treasuryAmount);
        bandwidthToken.approve(address(resourceAmm), treasuryAmount);
        storageToken.approve(address(resourceAmm), treasuryAmount);

        resourceAmm.seedPool(
            address(computeToken), address(trainingToken), AMM_PAIR_SEED, AMM_PAIR_SEED
        );
        resourceAmm.seedPool(
            address(computeToken), address(bandwidthToken), AMM_PAIR_SEED, AMM_PAIR_SEED
        );
        resourceAmm.seedPool(
            address(computeToken), address(storageToken), AMM_PAIR_SEED, AMM_PAIR_SEED
        );
        resourceAmm.seedPool(
            address(trainingToken), address(bandwidthToken), AMM_PAIR_SEED, AMM_PAIR_SEED
        );
        resourceAmm.seedPool(
            address(trainingToken), address(storageToken), AMM_PAIR_SEED, AMM_PAIR_SEED
        );
        resourceAmm.seedPool(
            address(bandwidthToken), address(storageToken), AMM_PAIR_SEED, AMM_PAIR_SEED
        );
    }
}
