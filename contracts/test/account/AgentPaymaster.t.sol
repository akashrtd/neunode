// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {AgentPaymaster} from "../../src/account/AgentPaymaster.sol";
import {IEntryPoint, PackedUserOperation} from "../../src/account/IEntryPoint.sol";

contract MockEntryPoint is IEntryPoint {
    mapping(address account => uint256 amount) public deposits;
    mapping(address account => uint256 amount) public stakes;
    mapping(address account => bool unlocked) public stakeUnlocked;

    function callValidate(
        AgentPaymaster paymaster,
        PackedUserOperation calldata userOp,
        bytes32 userOpHash,
        uint256 maxCost
    ) external view returns (bytes memory, uint256) {
        return paymaster.validatePaymasterUserOp(userOp, userOpHash, maxCost);
    }

    function callPostOp(AgentPaymaster paymaster) external view {
        paymaster.postOp(0, "", 0, 0);
    }

    function depositTo(address account) external payable {
        deposits[account] += msg.value;
    }

    function balanceOf(address account) external view returns (uint256) {
        return deposits[account];
    }

    function withdrawTo(address payable recipient, uint256 amount) external {
        deposits[msg.sender] -= amount;
        recipient.transfer(amount);
    }

    function addStake(uint32) external payable {
        stakes[msg.sender] += msg.value;
    }

    function unlockStake() external {
        stakeUnlocked[msg.sender] = true;
    }

    function withdrawStake(address payable recipient) external {
        require(stakeUnlocked[msg.sender], "locked");
        uint256 amount = stakes[msg.sender];
        stakes[msg.sender] = 0;
        recipient.transfer(amount);
    }
}

contract AgentPaymasterTest is Test {
    uint256 private constant SIGNER_KEY = 0xA11CE;
    address private constant ADMIN = address(0xAD);
    address private constant AGENT = address(0xA6E17);
    uint256 private constant SPONSOR_LIMIT = 0.05 ether;
    bytes32 private constant USER_OP_HASH = keccak256("agent operation");

    MockEntryPoint private entryPoint;
    AgentPaymaster private paymaster;

    function setUp() public {
        entryPoint = new MockEntryPoint();
        paymaster = new AgentPaymaster(address(entryPoint), vm.addr(SIGNER_KEY), ADMIN);
        vm.deal(ADMIN, 10 ether);
    }

    function test_validSponsorAuthorization() public view {
        uint48 validUntil = uint48(block.timestamp + 1 hours);
        uint48 validAfter = uint48(block.timestamp - 1);
        PackedUserOperation memory userOp =
            _signedUserOp(USER_OP_HASH, SPONSOR_LIMIT, validUntil, validAfter, SIGNER_KEY);

        (bytes memory context, uint256 validationData) =
            entryPoint.callValidate(paymaster, userOp, USER_OP_HASH, SPONSOR_LIMIT);

        assertEq(context.length, 0);
        assertEq(validationData & type(uint160).max, 0);
        assertEq(uint48(validationData >> 160), validUntil);
        assertEq(uint48(validationData >> 208), validAfter);
    }

    function test_invalidSignerReturnsSignatureFailure() public view {
        PackedUserOperation memory userOp =
            _signedUserOp(USER_OP_HASH, SPONSOR_LIMIT, uint48(block.timestamp + 1 hours), 0, 0xB0B);
        (, uint256 validationData) =
            entryPoint.callValidate(paymaster, userOp, USER_OP_HASH, SPONSOR_LIMIT);
        assertEq(validationData & type(uint160).max, 1);
    }

    function test_signatureBindsOperationHash() public view {
        PackedUserOperation memory userOp = _signedUserOp(
            USER_OP_HASH, SPONSOR_LIMIT, uint48(block.timestamp + 1 hours), 0, SIGNER_KEY
        );
        (, uint256 validationData) = entryPoint.callValidate(
            paymaster, userOp, keccak256("different operation"), SPONSOR_LIMIT
        );
        assertEq(validationData & type(uint160).max, 1);
    }

    function test_signatureBindsChain() public {
        PackedUserOperation memory userOp = _signedUserOp(
            USER_OP_HASH, SPONSOR_LIMIT, uint48(block.timestamp + 1 hours), 0, SIGNER_KEY
        );
        vm.chainId(block.chainid + 1);
        (, uint256 validationData) =
            entryPoint.callValidate(paymaster, userOp, USER_OP_HASH, SPONSOR_LIMIT);
        assertEq(validationData & type(uint160).max, 1);
    }

    function test_revertsAboveSignedSponsorLimit() public {
        PackedUserOperation memory userOp = _signedUserOp(
            USER_OP_HASH, SPONSOR_LIMIT, uint48(block.timestamp + 1 hours), 0, SIGNER_KEY
        );
        vm.expectRevert(
            abi.encodeWithSelector(
                AgentPaymaster.SponsorLimitExceeded.selector, SPONSOR_LIMIT + 1, SPONSOR_LIMIT
            )
        );
        entryPoint.callValidate(paymaster, userOp, USER_OP_HASH, SPONSOR_LIMIT + 1);
    }

    function test_revertsForWrongPaymasterPrefix() public {
        PackedUserOperation memory userOp = _signedUserOp(
            USER_OP_HASH, SPONSOR_LIMIT, uint48(block.timestamp + 1 hours), 0, SIGNER_KEY
        );
        userOp.paymasterAndData[19] = bytes1(uint8(userOp.paymasterAndData[19]) ^ 1);
        vm.expectRevert(AgentPaymaster.InvalidPaymasterData.selector);
        entryPoint.callValidate(paymaster, userOp, USER_OP_HASH, SPONSOR_LIMIT);
    }

    function test_revertsForInvalidV08SignatureFraming() public {
        PackedUserOperation memory userOp = _signedUserOp(
            USER_OP_HASH, SPONSOR_LIMIT, uint48(block.timestamp + 1 hours), 0, SIGNER_KEY
        );
        userOp.paymasterAndData[userOp.paymasterAndData.length - 1] = 0;
        vm.expectRevert(AgentPaymaster.InvalidPaymasterData.selector);
        entryPoint.callValidate(paymaster, userOp, USER_OP_HASH, SPONSOR_LIMIT);
    }

    function test_onlyEntryPointCanValidateOrPostOp() public {
        PackedUserOperation memory userOp;
        vm.expectRevert(AgentPaymaster.OnlyEntryPoint.selector);
        paymaster.validatePaymasterUserOp(userOp, USER_OP_HASH, 0);
        vm.expectRevert(AgentPaymaster.OnlyEntryPoint.selector);
        paymaster.postOp(0, "", 0, 0);
        entryPoint.callPostOp(paymaster);
    }

    function test_adminCanRotateSignerAndPause() public {
        address nextSigner = address(0xBEEF);
        vm.startPrank(ADMIN);
        paymaster.setSponsorSigner(nextSigner);
        paymaster.pause();
        vm.stopPrank();
        assertEq(paymaster.sponsorSigner(), nextSigner);

        PackedUserOperation memory userOp;
        vm.expectRevert();
        entryPoint.callValidate(paymaster, userOp, USER_OP_HASH, 0);

        vm.prank(ADMIN);
        paymaster.unpause();
        assertFalse(paymaster.paused());
    }

    function test_nonAdminCannotControlPolicyOrFunds() public {
        vm.expectRevert();
        paymaster.setSponsorSigner(address(1));
        vm.expectRevert();
        paymaster.pause();
        vm.expectRevert();
        paymaster.deposit{value: 1 ether}();
    }

    function test_depositAndWithdrawalLifecycle() public {
        vm.prank(ADMIN);
        paymaster.deposit{value: 2 ether}();
        assertEq(paymaster.depositBalance(), 2 ether);

        uint256 beforeBalance = ADMIN.balance;
        vm.prank(ADMIN);
        paymaster.withdrawDeposit(payable(ADMIN), 0.75 ether);
        assertEq(ADMIN.balance, beforeBalance + 0.75 ether);
        assertEq(paymaster.depositBalance(), 1.25 ether);
    }

    function test_stakeLifecycle() public {
        vm.startPrank(ADMIN);
        paymaster.addStake{value: 1 ether}(1 days);
        paymaster.unlockStake();
        uint256 beforeBalance = ADMIN.balance;
        paymaster.withdrawStake(payable(ADMIN));
        vm.stopPrank();
        assertEq(ADMIN.balance, beforeBalance + 1 ether);
        assertEq(entryPoint.stakes(address(paymaster)), 0);
    }

    function _signedUserOp(
        bytes32 userOpHash,
        uint256 sponsorLimit,
        uint48 validUntil,
        uint48 validAfter,
        uint256 signerKey
    ) private view returns (PackedUserOperation memory userOp) {
        bytes32 digest = paymaster.getSponsorshipHash(
            userOpHash, sponsorLimit, validUntil, validAfter
        );
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, digest);
        bytes memory signature = abi.encodePacked(r, s, v);
        userOp.sender = AGENT;
        userOp.paymasterAndData = abi.encodePacked(
            address(paymaster),
            uint128(100_000),
            uint128(50_000),
            abi.encode(validUntil, validAfter, sponsorLimit),
            signature,
            bytes2(uint16(signature.length)),
            paymaster.PAYMASTER_SIG_MAGIC()
        );
    }
}
