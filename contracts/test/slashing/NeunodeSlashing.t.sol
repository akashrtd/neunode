// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import "../../src/slashing/NeunodeSlashing.sol";
import "../../src/tokens/ComputeToken.sol";

/// @title NeunodeSlashingTest -- Comprehensive tests for NeunodeSlashing
contract NeunodeSlashingTest is Test {
    NeunodeSlashing public slashing;
    ComputeToken public token;

    address public admin;
    address public validator;
    address public reporter;
    address public attacker;
    address public governanceAdmin;

    uint256 constant STAKE_AMOUNT = 10_000e18;

    function setUp() public {
        admin = makeAddr("admin");
        validator = makeAddr("validator");
        reporter = makeAddr("reporter");
        attacker = makeAddr("attacker");
        governanceAdmin = makeAddr("governanceAdmin");

        token = new ComputeToken();
        vm.prank(admin);
        slashing = new NeunodeSlashing(address(token));

        // Grant governance admin the GOVERNANCE_ROLE on the token so slashStake works
        token.grantRole(token.GOVERNANCE_ROLE(), address(slashing));

        // Grant SLASHING_ROLE and REPORTER_ROLE on the slashing contract (admin has DEFAULT_ADMIN_ROLE)
        vm.startPrank(admin);
        slashing.grantRole(slashing.SLASHING_ROLE(), governanceAdmin);
        slashing.grantRole(slashing.REPORTER_ROLE(), reporter);
        vm.stopPrank();

        // Fund and stake the validator
        token.mint(validator, STAKE_AMOUNT);
        vm.prank(validator);
        token.stake(STAKE_AMOUNT);
    }

    // ─── Helpers ──────────────────────────────────────────────────────────

    function _buildEvidence(
        bytes32 h1,
        bytes32 h2,
        uint256 blockNum,
        bytes memory extra,
        address target
    ) internal view returns (NeunodeSlashing.SlashingEvidence memory) {
        return NeunodeSlashing.SlashingEvidence({
            blockHash1: h1,
            blockHash2: h2,
            blockNumber: blockNum,
            signature1: "",
            signature2: "",
            extraData: extra,
            reporter: target,
            timestamp: block.timestamp
        });
    }

    function _computeHash(
        NeunodeSlashing.OffenseType offense,
        bytes32 h1,
        bytes32 h2,
        uint256 blockNum,
        bytes memory sig1,
        bytes memory sig2,
        bytes memory extra,
        address target
    ) internal pure returns (bytes32) {
        return keccak256(abi.encode(offense, h1, h2, blockNum, sig1, sig2, extra, target));
    }

    // ─── 1. Submit Evidence: Double Sign ──────────────────────────────────

    function test_submitEvidence_doubleSign() public {
        bytes32 h1 = keccak256("block1");
        bytes32 h2 = keccak256("block2");
        NeunodeSlashing.SlashingEvidence memory evidence =
            _buildEvidence(h1, h2, 100, "", validator);
        bytes32 evidenceHash = _computeHash(
            NeunodeSlashing.OffenseType.DoubleSign, h1, h2, 100, "", "", "", validator
        );

        vm.expectEmit(true, true, false, true);
        emit NeunodeSlashing.EvidenceSubmitted(
            validator, NeunodeSlashing.OffenseType.DoubleSign, reporter, evidenceHash
        );

        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, evidence);

        // Verify validator was slashed and jailed
        assertTrue(slashing.isJailed(validator));

        NeunodeSlashing.ValidatorStatus memory status = slashing.getValidatorStatus(validator);
        assertTrue(status.isJailed);
        assertFalse(status.isTombstoned);
        assertEq(status.offenseCount, 1);

        // Verify slash was applied: 500 bps = 5% of 10_000e18 = 500e18
        uint256 expectedRemaining = STAKE_AMOUNT - (STAKE_AMOUNT * 500 / 10_000);
        assertEq(token.stakedBalanceOf(validator), expectedRemaining);
    }

    // ─── 2. Submit Evidence: Equivocation ─────────────────────────────────

    function test_submitEvidence_equivocation() public {
        bytes32 h1 = keccak256("equiv_block1");
        bytes32 h2 = keccak256("equiv_block2");
        bytes memory extra = abi.encode("equivocation_proof");
        NeunodeSlashing.SlashingEvidence memory evidence =
            _buildEvidence(h1, h2, 200, extra, validator);
        bytes32 evidenceHash = _computeHash(
            NeunodeSlashing.OffenseType.Equivocation, h1, h2, 200, "", "", extra, validator
        );

        vm.expectEmit(true, true, false, true);
        emit NeunodeSlashing.EvidenceSubmitted(
            validator, NeunodeSlashing.OffenseType.Equivocation, reporter, evidenceHash
        );

        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.Equivocation, evidence);

        assertTrue(slashing.isJailed(validator));

        // Equivocation first offense: 300 bps = 3%
        uint256 expectedRemaining = STAKE_AMOUNT - (STAKE_AMOUNT * 300 / 10_000);
        assertEq(token.stakedBalanceOf(validator), expectedRemaining);
    }

    // ─── 3. Submit Evidence: Rejects Invalid (tombstoned validator) ───────

    function test_submitEvidence_rejectsInvalid() public {
        // Tombstone the validator via 3 DoubleSign offenses
        for (uint256 i = 0; i < 3; i++) {
            NeunodeSlashing.SlashingEvidence memory ev = _buildEvidence(
                keccak256(abi.encode("block1", i)),
                keccak256(abi.encode("block2", i)),
                100 + i,
                abi.encode(i),
                validator
            );

            vm.prank(reporter);
            slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, ev);
        }

        // Verify the validator is tombstoned
        NeunodeSlashing.ValidatorStatus memory vs = slashing.getValidatorStatus(validator);
        assertTrue(vs.isTombstoned);

        // Now try submitting evidence again for the tombstoned validator
        NeunodeSlashing.SlashingEvidence memory ev =
            _buildEvidence(keccak256("block_new1"), keccak256("block_new2"), 999, "", validator);

        vm.prank(reporter);
        vm.expectRevert(NeunodeSlashing.ValidatorAlreadyTombstoned.selector);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, ev);
    }

    // ─── 4. Submit Evidence: Rejects Duplicate ────────────────────────────

    function test_submitEvidence_rejectsDuplicate() public {
        bytes32 h1 = keccak256("block1");
        bytes32 h2 = keccak256("block2");
        NeunodeSlashing.SlashingEvidence memory evidence =
            _buildEvidence(h1, h2, 100, "", validator);
        bytes32 evidenceHash = _computeHash(
            NeunodeSlashing.OffenseType.DoubleSign, h1, h2, 100, "", "", "", validator
        );

        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, evidence);

        // Submit identical evidence again
        vm.prank(reporter);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeSlashing.DuplicateEvidence.selector, evidenceHash)
        );
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, evidence);
    }

    // ─── 5. Penalty Schedule: First Offense ───────────────────────────────

    function test_penaltySchedule_firstOffense() public {
        // DoubleSign first offense: 5% stake slash, jailed
        NeunodeSlashing.SlashingPenalty memory penalty =
            slashing.getPenaltySchedule(NeunodeSlashing.OffenseType.DoubleSign, 0);
        assertEq(penalty.stakeSlashBps, 500);
        assertEq(penalty.reputationSlashBps, 1000);
        assertEq(uint8(penalty.outcome), uint8(NeunodeSlashing.PenaltyOutcome.Jailed));

        // Downtime first offense: 0.5% stake slash, jailed
        penalty = slashing.getPenaltySchedule(NeunodeSlashing.OffenseType.Downtime, 0);
        assertEq(penalty.stakeSlashBps, 50);
        assertEq(penalty.reputationSlashBps, 100);
        assertEq(uint8(penalty.outcome), uint8(NeunodeSlashing.PenaltyOutcome.Jailed));

        // Spamming first offense: 0.1% stake slash, rate limited
        penalty = slashing.getPenaltySchedule(NeunodeSlashing.OffenseType.Spamming, 0);
        assertEq(penalty.stakeSlashBps, 10);
        assertEq(penalty.reputationSlashBps, 20);
        assertEq(uint8(penalty.outcome), uint8(NeunodeSlashing.PenaltyOutcome.RateLimited));
    }

    // ─── 6. Penalty Schedule: Escalating Penalties ────────────────────────

    function test_penaltySchedule_escalating() public {
        // DoubleSign: 5% -> 15% -> tombstone
        NeunodeSlashing.SlashingPenalty memory p0 =
            slashing.getPenaltySchedule(NeunodeSlashing.OffenseType.DoubleSign, 0);
        NeunodeSlashing.SlashingPenalty memory p1 =
            slashing.getPenaltySchedule(NeunodeSlashing.OffenseType.DoubleSign, 1);
        NeunodeSlashing.SlashingPenalty memory p2 =
            slashing.getPenaltySchedule(NeunodeSlashing.OffenseType.DoubleSign, 2);

        assertLt(p0.stakeSlashBps, p1.stakeSlashBps);
        assertLt(p1.stakeSlashBps, p2.stakeSlashBps);
        assertEq(uint8(p2.outcome), uint8(NeunodeSlashing.PenaltyOutcome.Tombstoned));

        // Downtime: 0.5% -> 2% -> 5%
        p0 = slashing.getPenaltySchedule(NeunodeSlashing.OffenseType.Downtime, 0);
        p1 = slashing.getPenaltySchedule(NeunodeSlashing.OffenseType.Downtime, 1);
        p2 = slashing.getPenaltySchedule(NeunodeSlashing.OffenseType.Downtime, 2);

        assertLt(p0.stakeSlashBps, p1.stakeSlashBps);
        assertLt(p1.stakeSlashBps, p2.stakeSlashBps);
    }

    // ─── 7. Jail and Unjail Lifecycle ─────────────────────────────────────

    function test_jailAndUnjail() public {
        // Slash the validator for downtime (jail outcome)
        vm.prank(reporter);
        slashing.reportDowntime(validator, 600, 1000); // 60% missed in 1000 block window

        NeunodeSlashing.ValidatorStatus memory status = slashing.getValidatorStatus(validator);
        assertTrue(status.isJailed);
        assertFalse(status.isTombstoned);
        assertEq(status.offenseCount, 1);
        assertGt(status.jailReleaseBlock, block.number);

        // Try to unjail before expiry
        vm.prank(validator);
        vm.expectRevert(NeunodeSlashing.JailNotExpired.selector);
        slashing.unjail(validator);

        // Warp to after release block
        vm.roll(status.jailReleaseBlock + 1);

        // Unjail
        vm.expectEmit(true, false, false, true);
        emit NeunodeSlashing.ValidatorUnjailed(validator);
        vm.prank(validator);
        slashing.unjail(validator);

        status = slashing.getValidatorStatus(validator);
        assertFalse(status.isJailed);
    }

    // ─── 8. Tombstone After Third Offense ─────────────────────────────────

    function test_tombstone() public {
        // First DoubleSign offense
        NeunodeSlashing.SlashingEvidence memory ev1 =
            _buildEvidence(keccak256("ds1_b1"), keccak256("ds1_b2"), 100, "first", validator);
        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, ev1);

        // Second DoubleSign offense
        NeunodeSlashing.SlashingEvidence memory ev2 =
            _buildEvidence(keccak256("ds2_b1"), keccak256("ds2_b2"), 200, "second", validator);
        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, ev2);

        // Third DoubleSign offense -> tombstone
        NeunodeSlashing.SlashingEvidence memory ev3 =
            _buildEvidence(keccak256("ds3_b1"), keccak256("ds3_b2"), 300, "third", validator);

        vm.expectEmit(true, false, false, true);
        emit NeunodeSlashing.ValidatorTombstoned(validator);
        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, ev3);

        NeunodeSlashing.ValidatorStatus memory tombStatus = slashing.getValidatorStatus(validator);
        assertTrue(tombStatus.isTombstoned);
        assertFalse(tombStatus.isJailed);
        assertEq(tombStatus.offenseCount, 3);
        assertEq(tombStatus.jailReleaseBlock, 0);

        // Verify 3rd offense slash applied (5000 bps = 50% of remaining)
        assertLt(token.stakedBalanceOf(validator), STAKE_AMOUNT);
    }

    // ─── 9. Report Downtime ───────────────────────────────────────────────

    function test_reportDowntime() public {
        uint256 missedBlocks = 800;
        uint256 windowBlocks = 1000;

        // 50 bps = 0.5% of 10_000e18 = 50e18
        uint256 expectedSlash = STAKE_AMOUNT * 50 / 10_000;

        vm.expectEmit(true, false, false, true);
        emit NeunodeSlashing.ValidatorSlashed(
            validator, NeunodeSlashing.OffenseType.Downtime, expectedSlash, 100
        );

        vm.prank(reporter);
        slashing.reportDowntime(validator, missedBlocks, windowBlocks);

        assertTrue(slashing.isJailed(validator));
        assertEq(slashing.getOffenseCount(validator, NeunodeSlashing.OffenseType.Downtime), 1);

        // 0.5% of 10_000e18 = 50e18
        assertEq(token.stakedBalanceOf(validator), STAKE_AMOUNT - expectedSlash);
    }

    function test_reportDowntime_rejectsBelowThreshold() public {
        // 400 missed out of 1000 = 40%, below 50% threshold
        vm.prank(reporter);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeSlashing.DowntimeThresholdNotMet.selector, 400, 501)
        );
        slashing.reportDowntime(validator, 400, 1000);
    }

    // ─── 10. Access Control ───────────────────────────────────────────────

    function test_accessControl_setPenaltySchedule() public {
        // Admin can update penalty schedule
        vm.prank(admin);
        slashing.setPenaltySchedule(
            NeunodeSlashing.OffenseType.DoubleSign,
            0,
            600,
            1200,
            0,
            NeunodeSlashing.PenaltyOutcome.Jailed
        );

        NeunodeSlashing.SlashingPenalty memory penalty =
            slashing.getPenaltySchedule(NeunodeSlashing.OffenseType.DoubleSign, 0);
        assertEq(penalty.stakeSlashBps, 600);
        assertEq(penalty.reputationSlashBps, 1200);

        // Attacker cannot update penalty schedule
        vm.prank(attacker);
        vm.expectRevert();
        slashing.setPenaltySchedule(
            NeunodeSlashing.OffenseType.DoubleSign,
            0,
            100,
            200,
            0,
            NeunodeSlashing.PenaltyOutcome.Jailed
        );
    }

    function test_accessControl_pause() public {
        // Admin can pause
        vm.prank(admin);
        slashing.pause();
        assertTrue(slashing.paused());

        // Attacker cannot pause
        vm.prank(admin);
        slashing.unpause();

        vm.prank(attacker);
        vm.expectRevert();
        slashing.pause();
    }

    function test_accessControl_pauseBlocksEvidence() public {
        vm.prank(admin);
        slashing.pause();

        NeunodeSlashing.SlashingEvidence memory evidence =
            _buildEvidence(keccak256("block1"), keccak256("block2"), 100, "", validator);
        vm.prank(reporter);
        vm.expectRevert();
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, evidence);
    }

    function test_accessControl_reportDowntimeRequiresNotPaused() public {
        vm.prank(admin);
        slashing.pause();

        vm.prank(reporter);
        vm.expectRevert();
        slashing.reportDowntime(validator, 600, 1000);
    }

    // ─── 11. Fuzz Test: Submit Evidence ───────────────────────────────────

    function test_fuzz_submitEvidence(
        bytes32 blockHash1,
        bytes32 blockHash2,
        uint256 blockNumber,
        bytes calldata extraData
    ) public {
        vm.assume(blockHash1 != blockHash2);

        NeunodeSlashing.SlashingEvidence memory evidence =
            _buildEvidence(blockHash1, blockHash2, blockNumber, extraData, validator);

        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, evidence);

        assertEq(slashing.slashingEventCount(), 1);
        assertEq(slashing.getOffenseCount(validator, NeunodeSlashing.OffenseType.DoubleSign), 1);
    }

    // ─── 12. Gas Snapshot: Submit Evidence ────────────────────────────────

    function test_gas_submitEvidence() public {
        NeunodeSlashing.SlashingEvidence memory evidence =
            _buildEvidence(keccak256("block1"), keccak256("block2"), 100, "", validator);

        uint256 gasBefore = gasleft();
        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, evidence);
        uint256 gasUsed = gasBefore - gasleft();

        // Sanity check: should not exceed 300k gas
        assertLt(gasUsed, 300_000);
    }

    // ─── Additional: Report Double Sign with Signatures ───────────────────

    function test_reportDoubleSign() public {
        bytes memory header1 = abi.encodePacked("block_header_1");
        bytes memory header2 = abi.encodePacked("block_header_2");
        bytes32 h1 = keccak256(header1);
        bytes32 h2 = keccak256(header2);

        // Sign the block hashes (the contract will keccak256 the header bytes, then verify sig)
        (uint8 v1, bytes32 r1, bytes32 s1) = vm.sign(uint256(1), h1);
        (uint8 v2, bytes32 r2, bytes32 s2) = vm.sign(uint256(1), h2);

        bytes memory sig1 = abi.encodePacked(r1, s1, v1);
        bytes memory sig2 = abi.encodePacked(r2, s2, v2);

        // The validator for private key 1 is vm.addr(1)
        address expectedValidator = vm.addr(1);

        // Fund and stake the signer
        token.mint(expectedValidator, STAKE_AMOUNT);
        vm.prank(expectedValidator);
        token.stake(STAKE_AMOUNT);

        vm.prank(reporter);
        slashing.reportDoubleSign(header1, header2, sig1, sig2);

        assertTrue(slashing.isJailed(expectedValidator));
        assertEq(
            slashing.getOffenseCount(expectedValidator, NeunodeSlashing.OffenseType.DoubleSign), 1
        );
    }

    function test_reportDoubleSign_rejectsSameBlockHashes() public {
        bytes memory header = abi.encodePacked("same_header");
        bytes memory sig = new bytes(65);

        vm.prank(reporter);
        vm.expectRevert(NeunodeSlashing.SameBlockHashes.selector);
        slashing.reportDoubleSign(header, header, sig, sig);
    }

    function test_reportDoubleSign_rejectsMismatchedSigners() public {
        bytes memory header1 = abi.encodePacked("headerA");
        bytes memory header2 = abi.encodePacked("headerB");
        bytes32 h1 = keccak256(header1);
        bytes32 h2 = keccak256(header2);

        // Sign with different private keys
        (uint8 v1, bytes32 r1, bytes32 s1) = vm.sign(uint256(1), h1);
        (uint8 v2, bytes32 r2, bytes32 s2) = vm.sign(uint256(2), h2);

        bytes memory sig1 = abi.encodePacked(r1, s1, v1);
        bytes memory sig2 = abi.encodePacked(r2, s2, v2);

        vm.prank(reporter);
        vm.expectRevert(NeunodeSlashing.SignaturesDoNotMatchValidator.selector);
        slashing.reportDoubleSign(header1, header2, sig1, sig2);
    }

    // ─── Additional: Evidence Expiry ──────────────────────────────────────

    function test_submitEvidence_rejectsExpired() public {
        // Warp forward so we can create a timestamp in the past
        vm.warp(100 days);

        NeunodeSlashing.SlashingEvidence memory evidence = NeunodeSlashing.SlashingEvidence({
            blockHash1: keccak256("exp_b1"),
            blockHash2: keccak256("exp_b2"),
            blockNumber: 100,
            signature1: "",
            signature2: "",
            extraData: "",
            reporter: validator,
            timestamp: block.timestamp - 8 days // older than 7-day window
        });

        vm.prank(reporter);
        vm.expectRevert(NeunodeSlashing.EvidenceExpired.selector);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, evidence);
    }

    // ─── Additional: Unjail Edge Cases ────────────────────────────────────

    function test_unjail_revertsWhenNotJailed() public {
        vm.prank(validator);
        vm.expectRevert(NeunodeSlashing.ValidatorNotJailed.selector);
        slashing.unjail(validator);
    }

    function test_unjail_revertsWhenNotExpired() public {
        vm.prank(reporter);
        slashing.reportDowntime(validator, 600, 1000);

        // Don't advance blocks
        vm.prank(validator);
        vm.expectRevert(NeunodeSlashing.JailNotExpired.selector);
        slashing.unjail(validator);
    }

    // ─── Additional: Zero Address ─────────────────────────────────────────

    function test_constructor_revertsOnZeroToken() public {
        vm.expectRevert(NeunodeSlashing.ZeroAddress.selector);
        new NeunodeSlashing(address(0));
    }

    function test_reportDowntime_revertsOnZeroAddress() public {
        vm.prank(reporter);
        vm.expectRevert(NeunodeSlashing.ZeroAddress.selector);
        slashing.reportDowntime(address(0), 600, 1000);
    }

    // ─── Additional: Invalid Penalty Bps ──────────────────────────────────

    function test_setPenaltySchedule_revertsOnInvalidBps() public {
        vm.prank(admin);
        vm.expectRevert(abi.encodeWithSelector(NeunodeSlashing.InvalidPenaltyBps.selector, 10_001));
        slashing.setPenaltySchedule(
            NeunodeSlashing.OffenseType.DoubleSign,
            0,
            10_001,
            1000,
            0,
            NeunodeSlashing.PenaltyOutcome.Jailed
        );

        vm.prank(admin);
        vm.expectRevert(abi.encodeWithSelector(NeunodeSlashing.InvalidPenaltyBps.selector, 10_001));
        slashing.setPenaltySchedule(
            NeunodeSlashing.OffenseType.DoubleSign,
            0,
            500,
            10_001,
            0,
            NeunodeSlashing.PenaltyOutcome.Jailed
        );
    }

    // ─── Additional: Slashing Event Count ─────────────────────────────────

    function test_slashingEventCount() public {
        assertEq(slashing.slashingEventCount(), 0);

        NeunodeSlashing.SlashingEvidence memory evidence =
            _buildEvidence(keccak256("block1"), keccak256("block2"), 100, "", validator);
        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, evidence);
        assertEq(slashing.slashingEventCount(), 1);

        vm.prank(reporter);
        slashing.reportDowntime(validator, 600, 1000);
        assertEq(slashing.slashingEventCount(), 2);
    }

    // ─── Additional: Is Evidence Seen ─────────────────────────────────────

    function test_isEvidenceSeen() public {
        bytes32 h1 = keccak256("block1");
        bytes32 h2 = keccak256("block2");
        bytes32 evidenceHash = _computeHash(
            NeunodeSlashing.OffenseType.DoubleSign, h1, h2, 100, "", "", "", validator
        );

        assertFalse(slashing.isEvidenceSeen(evidenceHash));

        NeunodeSlashing.SlashingEvidence memory evidence =
            _buildEvidence(h1, h2, 100, "", validator);
        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, evidence);

        assertTrue(slashing.isEvidenceSeen(evidenceHash));
    }

    // ─── Additional: Governance Abuse Penalty (RateLimited) ───────────────

    function test_governanceAbuse_rateLimited() public {
        NeunodeSlashing.SlashingEvidence memory evidence =
            _buildEvidence(keccak256("gov_b1"), keccak256("gov_b2"), 100, "", validator);

        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.GovernanceAbuse, evidence);

        // RateLimited: NOT jailed, NOT tombstoned
        NeunodeSlashing.ValidatorStatus memory rlStatus = slashing.getValidatorStatus(validator);
        assertFalse(rlStatus.isJailed);
        assertFalse(rlStatus.isTombstoned);

        // But still slashed: 200 bps = 2%
        assertEq(token.stakedBalanceOf(validator), STAKE_AMOUNT - (STAKE_AMOUNT * 200 / 10_000));
    }

    // ─── Additional: Multiple Offense Types ───────────────────────────────

    function test_differentOffenseTypesTrackSeparately() public {
        // DoubleSign first offense
        NeunodeSlashing.SlashingEvidence memory dsEvidence =
            _buildEvidence(keccak256("ds_b1"), keccak256("ds_b2"), 100, "ds", validator);
        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.DoubleSign, dsEvidence);

        // Spamming first offense
        NeunodeSlashing.SlashingEvidence memory spEvidence =
            _buildEvidence(keccak256("sp_b1"), keccak256("sp_b2"), 200, "spam", validator);
        vm.prank(reporter);
        slashing.submitEvidence(NeunodeSlashing.OffenseType.Spamming, spEvidence);

        // Each offense type tracks separately
        assertEq(slashing.getOffenseCount(validator, NeunodeSlashing.OffenseType.DoubleSign), 1);
        assertEq(slashing.getOffenseCount(validator, NeunodeSlashing.OffenseType.Spamming), 1);

        // Total offense count is 2
        NeunodeSlashing.ValidatorStatus memory multiStatus = slashing.getValidatorStatus(validator);
        assertEq(multiStatus.offenseCount, 2);
    }
}
