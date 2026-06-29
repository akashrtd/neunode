// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../../src/royalty/ModelRegistry.sol";
import "../../src/royalty/RoyaltySplitter.sol";
import "../../src/royalty/IRoyaltySplitter.sol";
import "../../src/tokens/ComputeToken.sol";

/// @title RoyaltySystemTest — Comprehensive tests for ModelRegistry + RoyaltySplitter
contract RoyaltySystemTest is Test {
    ModelRegistry public registry;
    RoyaltySplitter public splitter;
    ComputeToken public token;

    address public admin;
    address public contributor1;
    address public contributor2;
    address public contributor3;
    address public contributor4;
    address public outsider;

    // Model CIDs (content hashes)
    bytes32 constant ROOT_MODEL = keccak256("root_model_v1");
    bytes32 constant FINETUNE_MODEL = keccak256("finetune_medical_v1");
    bytes32 constant RL_MODEL = keccak256("rl_alignment_v1");
    bytes32 constant SERVING_MODEL = keccak256("serving_deployed_v1");
    bytes32 constant MERGE_MODEL = keccak256("merge_two_parents_v1");

    uint256 constant DISTRIBUTION_AMOUNT = 10_000e18;

    // Cached role constants (avoid consuming vm.prank with view calls)
    bytes32 public registrarRole;
    bytes32 public splitterAdminRole;

    function setUp() public {
        admin = address(this);
        contributor1 = makeAddr("contributor1");
        contributor2 = makeAddr("contributor2");
        contributor3 = makeAddr("contributor3");
        contributor4 = makeAddr("contributor4");
        outsider = makeAddr("outsider");

        registry = new ModelRegistry();
        splitter = new RoyaltySplitter(address(registry));
        token = new ComputeToken();

        // Cache role constants BEFORE any prank usage
        registrarRole = registry.REGISTRAR_ROLE();
        splitterAdminRole = splitter.ADMIN_ROLE();

        // Mint tokens to admin for distributions
        token.mint(admin, 1_000_000e18);
        token.approve(address(splitter), type(uint256).max);
    }

    // ─── Helper: Register model as admin ──────────────────────────────────

    function _register(bytes32 cid, bytes32[] memory parents, IModelRegistry.ContributionType ctype)
        internal
    {
        bytes32 proof =
            parents.length > 0 ? keccak256(abi.encodePacked(cid, parents[0])) : bytes32(0);
        registry.registerModel(cid, parents, ctype, "ipfs://metadata", proof);
    }

    function _emptyParents() internal pure returns (bytes32[] memory) {
        return new bytes32[](0);
    }

    function _singleParent(bytes32 p) internal pure returns (bytes32[] memory) {
        bytes32[] memory parents = new bytes32[](1);
        parents[0] = p;
        return parents;
    }

    function _twoParents(bytes32 p1, bytes32 p2) internal pure returns (bytes32[] memory) {
        bytes32[] memory parents = new bytes32[](2);
        parents[0] = p1;
        parents[1] = p2;
        return parents;
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ModelRegistry Tests
    // ═══════════════════════════════════════════════════════════════════════

    // ─── 1. Register single model (no parents) ────────────────────────────

    function testRegisterRootModel() public {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);

        IModelRegistry.ModelInfo memory info = registry.getModel(ROOT_MODEL);
        assertTrue(info.exists);
        assertEq(info.cid, ROOT_MODEL);
        assertEq(info.contributor, admin);
        assertEq(uint8(info.contribution), uint8(IModelRegistry.ContributionType.PreTraining));
        assertEq(info.metadataURI, "ipfs://metadata");
        assertGt(info.registeredAt, 0);
    }

    // ─── 2. Register model with 1 parent → verify parent-child link ──────

    function testRegisterModelWithOneParent() public {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        _register(
            FINETUNE_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.FineTune
        );

        // Check parent link
        bytes32[] memory parents = registry.getParents(FINETUNE_MODEL);
        assertEq(parents.length, 1);
        assertEq(parents[0], ROOT_MODEL);

        // Check child link
        bytes32[] memory children = registry.getChildren(ROOT_MODEL);
        assertEq(children.length, 1);
        assertEq(children[0], FINETUNE_MODEL);
    }

    // ─── 3. Register model with multiple parents ──────────────────────────

    function testRegisterModelWithMultipleParents() public {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        _register(
            FINETUNE_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.FineTune
        );
        _register(RL_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.RL);
        _register(
            MERGE_MODEL,
            _twoParents(FINETUNE_MODEL, RL_MODEL),
            IModelRegistry.ContributionType.FineTune
        );

        bytes32[] memory parents = registry.getParents(MERGE_MODEL);
        assertEq(parents.length, 2);
        assertEq(parents[0], FINETUNE_MODEL);
        assertEq(parents[1], RL_MODEL);
    }

    // ─── 4. Multi-generation lineage (3+ levels) → getLineageDepth ────────

    function testMultiGenerationLineageDepth() public {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        _register(
            FINETUNE_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.FineTune
        );
        _register(RL_MODEL, _singleParent(FINETUNE_MODEL), IModelRegistry.ContributionType.RL);
        _register(SERVING_MODEL, _singleParent(RL_MODEL), IModelRegistry.ContributionType.Serving);

        // Root has depth 0 (no parents)
        assertEq(registry.getLineageDepth(ROOT_MODEL), 0);
        // FineTune is 1 hop from root
        assertEq(registry.getLineageDepth(FINETUNE_MODEL), 1);
        // RL is 2 hops from root
        assertEq(registry.getLineageDepth(RL_MODEL), 2);
        // Serving is 3 hops from root
        assertEq(registry.getLineageDepth(SERVING_MODEL), 3);
    }

    // ─── 5. Cannot register same CID twice ────────────────────────────────

    function testRevertRegisterDuplicateCid() public {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);

        vm.expectRevert(
            abi.encodeWithSelector(ModelRegistry.ModelAlreadyExists.selector, ROOT_MODEL)
        );
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
    }

    // ─── 6. Only REGISTRAR_ROLE can register ──────────────────────────────

    function testRevertNonRegistrarRegister() public {
        vm.prank(outsider);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, outsider, registrarRole
            )
        );
        registry.registerModel(
            ROOT_MODEL,
            _emptyParents(),
            IModelRegistry.ContributionType.PreTraining,
            "ipfs://metadata",
            bytes32(0)
        );
    }

    // ─── 7. Cannot register with zero CID ─────────────────────────────────

    function testRevertInvalidCid() public {
        vm.expectRevert(abi.encodeWithSelector(ModelRegistry.InvalidCid.selector, bytes32(0)));
        _register(bytes32(0), _emptyParents(), IModelRegistry.ContributionType.PreTraining);
    }

    // ─── 8. Cannot register with non-existent parent ──────────────────────

    function testRevertParentNotFound() public {
        bytes32 ghost = keccak256("nonexistent");
        vm.expectRevert(abi.encodeWithSelector(ModelRegistry.ParentNotFound.selector, ghost));
        _register(FINETUNE_MODEL, _singleParent(ghost), IModelRegistry.ContributionType.FineTune);
    }

    // ─── 9. getModel reverts on non-existent model ────────────────────────

    function testRevertGetModelNotFound() public {
        bytes32 ghost = keccak256("ghost");
        vm.expectRevert(abi.encodeWithSelector(ModelRegistry.ModelNotFound.selector, ghost));
        registry.getModel(ghost);
    }

    // ─── 10. getChildren for leaf model returns empty ─────────────────────

    function testLeafModelHasNoChildren() public {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        _register(
            FINETUNE_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.FineTune
        );

        bytes32[] memory children = registry.getChildren(FINETUNE_MODEL);
        assertEq(children.length, 0);
    }

    // ─── 11. getModelCount tracks total ───────────────────────────────────

    function testGetModelCount() public {
        assertEq(registry.getModelCount(), 0);

        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        assertEq(registry.getModelCount(), 1);

        _register(
            FINETUNE_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.FineTune
        );
        assertEq(registry.getModelCount(), 2);
    }

    // ─── 12. modelExists returns correct boolean ──────────────────────────

    function testModelExists() public {
        assertFalse(registry.modelExists(ROOT_MODEL));

        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        assertTrue(registry.modelExists(ROOT_MODEL));
    }

    // ─── 13. Events emitted on registration ───────────────────────────────

    function testEventsEmitted() public {
        vm.expectEmit(true, true, false, true);
        emit IModelRegistry.ModelRegistered(
            ROOT_MODEL, admin, IModelRegistry.ContributionType.PreTraining, _emptyParents()
        );
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);

        // Register child and check both events
        vm.expectEmit(true, true, false, true);
        emit IModelRegistry.ModelRegistered(
            FINETUNE_MODEL,
            admin,
            IModelRegistry.ContributionType.FineTune,
            _singleParent(ROOT_MODEL)
        );
        vm.expectEmit(true, true, true, true);
        emit IModelRegistry.LineageExtended(ROOT_MODEL, FINETUNE_MODEL, admin);
        _register(
            FINETUNE_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.FineTune
        );
    }

    // ─── 14. Grant and revoke REGISTRAR_ROLE ──────────────────────────────

    function testGrantRegistrarRole() public {
        registry.grantRole(registrarRole, contributor1);

        vm.prank(contributor1);
        registry.registerModel(
            keccak256("contrib1_model"),
            _emptyParents(),
            IModelRegistry.ContributionType.Data,
            "ipfs://meta",
            bytes32(0)
        );

        assertTrue(registry.modelExists(keccak256("contrib1_model")));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // RoyaltySplitter Tests
    // ═══════════════════════════════════════════════════════════════════════

    // ─── Setup helper for royalty tests ───────────────────────────────────

    function _setupSimpleLineage() internal {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        _register(SERVING_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.Serving);
    }

    function _setupMultiParentLineage() internal {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        _register(
            FINETUNE_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.FineTune
        );
        _register(RL_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.RL);
        _register(
            MERGE_MODEL,
            _twoParents(FINETUNE_MODEL, RL_MODEL),
            IModelRegistry.ContributionType.Serving
        );
    }

    function _setupDeepLineage() internal {
        // 4-level chain: root → fineTune → RL → serving
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        _register(
            FINETUNE_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.FineTune
        );
        _register(RL_MODEL, _singleParent(FINETUNE_MODEL), IModelRegistry.ContributionType.RL);
        _register(SERVING_MODEL, _singleParent(RL_MODEL), IModelRegistry.ContributionType.Serving);
    }

    // ─── 15. Simple royalty: 1 parent → child, verify single recipient ────

    function testSimpleRoyaltyDistribution() public {
        _setupSimpleLineage();

        // SERVING_MODEL has one ancestor: ROOT_MODEL at depth 1
        // Admin is contributor for both, so admin pays (transferFrom) and receives back
        uint256 adminBalBefore = token.balanceOf(admin);

        splitter.distributeRoyalties(SERVING_MODEL, DISTRIBUTION_AMOUNT, address(token));

        // Tokens transferred from admin to splitter, then all distributed back to admin
        // Net effect: admin balance ≈ same (transferFrom debit + distribution credit)
        uint256 adminBalAfter = token.balanceOf(admin);
        // For 1 recipient: share = amount * weight / weight = exact amount, so net change = 0
        assertEq(adminBalAfter, adminBalBefore);

        // Splitter should have 0 remaining
        assertEq(token.balanceOf(address(splitter)), 0);

        // Accumulated royalties should track
        assertEq(splitter.accumulatedRoyalties(SERVING_MODEL, address(token)), DISTRIBUTION_AMOUNT);
    }

    // ─── 16. Multi-parent royalty: verify proportional split ──────────────

    function testMultiParentRoyaltySplit() public {
        _setupMultiParentLineage();

        // MERGE_MODEL ancestors: FINETUNE(d=1), RL(d=1), ROOT(d=2 via FINETUNE path)
        uint256 adminBalBefore = token.balanceOf(admin);

        splitter.distributeRoyalties(MERGE_MODEL, DISTRIBUTION_AMOUNT, address(token));

        // All contributors are admin, so net ≈ 0 (minus rounding dust from split)
        uint256 adminBalAfter = token.balanceOf(admin);
        assertGe(adminBalBefore - adminBalAfter, 0);
        // Rounding dust: at most 2 wei lost per split (3 recipients with integer division)
        assertLe(adminBalBefore - adminBalAfter, 3);

        // Accumulated royalties should reflect the full amount
        assertEq(
            splitter.accumulatedRoyalties(MERGE_MODEL, address(token)),
            DISTRIBUTION_AMOUNT - (adminBalBefore - adminBalAfter)
        );
    }

    // ─── 17. Deep lineage royalty with recency decay ──────────────────────

    function testDeepLineageRoyaltyDecay() public {
        _setupDeepLineage();

        IRoyaltySplitter.RecipientInfo[] memory recipients = splitter.getRecipients(SERVING_MODEL);
        // 3 ancestors: RL(depth=1), FineTune(depth=2), Root(depth=3)
        assertEq(recipients.length, 3);

        // Verify depth ordering
        assertEq(recipients[0].depth, 1); // RL (first parent)
        assertEq(recipients[1].depth, 2); // FineTune
        assertEq(recipients[2].depth, 3); // Root

        // Verify weights reflect contribution type and decay
        // RL: 100 * 70 * 90 = 630000 (depth 1, decay=90)
        // FineTune: 100 * 80 * 81 = 648000 (depth 2, decay=81)
        // Root: 100 * 100 * 72 = 720000 (depth 3, decay=72)
        assertEq(recipients[0].weight, 100 * 70 * 90); // RL
        assertEq(recipients[1].weight, 100 * 80 * 81); // FineTune
        assertEq(recipients[2].weight, 100 * 100 * 72); // Root
    }

    // ─── 18. ERC-2981 compliance ──────────────────────────────────────────

    function testERC2981RoyaltyInfo() public {
        _setupSimpleLineage();

        (address receiver, uint256 royaltyAmount) =
            splitter.royaltyInfo(uint256(ROOT_MODEL), 10000e18);

        assertEq(receiver, admin); // default receiver
        // 10% of 10000 = 1000
        assertEq(royaltyAmount, 1000e18);
    }

    function testERC2981NonExistentModel() public view {
        (address receiver, uint256 royaltyAmount) =
            splitter.royaltyInfo(uint256(keccak256("ghost")), 10000e18);

        assertEq(receiver, address(0));
        assertEq(royaltyAmount, 0);
    }

    // ─── 19. Distribution with zero amount reverts ────────────────────────

    function testRevertZeroAmountDistribution() public {
        _setupSimpleLineage();
        vm.expectRevert(IRoyaltySplitter.ZeroAmount.selector);
        splitter.distributeRoyalties(SERVING_MODEL, 0, address(token));
    }

    // ─── 20. Distribution to non-existent model reverts ───────────────────

    function testRevertNonExistentModelDistribution() public {
        bytes32 ghost = keccak256("ghost");
        vm.expectRevert(abi.encodeWithSelector(IRoyaltySplitter.ModelNotFound.selector, ghost));
        splitter.distributeRoyalties(ghost, DISTRIBUTION_AMOUNT, address(token));
    }

    // ─── 21. Root model (no parents) → NoLineage revert ───────────────────

    function testRevertRootModelNoLineage() public {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);

        // Root model has no ancestors to distribute to
        vm.expectRevert(abi.encodeWithSelector(IRoyaltySplitter.NoLineage.selector, ROOT_MODEL));
        splitter.distributeRoyalties(ROOT_MODEL, DISTRIBUTION_AMOUNT, address(token));
    }

    // ─── 22. Get children of a parent model ───────────────────────────────

    function testGetChildrenOfParent() public {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        _register(
            FINETUNE_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.FineTune
        );
        _register(RL_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.RL);

        bytes32[] memory children = registry.getChildren(ROOT_MODEL);
        assertEq(children.length, 2);
        assertEq(children[0], FINETUNE_MODEL);
        assertEq(children[1], RL_MODEL);
    }

    // ─── 23. Contribution type weights affect distribution ────────────────

    function testContributionTypeWeights() public view {
        assertEq(splitter.getContributionTypeWeight(0), 100); // PreTraining
        assertEq(splitter.getContributionTypeWeight(1), 80); // FineTune
        assertEq(splitter.getContributionTypeWeight(2), 70); // RL
        assertEq(splitter.getContributionTypeWeight(3), 60); // Data
        assertEq(splitter.getContributionTypeWeight(4), 50); // Compute
        assertEq(splitter.getContributionTypeWeight(5), 30); // Serving
    }

    // ─── 24. Multiple distributions accumulate ────────────────────────────

    function testMultipleDistributionsAccumulate() public {
        _setupSimpleLineage();

        uint256 adminBalBefore = token.balanceOf(admin);

        splitter.distributeRoyalties(SERVING_MODEL, DISTRIBUTION_AMOUNT, address(token));
        splitter.distributeRoyalties(SERVING_MODEL, DISTRIBUTION_AMOUNT, address(token));

        // Admin pays and receives back — net ≈ 0 for single recipient (exact split)
        assertEq(token.balanceOf(admin), adminBalBefore);

        // Accumulated royalties should track 2x
        assertEq(
            splitter.accumulatedRoyalties(SERVING_MODEL, address(token)), DISTRIBUTION_AMOUNT * 2
        );
    }

    // ─── 25. Admin can update protocol royalty BPS ────────────────────────

    function testSetProtocolRoyaltyBps() public {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        splitter.setProtocolRoyaltyBps(500); // 5%

        (address receiver, uint256 royaltyAmount) =
            splitter.royaltyInfo(uint256(ROOT_MODEL), 10000e18);

        assertEq(royaltyAmount, 500e18); // 5% of 10000
    }

    function testRevertSetProtocolRoyaltyBpsTooHigh() public {
        vm.expectRevert(abi.encodeWithSelector(RoyaltySplitter.BpsExceedsMax.selector, 5001, 5000));
        splitter.setProtocolRoyaltyBps(5001);
    }

    function testRevertNonAdminSetProtocolRoyaltyBps() public {
        vm.prank(outsider);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                outsider,
                splitterAdminRole
            )
        );
        splitter.setProtocolRoyaltyBps(500);
    }

    // ─── 26. Gas efficiency: 10 ancestors ─────────────────────────────────

    function testGasEfficiency10Ancestors() public {
        // Create a 10-level deep chain
        bytes32 prev = keccak256("level_0");
        _register(prev, _emptyParents(), IModelRegistry.ContributionType.PreTraining);

        for (uint256 i = 1; i < 10; i++) {
            bytes32 current = keccak256(abi.encode("level_", i));
            bytes32[] memory parents = new bytes32[](1);
            parents[0] = prev;
            _register(current, parents, IModelRegistry.ContributionType.FineTune);
            prev = current;
        }

        // Distribute — should not run out of gas
        bytes32 leafModel = prev;
        splitter.distributeRoyalties(leafModel, DISTRIBUTION_AMOUNT, address(token));

        // Verify depth
        assertEq(registry.getLineageDepth(leafModel), 9);
    }

    // ─── 27. getRecipients returns correct count ──────────────────────────

    function testGetRecipientsCount() public {
        _setupDeepLineage();

        IRoyaltySplitter.RecipientInfo[] memory recipients = splitter.getRecipients(SERVING_MODEL);
        assertEq(recipients.length, 3); // 3 ancestors (RL, FineTune, Root)
    }

    // ─── 28. Recipient weights sum for correct proportionality ─────────────

    function testRecipientWeightsProportionality() public {
        // Build lineage: root → fineTune → serving
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);
        _register(
            FINETUNE_MODEL, _singleParent(ROOT_MODEL), IModelRegistry.ContributionType.FineTune
        );
        _register(
            SERVING_MODEL, _singleParent(FINETUNE_MODEL), IModelRegistry.ContributionType.Serving
        );

        IRoyaltySplitter.RecipientInfo[] memory recipients = splitter.getRecipients(SERVING_MODEL);

        // 2 ancestors: FineTune(depth=1), Root(depth=2)
        assertEq(recipients.length, 2);

        uint256 totalWeight = recipients[0].weight + recipients[1].weight;
        assertGt(totalWeight, 0);

        // FineTune(d=1): 100 * 80 * 90 = 720000
        // Root(d=2): 100 * 100 * 81 = 810000
        // Root gets more despite being deeper (PreTraining weight 100 > FineTune 80)
        assertGt(recipients[1].weight, recipients[0].weight);
    }

    // ─── 29. setDefaultReceiver ───────────────────────────────────────────

    function testSetDefaultReceiver() public {
        _register(ROOT_MODEL, _emptyParents(), IModelRegistry.ContributionType.PreTraining);

        address newReceiver = makeAddr("newReceiver");
        splitter.setDefaultReceiver(newReceiver);

        (address receiver,) = splitter.royaltyInfo(uint256(ROOT_MODEL), 10000e18);
        assertEq(receiver, newReceiver);
    }

    function testRevertSetDefaultReceiverZeroAddress() public {
        vm.expectRevert(RoyaltySplitter.ZeroAddress.selector);
        splitter.setDefaultReceiver(address(0));
    }

    // ─── 30. ERC-165 supportsInterface ────────────────────────────────────

    function testSupportsERC2981Interface() public view {
        assertTrue(splitter.supportsInterface(type(IERC2981).interfaceId));
    }

    function testSupportsAccessControlInterface() public view {
        // type(IERC165).interfaceId == 0x01ffc9a7
        assertTrue(splitter.supportsInterface(0x01ffc9a7));
    }

    // ─── 31. ProtocolRoyaltyBpsUpdated event ──────────────────────────────

    function testProtocolRoyaltyBpsUpdatedEvent() public {
        vm.expectEmit(false, false, false, true);
        emit IRoyaltySplitter.ProtocolRoyaltyBpsUpdated(1000, 500);
        splitter.setProtocolRoyaltyBps(500);
    }

    // ─── 32. RecipientPaid and RoyaltyDistributed events ──────────────────

    function testRoyaltyDistributionEvents() public {
        _setupSimpleLineage();

        // SERVING_MODEL has one ancestor: ROOT at depth 1
        // Expect RecipientPaid for ROOT, then RoyaltyDistributed
        vm.expectEmit(true, true, false, true);
        emit IRoyaltySplitter.RecipientPaid(SERVING_MODEL, admin, DISTRIBUTION_AMOUNT, 1);

        vm.expectEmit(true, true, false, true);
        emit IRoyaltySplitter.RoyaltyDistributed(
            SERVING_MODEL, address(token), DISTRIBUTION_AMOUNT, 1
        );

        splitter.distributeRoyalties(SERVING_MODEL, DISTRIBUTION_AMOUNT, address(token));
    }
}
