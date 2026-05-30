// File: tests/unit/governance_test.sol
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../../src/governance/ValidatorGovernance.sol";
import "../../src/reputation/ReputationModule.sol";
import "../../src/token/NeuToken.sol";

/// @title Governance Contract Unit Tests
/// @notice Validates validator set management, reputation-weighted voting, slashing, and epoch transitions
contract GovernanceTest is Test {
    // Core contracts under test
    ValidatorGovernance public governance;
    ReputationModule public reputation;
    NeuToken public neu;

    // Test accounts
    address public validator1 = address(0x1001);
    address public validator2 = address(0x2002);
    address public validator3 = address(0x3003);
    address public validator4 = address(0x4004);
    address public nonValidator = address(0x5005);
    address public governanceOwner = address(0x6006);

    // Constants matching production weights
    uint256 public constant STAKE_WEIGHT = 30;
    uint256 public constant ATTEST_WEIGHT = 25;
    uint256 public constant ACTIVITY_WEIGHT = 20;
    uint256 public constant VERIFY_WEIGHT = 15;
    uint256 public constant TENURE_WEIGHT = 10;

    // Epoch parameters
    uint256 public constant EPOCH_LENGTH = 1000; // blocks
    uint256 public constant MIN_VOTING_POWER = 1 ether;

    function setUp() public {
        // Deploy token
        vm.startPrank(governanceOwner);
        neu = new NeuToken();
        // Reputation module
        reputation = new ReputationModule(
            address(neu),
            STAKE_WEIGHT, ATTEST_WEIGHT, ACTIVITY_WEIGHT,
            VERIFY_WEIGHT, TENURE_WEIGHT
        );
        // Governance contract
        governance = new ValidatorGovernance(
            address(reputation),
            EPOCH_LENGTH,
            MIN_VOTING_POWER
        );
        vm.stopPrank();

        // Initialize validators with stakes
        vm.deal(validator1, 100 ether);
        vm.deal(validator2, 50 ether);
        vm.deal(validator3, 30 ether);
        vm.deal(validator4, 10 ether);

        // Stake and register validators
        vm.startPrank(validator1);
        neu.deposit{value: 100 ether}();
        neu.approve(address(reputation), 100 ether);
        reputation.registerValidator(validator1);
        vm.stopPrank();

        vm.startPrank(validator2);
        neu.deposit{value: 50 ether}();
        neu.approve(address(reputation), 50 ether);
        reputation.registerValidator(validator2);
        vm.stopPrank();

        vm.startPrank(validator3);
        neu.deposit{value: 30 ether}();
        neu.approve(address(reputation), 30 ether);
        reputation.registerValidator(validator3);
        vm.stopPrank();

        // Set initial attestations, activity, verification, tenure scores
        vm.prank(governanceOwner);
        reputation.setAttestationScore(validator1, 80);
        reputation.setActivityScore(validator1, 90);
        reputation.setVerificationScore(validator1, 70);
        reputation.setTenureScore(validator1, 100);

        reputation.setAttestationScore(validator2, 60);
        reputation.setActivityScore(validator2, 70);
        reputation.setVerificationScore(validator2, 80);
        reputation.setTenureScore(validator2, 50);
    }

    // ========================
    // Reputation Score Tests
    // ========================

    function testReputationCalculation() public view {
        // validator1: stake=100, att=80, act=90, ver=70, ten=100
        uint256 totalWeight = reputation.computeTotalScore(validator1);
        // Expected: (100 * 30) + (80 * 25) + (90 * 20) + (70 * 15) + (100 * 10) = 3000+2000+1800+1050+1000 = 8850
        assertEq(totalWeight, 8850, "Total score mismatch for validator1");

        // validator2: stake=50, att=60, act=70, ver=80, ten=50
        uint256 totalWeight2 = reputation.computeTotalScore(validator2);
        // Expected: (50*30)+(60*25)+(70*20)+(80*15)+(50*10) = 1500+1500+1400+1200+500 = 6100
        assertEq(totalWeight2, 6100, "Total score mismatch for validator2");
    }

    function testVotingPowerProportionalToScore() public view {
        // Total score = 8850 + 6100 = 14950 for validators1+2 (validator3 and 4 have 0 attest/activity scores)
        (uint256 vp1, uint256 vp2) = (governance.getVotingPower(validator1), governance.getVotingPower(validator2));
        // Voting power should be proportional to reputation score
        // Since only validator1 and validator2 have non-zero scores, their voting power ratio = 8850:6100
        assertApproxEqRel(vp1 * 6100, vp2 * 8850, 1e15, "Voting power ratio mismatch");
    }

    // ============================
    // Validator Set Management
    // ============================

    function testGrantValidatorRole() public {
        vm.prank(governanceOwner);
        governance.grantValidatorRole(validator4);
        assertTrue(governance.isValidator(validator4), "validator4 should be validator");
    }

    function testRevokeValidatorRole() public {
        vm.prank(governanceOwner);
        governance.revokeValidatorRole(validator1);
        assertFalse(governance.isValidator(validator1), "validator1 should not be validator");
    }

    function testValidatorSetUpdateAfterEpoch() public {
        // Advance blocks to trigger epoch boundary
        vm.roll(block.number + EPOCH_LENGTH + 1);
        // Check that governance triggers update
        vm.prank(governanceOwner);
        governance.executeValidatorSetUpdate();
        // Now the set should include validator4 if granted
    }

    function testCantUpdateSetBeforeEpoch() public {
        vm.prank(governanceOwner);
        vm.expectRevert("Epoch not complete");
        governance.executeValidatorSetUpdate();
    }

    // ============================
    // Reputation-Weighted Voting
    // ============================

    function testProposalCreation() public {
        bytes memory proposalData = abi.encode(ValidatorGovernance.ProposalType.ChangeStakeThreshold, 50 ether);
        vm.prank(validator1);
        governance.createProposal(proposalData, "Increase stake threshold to 50");
        assertEq(governance.proposalCount(), 1, "Proposal count should increment");
    }

    function testNonValidatorCannotCreateProposal() public {
        vm.prank(nonValidator);
        bytes memory proposalData = abi.encode(ValidatorGovernance.ProposalType.ChangeStakeThreshold, 50 ether);
        vm.expectRevert("Not a validator");
        governance.createProposal(proposalData, "hack");
    }

    function testVoteWithWeight() public {
        // Create proposal
        bytes memory proposalData = abi.encode(ValidatorGovernance.ProposalType.MinVotingPower, 2 ether);
        vm.prank(validator1);
        governance.createProposal(proposalData, "Increase min voting power");

        // Vote
        vm.prank(validator2);
        governance.castVote(1, true);

        // Check vote weight: validator2 should have voting power proportional to 6100
        (uint256 forVotes, uint256 againstVotes) = governance.getVoteTallies(1);
        assertGt(forVotes, 0, "For votes should be >0");
        assertEq(againstVotes, 0, "Against votes should be 0");

        // Voting power used should match validator2's voting power
        uint256 expectedVP = governance.getVotingPower(validator2);
        assertApproxEqAbs(forVotes, expectedVP, 1e10, "Vote weight mismatch");
    }

    function testDoubleVotingPrevented() public {
        bytes memory proposalData = abi.encode(ValidatorGovernance.ProposalType.ChangeStakeThreshold, 50 ether);
        vm.prank(validator1);
        governance.createProposal(proposalData, "test");

        vm.prank(validator1);
        governance.castVote(1, true);
        vm.expectRevert("Already voted");
        vm.prank(validator1);
        governance.castVote(1, false);
    }

    // ============================
    // Slashing
    // ============================

    function testSlashReducesStakeAndVotingPower() public {
        uint256 initialStake = reputation.getTotalStake(validator1);
        uint256 initialVotingPower = governance.getVotingPower(validator1);

        // Simulate malicious behavior -> slashing
        vm.prank(governanceOwner);
        governance.slash(validator1, 10 ether, "Double signing");

        uint256 finalStake = reputation.getTotalStake(validator1);
        uint256 finalVotingPower = governance.getVotingPower(validator1);

        assertEq(finalStake, initialStake - 10 ether, "Stake reduction mismatch");
        assertTrue(finalVotingPower < initialVotingPower, "Voting power should decrease");
    }

    function testSlashByNonOwnerReverts() public {
        vm.prank(nonValidator);
        vm.expectRevert("Ownable: caller is not the owner");
        governance.slash(validator1, 5 ether, "unauthorized");
    }

    function testSlashZeroAmountNotAllowed() public {
        vm.prank(governanceOwner);
        vm.expectRevert("Cannot slash zero");
        governance.slash(validator1, 0, "zero slash");
    }

    // ============================
    // Edge Cases
    // ============================

    function testRevertIfValidatorNotRegistered() public {
        vm.prank(governanceOwner);
        vm.expectRevert("Validator not registered");
        governance.getVotingPower(nonValidator);
    }

    function testEpochTransitionTriggersSetUpdate() public {
        // Assume that executeValidatorSetUpdate is called automatically at epoch boundary
        // For now we test that it's callable after sufficient blocks
        vm.roll(block.number + EPOCH_LENGTH);
        // Manually call after roll (in production would be triggered by consensus)
        vm.prank(governanceOwner);
        governance.executeValidatorSetUpdate();
    }

    function testFunctionalValidatorWeightChanges() public {
        // Change activity score and verify voting power updates
        vm.prank(governanceOwner);
        reputation.setActivityScore(validator1, 100);

        uint256 newScore = reputation.computeTotalScore(validator1);
        // Expected: (100*30)+(80*25)+(100*20)+(70*15)+(100*10) = 3000+2000+2000+1050+1000 = 9050
        assertEq(newScore, 9050, "Updated score should be 9050");
    }

    // ============================
    // Fuzz Testing
    // ============================

    function testFuzz_VotingPowerProportionalToStake(uint256 stake1, uint256 stake2) public {
        vm.assume(stake1 >= 1 ether && stake1 <= 1_000_000 ether);
        vm.assume(stake2 >= 1 ether && stake2 <= 1_000_000 ether);

        // Register new validators with exact stake
        address alice = address(0xAAA);
        address bob = address(0xBBB);

        vm.deal(alice, stake1);
        vm.deal(bob, stake2);

        vm.startPrank(alice);
        neu.deposit{value: stake1}();
        neu.approve(address(reputation), stake1);
        reputation.registerValidator(alice);
        vm.stopPrank();

        vm.startPrank(bob);
        neu.deposit{value: stake2}();
        neu.approve(address(reputation), stake2);
        reputation.registerValidator(bob);
        vm.stopPrank();

        // Set identical other scores
        vm.prank(governanceOwner);
        reputation.setAttestationScore(alice, 50);
        reputation.setActivityScore(alice, 50);
        reputation.setVerificationScore(alice, 50);
        reputation.setTenureScore(alice, 50);

        reputation.setAttestationScore(bob, 50);
        reputation.setActivityScore(bob, 50);
        reputation.setVerificationScore(bob, 50);
        reputation.setTenureScore(bob, 50);

        // Total score = stake*30 + (50*25+50*20+50*15+50*10) = 30*stake + 3500
        // So ratio = (30*stake1+3500)/(30*stake2+3500)
        uint256 vpAlice = governance.getVotingPower(alice);
        uint256 vpBob = governance.getVotingPower(bob);
        // Check proper ratio within rounding
        assertApproxEqRel(vpAlice * (30 * stake2 + 3500), vpBob * (30 * stake1 + 3500), 1e17, "Fuzzed voting power ratio");
    }

    function testFuzz_SlashingReducesScore(uint256 slashAmount) public {
        vm.assume(slashAmount >= 0.01 ether && slashAmount < 100 ether);
        uint256 initialScore = reputation.computeTotalScore(validator1);
        vm.prank(governanceOwner);
        governance.slash(validator1, slashAmount, "fuzz slash");
        uint256 finalScore = reputation.computeTotalScore(validator1);
        // Slashing reduces stake component weight
        uint256 expectedStakeReduction = slashAmount * STAKE_WEIGHT;
        assertLe(finalScore, initialScore - expectedStakeReduction + 1e10, "Score reduction too small");
    }
}