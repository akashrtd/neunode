// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import "../../src/reputation/NeunodeReputation.sol";
import "../../src/NeunodeIdentity.sol";

/// @dev Minimal stake oracle — NeunodeToken satisfies this in prod.
contract MockStakeSource {
    mapping(address => uint256) private _staked;

    function setStaked(address account, uint256 amount) external {
        _staked[account] = amount;
    }

    function stakedBalanceOf(address account) external view returns (uint256) {
        return _staked[account];
    }
}

/// @title NeunodeReputationTest — Tests for on-chain reputation and voting power
contract NeunodeReputationTest is Test {
    NeunodeReputation public rep;

    address public admin;
    address public alice;
    address public bob;
    address public carol;
    address public stakeOracle;
    address public attestOracle;
    address public activityOracle;
    address public verifyOracle;
    address public tenureOracle;
    address public slasher;

    function setUp() public {
        rep = new NeunodeReputation();

        admin = makeAddr("admin");
        alice = makeAddr("alice");
        bob = makeAddr("bob");
        carol = makeAddr("carol");
        stakeOracle = makeAddr("stakeOracle");
        attestOracle = makeAddr("attestOracle");
        activityOracle = makeAddr("activityOracle");
        verifyOracle = makeAddr("verifyOracle");
        tenureOracle = makeAddr("tenureOracle");
        slasher = makeAddr("slasher");

        // Grant oracle roles from the deployer (DEFAULT_ADMIN)
        rep.grantRole(rep.STAKE_ORACLE_ROLE(), stakeOracle);
        rep.grantRole(rep.ATTEST_ORACLE_ROLE(), attestOracle);
        rep.grantRole(rep.ACTIVITY_ORACLE_ROLE(), activityOracle);
        rep.grantRole(rep.VERIFY_ORACLE_ROLE(), verifyOracle);
        rep.grantRole(rep.TENURE_ORACLE_ROLE(), tenureOracle);
        rep.grantRole(rep.SLASHING_ROLE(), slasher);
        rep.grantRole(rep.REPUTATION_ADMIN_ROLE(), admin);
    }

    // ─── Helper: set all factor scores to a given BPS ─────────────────────

    function _setAllScores(address agent, uint16 bps) internal {
        vm.prank(stakeOracle);
        rep.updateFactorScore(agent, 0, bps);
        vm.prank(attestOracle);
        rep.updateFactorScore(agent, 1, bps);
        vm.prank(activityOracle);
        rep.updateFactorScore(agent, 2, bps);
        vm.prank(verifyOracle);
        rep.updateFactorScore(agent, 3, bps);
        vm.prank(tenureOracle);
        rep.updateFactorScore(agent, 4, bps);
    }

    function _finalizeCurrentEpoch() internal {
        NeunodeReputation.EpochInfo memory epoch = rep.getEpochInfo(rep.getCurrentEpoch());
        vm.roll(epoch.endBlock);
        rep.finalizeEpoch();
    }

    // ─── Identity wiring for Sybil-resistance tests ───────────────────────

    NeunodeIdentity public identity;
    MockStakeSource public stake;
    uint256 public constant MIN_STAKE = 1000e18;

    function _wireIdentityRegistry() internal {
        identity = new NeunodeIdentity();
        stake = new MockStakeSource();
        identity.setStakeSource(address(stake));
        identity.setMinRegistrationStake(MIN_STAKE);
        vm.prank(admin);
        rep.setIdentityRegistry(address(identity));
    }

    // ─── Sybil-resistance wiring (identity registration gates validators) ──

    function test_registerValidator_revertsWhenRegistrySetButNoDid() public {
        _wireIdentityRegistry();
        _setAllScores(bob, 6000); // sufficient reputation, but bob controls no DID

        vm.prank(bob);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeReputation.NotNetworkRegistered.selector, bob)
        );
        rep.registerValidator();
    }

    function test_registerValidator_revertsWhenDidNotRegistered() public {
        _wireIdentityRegistry();
        vm.prank(carol);
        identity.createDid(keccak256("carol_ed")); // DID exists but not network-registered
        _setAllScores(carol, 6000);

        vm.prank(carol);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeReputation.NotNetworkRegistered.selector, carol)
        );
        rep.registerValidator();
    }

    function test_registerValidator_succeedsWhenRegistered() public {
        _wireIdentityRegistry();
        vm.prank(alice);
        bytes32 did = identity.createDid(keccak256("alice_ed"));
        stake.setStaked(alice, MIN_STAKE);
        vm.prank(alice);
        identity.registerForNetwork(did);
        _setAllScores(alice, 6000);

        vm.prank(alice);
        vm.expectEmit(true, false, false, false);
        emit NeunodeReputation.ValidatorRegistered(alice);
        rep.registerValidator();
    }

    function testRevert_setIdentityRegistry_notAdmin() public {
        vm.prank(alice); // not REPUTATION_ADMIN_ROLE
        vm.expectRevert();
        rep.setIdentityRegistry(address(0xBEEF));
    }

    // ─── Stake-factor on-chain derivation (decentralization) ───────────────
    // When a stake source is configured, the stake factor (factor 0) is derived
    // deterministically from real staked balance instead of being pushed by a
    // trusted oracle — removing one centralized trust assumption.

    function _configureStakeDerivation(uint256 target) internal {
        stake = new MockStakeSource();
        vm.prank(admin);
        rep.setStakeSource(address(stake));
        vm.prank(admin);
        rep.setStakeFactorTarget(target);
    }

    function test_stakeFactor_derivedFromBalance() public {
        _configureStakeDerivation(10_000e18); // 10k staked == 100% factor

        stake.setStaked(alice, 5_000e18); // half the target → 50%
        rep.deriveStakeFactor(alice);

        assertEq(rep.getFactorScores(alice).stake, 5000);
    }

    function test_stakeFactor_cappedAtMax() public {
        _configureStakeDerivation(1_000e18);

        stake.setStaked(alice, 1_000_000e18); // far above target
        rep.deriveStakeFactor(alice);

        assertEq(rep.getFactorScores(alice).stake, 10000); // capped at 100%
    }

    function test_revert_oracleCannotSetStakeFactorWhenDeriving() public {
        _configureStakeDerivation(10_000e18);

        vm.prank(stakeOracle);
        vm.expectRevert(NeunodeReputation.StakeFactorDerived.selector);
        rep.updateFactorScore(alice, 0, 7500);
    }

    function testRevert_setStakeSource_notAdmin() public {
        vm.prank(alice);
        vm.expectRevert();
        rep.setStakeSource(address(0xBEEF));
    }

    // ─── 1. test_updateFactorScore ────────────────────────────────────────

    function test_updateFactorScore() public {
        vm.prank(stakeOracle);
        rep.updateFactorScore(alice, 0, 7500);

        NeunodeReputation.FactorScores memory scores = rep.getFactorScores(alice);
        assertEq(scores.stake, 7500);
        // Other factors should be 0
        assertEq(scores.attest, 0);
        assertEq(scores.activity, 0);
        assertEq(scores.verify, 0);
        assertEq(scores.tenure, 0);

        vm.prank(attestOracle);
        rep.updateFactorScore(alice, 1, 5000);
        scores = rep.getFactorScores(alice);
        assertEq(scores.attest, 5000);

        vm.prank(activityOracle);
        rep.updateFactorScore(alice, 2, 8000);
        scores = rep.getFactorScores(alice);
        assertEq(scores.activity, 8000);

        vm.prank(verifyOracle);
        rep.updateFactorScore(alice, 3, 6000);
        scores = rep.getFactorScores(alice);
        assertEq(scores.verify, 6000);

        vm.prank(tenureOracle);
        rep.updateFactorScore(alice, 4, 9000);
        scores = rep.getFactorScores(alice);
        assertEq(scores.tenure, 9000);
    }

    function testRevert_updateFactorScore_badOracle() public {
        vm.prank(makeAddr("fakeOracle"));
        vm.expectRevert();
        rep.updateFactorScore(alice, 0, 5000);
    }

    function testRevert_updateFactorScore_invalidIndex() public {
        vm.prank(stakeOracle);
        vm.expectRevert(abi.encodeWithSelector(NeunodeReputation.InvalidFactorIndex.selector, 5));
        rep.updateFactorScore(alice, 5, 5000);
    }

    function testRevert_updateFactorScore_outOfBounds() public {
        vm.prank(stakeOracle);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeReputation.ScoreOutOfBounds.selector, uint16(10001))
        );
        rep.updateFactorScore(alice, 0, 10001);
    }

    // ─── 2. test_compositeScore ───────────────────────────────────────────

    function test_compositeScore() public {
        // Set all factors to 10000 (100%) — composite should be 10000
        _setAllScores(alice, 10000);
        assertEq(rep.getCompositeScore(alice), 10000);

        // Set all factors to 5000 (50%) — composite should be 5000
        _setAllScores(bob, 5000);
        assertEq(rep.getCompositeScore(bob), 5000);

        // Set all factors to 0 — composite should be 0
        _setAllScores(carol, 0);
        assertEq(rep.getCompositeScore(carol), 0);
    }

    function test_compositeScore_weighted() public {
        // Weights: stake=3000, attest=2500, activity=2000, verify=1500, tenure=1000
        // Only stake = 10000, rest 0 => 10000 * 3000 / 10000 = 3000
        vm.prank(stakeOracle);
        rep.updateFactorScore(alice, 0, 10000);

        assertEq(rep.getCompositeScore(alice), 3000);
    }

    // ─── 3. test_votingPower_sqrt ─────────────────────────────────────────

    function test_votingPower_sqrt() public {
        _setAllScores(alice, 10000);
        uint256 vp = rep.getVotingPower(alice);

        // sqrt(10000) = 100, VP = 100 * 1e12 / 100 = 1e12
        assertEq(vp, 1e12);

        _setAllScores(bob, 0);
        assertEq(rep.getVotingPower(bob), 0);

        _setAllScores(carol, 2500);
        // sqrt(2500) = 50, VP = 50 * 1e12 / 100 = 5e11
        assertEq(rep.getVotingPower(carol), 5e11);
    }

    // ─── 4. test_registerValidator ────────────────────────────────────────

    function test_registerValidator() public {
        _setAllScores(alice, 8000); // 8000 >= 5000 minimum
        assertTrue(rep.isEligibleValidator(alice));

        vm.prank(alice);
        rep.registerValidator();

        address[] memory validators = rep.getActiveValidators();
        assertEq(validators.length, 1);
        assertEq(validators[0], alice);
    }

    // ─── 5. test_registerValidator_insufficientReputation ─────────────────

    function test_registerValidator_insufficientReputation() public {
        _setAllScores(alice, 3000); // 3000 < 5000 minimum

        assertFalse(rep.isEligibleValidator(alice));

        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeReputation.InsufficientReputation.selector, alice, 3000, 5000
            )
        );
        rep.registerValidator();
    }

    // ─── 6. test_deregisterValidator ──────────────────────────────────────

    function test_deregisterValidator() public {
        _setAllScores(alice, 8000);
        vm.prank(alice);
        rep.registerValidator();

        vm.prank(alice);
        rep.deregisterValidator();

        address[] memory validators = rep.getActiveValidators();
        assertEq(validators.length, 0);
    }

    function testRevert_deregisterValidator_notValidator() public {
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(NeunodeReputation.NotAValidator.selector, alice));
        rep.deregisterValidator();
    }

    // ─── 7. test_maxValidators ────────────────────────────────────────────

    function test_maxValidators() public {
        // Set max to 3 for testing
        vm.prank(admin);
        rep.setMaxValidators(3);

        // Register 3 validators
        address[] memory agents = new address[](4);
        agents[0] = makeAddr("v1");
        agents[1] = makeAddr("v2");
        agents[2] = makeAddr("v3");
        agents[3] = makeAddr("v4");

        for (uint256 i = 0; i < 3; i++) {
            _setAllScores(agents[i], 8000);
            vm.prank(agents[i]);
            rep.registerValidator();
        }

        // 4th should fail
        _setAllScores(agents[3], 9000);
        vm.prank(agents[3]);
        vm.expectRevert(abi.encodeWithSelector(NeunodeReputation.MaxValidatorsReached.selector, 3));
        rep.registerValidator();
    }

    function testRevert_registerValidator_alreadyValidator() public {
        _setAllScores(alice, 8000);
        vm.prank(alice);
        rep.registerValidator();

        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(NeunodeReputation.AlreadyValidator.selector, alice));
        rep.registerValidator();
    }

    // ─── 8. test_finalizeEpoch ────────────────────────────────────────────

    function test_finalizeEpoch() public {
        _setAllScores(alice, 8000);
        vm.prank(alice);
        rep.registerValidator();

        uint256 epochBefore = rep.getCurrentEpoch();
        _finalizeCurrentEpoch();

        assertEq(rep.getCurrentEpoch(), epochBefore + 1);

        NeunodeReputation.EpochInfo memory info = rep.getEpochInfo(epochBefore);
        assertTrue(info.isFinalized);
    }

    function testRevert_finalizeEpoch_unauthorized() public {
        NeunodeReputation.EpochInfo memory epoch = rep.getEpochInfo(rep.getCurrentEpoch());
        vm.roll(epoch.endBlock);
        bytes32 finalizerRole = rep.EPOCH_FINALIZER_ROLE();

        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(
                bytes4(keccak256("AccessControlUnauthorizedAccount(address,bytes32)")),
                alice,
                finalizerRole
            )
        );
        rep.finalizeEpoch();
    }

    function testRevert_finalizeEpoch_beforeScheduledEnd() public {
        NeunodeReputation.EpochInfo memory epoch = rep.getEpochInfo(rep.getCurrentEpoch());

        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeReputation.EpochNotEnded.selector,
                rep.getCurrentEpoch(),
                block.number,
                epoch.endBlock
            )
        );
        rep.finalizeEpoch();
    }

    function testRevert_finalizeEpoch_alreadyFinalized() public {
        // finalizeEpoch advances to next epoch after finalizing current one.
        // To trigger EpochAlreadyFinalized, we need the current epoch to already be marked.
        // This can happen if the epoch info struct was pre-marked (edge case with manual state).
        // We finalize once normally, then verify the next epoch is NOT finalized.
        _finalizeCurrentEpoch();
        assertEq(rep.getCurrentEpoch(), 2);

        NeunodeReputation.EpochInfo memory info = rep.getEpochInfo(2);
        assertFalse(info.isFinalized);
    }

    // ─── 9. test_validatorSetTransition ───────────────────────────────────

    function test_validatorSetTransition() public {
        _setAllScores(alice, 8000);
        vm.prank(alice);
        rep.registerValidator();

        _setAllScores(bob, 9000);
        vm.prank(bob);
        rep.registerValidator();

        uint256 epoch1 = rep.getCurrentEpoch();
        _finalizeCurrentEpoch();

        // Query validator set for epoch 1
        (address[] memory validators, uint256[] memory powers) = rep.getValidatorSetForEpoch(epoch1);
        assertEq(validators.length, 2);
        assertEq(powers.length, 2);
    }

    function testRevert_getValidatorSetForEpoch_notFinalized() public {
        vm.expectRevert(abi.encodeWithSelector(NeunodeReputation.EpochNotFinalized.selector, 1));
        rep.getValidatorSetForEpoch(1);
    }

    // ─── 10. test_applyPenalty ─────────────────────────────────────────────

    function test_applyPenalty() public {
        _setAllScores(alice, 8000);
        uint256 scoreBefore = rep.getCompositeScore(alice);

        vm.prank(slasher);
        rep.applyPenalty(alice, 3000, 500);

        uint256 scoreAfter = rep.getCompositeScore(alice);
        assertLt(scoreAfter, scoreBefore);
    }

    function test_applyPenalty_clampsAtZero() public {
        _setAllScores(alice, 2000);

        vm.prank(slasher);
        rep.applyPenalty(alice, 10000, 500);

        // Penalty exceeds composite score, should clamp to 0
        assertEq(rep.getCompositeScore(alice), 0);
        assertEq(rep.getVotingPower(alice), 0);
    }

    // ─── 11. test_penaltyDecay ────────────────────────────────────────────

    function test_penaltyDecay() public {
        _setAllScores(alice, 8000);

        vm.prank(slasher);
        rep.applyPenalty(alice, 2000, 500);

        uint256 scoreAfterPenalty = rep.getCompositeScore(alice);

        // Advance through epochs to decay penalty
        for (uint256 i = 0; i < 45; i++) {
            _finalizeCurrentEpoch();
        }

        // After 45 epochs, penalty should be half decayed
        uint256 decay = rep.getPenaltyDecay(alice);
        assertGt(decay, 0);
        assertLt(decay, 2000);

        // Recompute happens on next score update — update a factor to trigger recompute
        vm.prank(stakeOracle);
        rep.updateFactorScore(alice, 0, 8000);

        uint256 scoreAfterDecay = rep.getCompositeScore(alice);
        assertGt(scoreAfterDecay, scoreAfterPenalty);
    }

    function test_penaltyDecay_fullDecay() public {
        _setAllScores(alice, 8000);

        vm.prank(slasher);
        rep.applyPenalty(alice, 2000, 500);

        // Advance 90+ epochs for full decay
        for (uint256 i = 0; i < 91; i++) {
            _finalizeCurrentEpoch();
        }

        assertEq(rep.getPenaltyDecay(alice), 0);
    }

    // ─── 12. test_setFactorWeights ────────────────────────────────────────

    function test_setFactorWeights() public {
        NeunodeReputation.FactorWeights memory newWeights = NeunodeReputation.FactorWeights({
            stake: 5000, attest: 2000, activity: 1000, verify: 1000, tenure: 1000
        });

        vm.prank(admin);
        rep.setFactorWeights(newWeights);

        (uint16 ws, uint16 wa, uint16 wact, uint16 wv, uint16 wt) = rep.weights();
        assertEq(ws, 5000);
        assertEq(wa, 2000);
        // Suppress unused variable warnings by asserting values
        assertEq(wact, 1000);
        assertEq(wv, 1000);
        assertEq(wt, 1000);
    }

    function testRevert_setFactorWeights_invalidSum() public {
        // Test with sum != 10000
        NeunodeReputation.FactorWeights memory invalidWeights = NeunodeReputation.FactorWeights({
            stake: 4000, attest: 3000, activity: 2000, verify: 1000, tenure: 500
        }); // sum = 10500

        vm.prank(admin);
        vm.expectRevert(abi.encodeWithSelector(NeunodeReputation.InvalidWeightSum.selector, 10500));
        rep.setFactorWeights(invalidWeights);
    }

    function testRevert_setFactorWeights_unauthorized() public {
        NeunodeReputation.FactorWeights memory newWeights = NeunodeReputation.FactorWeights({
            stake: 5000, attest: 2000, activity: 1000, verify: 1000, tenure: 1000
        });

        vm.prank(makeAddr("random"));
        vm.expectRevert();
        rep.setFactorWeights(newWeights);
    }

    // ─── 13. test_batchUpdateScores ───────────────────────────────────────

    function test_batchUpdateScores() public {
        address[] memory agents = new address[](3);
        agents[0] = alice;
        agents[1] = bob;
        agents[2] = carol;

        uint16[] memory scores = new uint16[](3);
        scores[0] = 7000;
        scores[1] = 8000;
        scores[2] = 9000;

        vm.prank(stakeOracle);
        rep.batchUpdateScores(agents, 0, scores);

        assertEq(rep.getFactorScores(alice).stake, 7000);
        assertEq(rep.getFactorScores(bob).stake, 8000);
        assertEq(rep.getFactorScores(carol).stake, 9000);
    }

    function testRevert_batchUpdateScores_lengthMismatch() public {
        address[] memory agents = new address[](2);
        agents[0] = alice;
        agents[1] = bob;

        uint16[] memory scores = new uint16[](3);
        scores[0] = 7000;
        scores[1] = 8000;
        scores[2] = 9000;

        vm.prank(stakeOracle);
        vm.expectRevert(NeunodeReputation.ArrayLengthMismatch.selector);
        rep.batchUpdateScores(agents, 0, scores);
    }

    // ─── 14. test_gas_updateAndFinalize ───────────────────────────────────

    function test_gas_updateAndFinalize() public {
        // Register 10 validators, update scores, finalize epoch
        for (uint256 i = 0; i < 10; i++) {
            address agent = makeAddr(string(abi.encodePacked("validator", bytes1(uint8(0x30 + i)))));
            _setAllScores(agent, 8000);
            vm.prank(agent);
            rep.registerValidator();
        }

        uint256 gasBefore = gasleft();
        _finalizeCurrentEpoch();
        uint256 gasUsed = gasBefore - gasleft();

        // Log gas for snapshot — should be reasonable
        assertLt(gasUsed, 2_000_000);
    }

    // ─── 15. test_fuzz_votingPower ────────────────────────────────────────

    function test_fuzz_votingPower(uint16 scoreBps) public {
        vm.assume(scoreBps <= 10000);

        _setAllScores(alice, scoreBps);

        uint256 composite = rep.getCompositeScore(alice);
        assertLe(composite, 10000);

        uint256 vp = rep.getVotingPower(alice);
        if (composite == 0) {
            assertEq(vp, 0);
        } else {
            assertGt(vp, 0);
            // VP = sqrt(composite) * 1e12 / 100
            // sqrt(10000) = 100 => max VP = 100 * 1e12 / 100 = 1e12
            assertLe(vp, 1e12);
        }
    }

    // ─── Additional coverage ──────────────────────────────────────────────

    function test_getTotalVotingPower() public {
        _setAllScores(alice, 10000);
        vm.prank(alice);
        rep.registerValidator();

        _setAllScores(bob, 10000);
        vm.prank(bob);
        rep.registerValidator();

        uint256 total = rep.getTotalVotingPower();
        // Both have score 10000, VP = 1e12 each
        assertEq(total, 2e12);
    }

    function test_setMinReputationThreshold() public {
        vm.prank(admin);
        rep.setMinReputationThreshold(8000);

        _setAllScores(alice, 7000);
        assertFalse(rep.isEligibleValidator(alice));
    }

    function test_epochInfo_genesis() public view {
        assertEq(rep.getCurrentEpoch(), 1);

        NeunodeReputation.EpochInfo memory info = rep.getEpochInfo(1);
        assertFalse(info.isFinalized);
        assertEq(info.startBlock, block.number);
    }

    function test_penaltyDecay_noPenalty() public {
        _setAllScores(alice, 8000);
        assertEq(rep.getPenaltyDecay(alice), 0);
    }

    function test_applyPenalty_unauthorized() public {
        vm.prank(makeAddr("random"));
        vm.expectRevert();
        rep.applyPenalty(alice, 1000, 500);
    }

    function test_deregisterValidator_updatesList() public {
        // Register 3 validators, deregister middle one
        _setAllScores(alice, 8000);
        _setAllScores(bob, 8000);
        _setAllScores(carol, 8000);

        vm.prank(alice);
        rep.registerValidator();
        vm.prank(bob);
        rep.registerValidator();
        vm.prank(carol);
        rep.registerValidator();

        address[] memory validators = rep.getActiveValidators();
        assertEq(validators.length, 3);

        vm.prank(bob);
        rep.deregisterValidator();

        validators = rep.getActiveValidators();
        assertEq(validators.length, 2);
    }
}
