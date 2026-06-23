// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/NeunodeIdentity.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

/// @title NeunodeIdentityTest — Tests for DID Registry
contract NeunodeIdentityTest is Test {
    NeunodeIdentity public identity;
    MockStakeSource public stakeSource;
    address public alice;
    address public bob;
    bytes32 public alicePubKeyHash;

    uint256 public constant MIN_STAKE = 1000e18;

    function setUp() public {
        identity = new NeunodeIdentity();
        alice = makeAddr("alice");
        bob = makeAddr("bob");
        alicePubKeyHash = keccak256("alice_ed25519_pubkey");

        // Sybil-resistance gate: register a stake source + minimum for the suite.
        stakeSource = new MockStakeSource();
        identity.setStakeSource(address(stakeSource));
        identity.setMinRegistrationStake(MIN_STAKE);
    }

    // ─── Create DID ───────────────────────────────────────────────────────

    function testCreateDid() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        assertEq(identity.getController(didHash), alice);
        assertTrue(identity.isActive(didHash));
        assertEq(identity.getDidForAddress(alice), didHash);

        NeunodeIdentity.DidDocument memory doc = identity.getDocument(didHash);
        assertEq(doc.controller, alice);
        assertEq(doc.ed25519PublicKeyHash, alicePubKeyHash);
        assertTrue(doc.active);
    }

    function testRevertCreateDidZeroPubKey() public {
        vm.prank(alice);
        vm.expectRevert(NeunodeIdentity.InvalidPublicKeyHash.selector);
        identity.createDid(bytes32(0));
    }

    function testRevertCreateDidAlreadyExists() public {
        vm.prank(alice);
        identity.createDid(alicePubKeyHash);

        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeIdentity.AddressAlreadyHasDid.selector, alice)
        );
        identity.createDid(keccak256("different_key"));
    }

    // ─── Update Controller ────────────────────────────────────────────────

    function testUpdateController() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        vm.prank(alice);
        identity.updateController(didHash, bob);

        assertEq(identity.getController(didHash), bob);
        assertEq(identity.getDidForAddress(bob), didHash);
        assertEq(identity.getDidForAddress(alice), bytes32(0));
    }

    function testRevertUpdateControllerNotOwner() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        vm.prank(bob);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeIdentity.NotController.selector, didHash, bob)
        );
        identity.updateController(didHash, bob);
    }

    function testRevertUpdateControllerToZero() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        vm.prank(alice);
        vm.expectRevert(NeunodeIdentity.InvalidPublicKeyHash.selector);
        identity.updateController(didHash, address(0));
    }

    function testRevertUpdateControllerToAddressWithDid() public {
        vm.prank(alice);
        bytes32 aliceDid = identity.createDid(alicePubKeyHash);

        vm.prank(bob);
        bytes32 bobDid = identity.createDid(keccak256("bob_key"));

        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(NeunodeIdentity.AddressAlreadyHasDid.selector, bob));
        identity.updateController(aliceDid, bob);
    }

    // ─── Deactivate ───────────────────────────────────────────────────────

    function testDeactivateDid() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        vm.prank(alice);
        identity.deactivateDid(didHash);

        assertFalse(identity.isActive(didHash));
        assertEq(identity.getDidForAddress(alice), bytes32(0));
    }

    function testRevertDeactivateNotController() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        vm.prank(bob);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeIdentity.NotController.selector, didHash, bob)
        );
        identity.deactivateDid(didHash);
    }

    function testRevertDeactivateTwice() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        vm.prank(alice);
        identity.deactivateDid(didHash);

        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(NeunodeIdentity.DidNotActive.selector, didHash));
        identity.deactivateDid(didHash);
    }

    // ─── Verify Signature ─────────────────────────────────────────────────

    function testVerifySignature() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        bytes32 messageHash = keccak256("test_message");
        bytes32 ethSignedHash =
            keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash));

        // Sign with alice's key (foundry cheatcode)
        (uint8 v, bytes32 r, bytes32 s) =
            vm.sign(uint256(keccak256("alice_private_key")), ethSignedHash);
        // Note: This won't match alice's address since we can't get alice's real private key
        // So we test with a funded signer instead

        // Create DID for this contract address
        bytes32 myPubKeyHash = keccak256("test_key");
        bytes32 myDid = identity.createDid(myPubKeyHash);

        // Sign with this contract's key (test contract is the controller)
        (v, r, s) = vm.sign(0x1, ethSignedHash);
        // This won't work either since address(1) isn't the controller
        // Instead, let's verify the function returns false for wrong signer
        bytes memory sig = abi.encodePacked(r, s, v);
        assertFalse(identity.verifySignature(myDid, messageHash, sig));
    }

    function testVerifySignatureInactiveDid() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        vm.prank(alice);
        identity.deactivateDid(didHash);

        bytes memory emptySig = new bytes(65);
        assertFalse(identity.verifySignature(didHash, keccak256("test"), emptySig));
    }

    // ─── Get Document ─────────────────────────────────────────────────────

    function testRevertGetDocumentNotFound() public {
        bytes32 fakeDid = keccak256("nonexistent");
        vm.expectRevert(abi.encodeWithSelector(NeunodeIdentity.DidNotFound.selector, fakeDid));
        identity.getDocument(fakeDid);
    }

    // ─── Events ───────────────────────────────────────────────────────────

    function testDidCreatedEvent() public {
        vm.prank(alice);
        vm.expectEmit(true, true, false, true);
        emit NeunodeIdentity.DidCreated(
            keccak256(abi.encodePacked(alice, alicePubKeyHash, block.timestamp)),
            alice,
            block.timestamp
        );
        identity.createDid(alicePubKeyHash);
    }

    // ─── ECDSA Malleability ────────────────────────────────────────────────

    function testVerifySignatureValid() public {
        // Fund a known signer so we can create a DID for them
        uint256 signerPk = 0xA11CE;
        address signer = vm.addr(signerPk);

        vm.prank(signer);
        bytes32 didHash = identity.createDid(keccak256("signer_ed25519_key"));

        bytes32 messageHash = keccak256("important_message");
        bytes32 ethSignedHash =
            keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash));

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerPk, ethSignedHash);
        bytes memory sig = abi.encodePacked(r, s, v);

        assertTrue(identity.verifySignature(didHash, messageHash, sig));
    }

    function testRevertMalleableSignatureHighS() public {
        // Create DID for a known signer
        uint256 signerPk = 0xA11CE;
        address signer = vm.addr(signerPk);

        vm.prank(signer);
        bytes32 didHash = identity.createDid(keccak256("signer_ed25519_key"));

        bytes32 messageHash = keccak256("important_message");
        bytes32 ethSignedHash =
            keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash));

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerPk, ethSignedHash);

        // Flip s to its malleable counterpart (n - s), which is > n/2
        uint256 secp256k1n = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;
        bytes32 malleableS = bytes32(secp256k1n - uint256(s));
        // Adjust v (flip between 27 and 28)
        uint8 malleableV = v == 27 ? 28 : 27;

        bytes memory malleableSig = abi.encodePacked(r, malleableS, malleableV);

        // OZ ECDSA.recover reverts with ECDSAInvalidSignatureS for non-low-s signatures
        bytes32 expectedS = malleableS;
        vm.expectRevert(abi.encodeWithSelector(ECDSA.ECDSAInvalidSignatureS.selector, expectedS));
        identity.verifySignature(didHash, messageHash, malleableSig);
    }

    function testVerifySignatureWrongSigner() public {
        uint256 signerPk = 0xA11CE;
        address signer = vm.addr(signerPk);

        vm.prank(signer);
        bytes32 didHash = identity.createDid(keccak256("signer_ed25519_key"));

        bytes32 messageHash = keccak256("important_message");
        bytes32 ethSignedHash =
            keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash));

        // Sign with a different key
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(0xB0B, ethSignedHash);
        bytes memory sig = abi.encodePacked(r, s, v);

        assertFalse(identity.verifySignature(didHash, messageHash, sig));
    }

    // ─── Network Registration (Sybil Resistance) ─────────────────────────
    // DID creation stays free (key generation); participation in reputation /
    // validator eligibility requires a slashable stake ≥ minRegistrationStake.

    function testRegisterForNetworkRequiresStake() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        // alice holds zero stake → must be rejected
        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(
                NeunodeIdentity.InsufficientRegistrationStake.selector, alice, uint256(0), MIN_STAKE
            )
        );
        identity.registerForNetwork(didHash);
    }

    function testRegisterForNetworkSucceedsWithStake() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);
        stakeSource.setStaked(alice, MIN_STAKE);

        vm.prank(alice);
        vm.expectEmit(true, true, false, true);
        emit NeunodeIdentity.NetworkRegistered(didHash, alice, MIN_STAKE);
        identity.registerForNetwork(didHash);

        assertTrue(identity.isRegistered(didHash));
    }

    function testRegisterForNetworkFreeWhenNoMinimum() public {
        // Unconfigured gate (min = 0) stays backward-compatible: no stake needed.
        NeunodeIdentity bare = new NeunodeIdentity();
        vm.prank(alice);
        bytes32 didHash = bare.createDid(alicePubKeyHash);

        vm.prank(alice);
        bare.registerForNetwork(didHash);
        assertTrue(bare.isRegistered(didHash));
    }

    function testRevertRegisterNotController() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        vm.prank(bob);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeIdentity.NotController.selector, didHash, bob)
        );
        identity.registerForNetwork(didHash);
    }

    function testRevertRegisterInactiveDid() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);
        stakeSource.setStaked(alice, MIN_STAKE);

        vm.prank(alice);
        identity.deactivateDid(didHash);

        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(NeunodeIdentity.DidNotActive.selector, didHash));
        identity.registerForNetwork(didHash);
    }

    function testDeregisterFromNetwork() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);
        stakeSource.setStaked(alice, MIN_STAKE);

        vm.prank(alice);
        identity.registerForNetwork(didHash);

        vm.prank(alice);
        identity.deregisterFromNetwork(didHash);
        assertFalse(identity.isRegistered(didHash));
    }

    function testRevertDeregisterUnregistered() public {
        vm.prank(alice);
        bytes32 didHash = identity.createDid(alicePubKeyHash);

        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(NeunodeIdentity.NotRegistered.selector, didHash));
        identity.deregisterFromNetwork(didHash);
    }

    function testRevertSetStakeSourceNotOwner() public {
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(NeunodeIdentity.NotOwner.selector, alice));
        identity.setStakeSource(address(0xBEEF));
    }
}

/// @dev Minimal stake oracle for tests — NeunodeToken satisfies this interface in prod.
contract MockStakeSource {
    mapping(address => uint256) private _staked;

    function setStaked(address account, uint256 amount) external {
        _staked[account] = amount;
    }

    function stakedBalanceOf(address account) external view returns (uint256) {
        return _staked[account];
    }
}
