// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/NeunodeIdentity.sol";
import "../src/NeunodeRegistry.sol";

/// @title NeunodeRegistryTest — Tests for Agent Registration
contract NeunodeRegistryTest is Test {
    NeunodeIdentity public identity;
    NeunodeRegistry public registry;

    address public alice;
    address public bob;
    bytes32 public aliceDid;
    bytes32 public bobDid;

    string constant CAPABILITIES = '{"inference":true,"training":true}';
    string constant ENDPOINT = "/ip4/1.2.3.4/tcp/4001/p2p/QmTest";
    string constant CAPABILITIES_V2 = '{"inference":true,"training":true,"fine-tuning":true}';
    string constant ENDPOINT_V2 = "/ip4/5.6.7.8/tcp/4001/p2p/QmTest2";

    function setUp() public {
        alice = makeAddr("alice");
        bob = makeAddr("bob");

        identity = new NeunodeIdentity();
        registry = new NeunodeRegistry(address(identity));

        // Create DIDs for alice and bob
        vm.prank(alice);
        aliceDid = identity.createDid(keccak256("alice_ed25519"));

        vm.prank(bob);
        bobDid = identity.createDid(keccak256("bob_ed25519"));
    }

    // ─── Register ─────────────────────────────────────────────────────────

    function testRegister() public {
        vm.prank(alice);
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);

        NeunodeRegistry.AgentRegistration memory agent = registry.getAgent(aliceDid);
        assertEq(agent.didHash, aliceDid);
        assertEq(agent.capabilities, CAPABILITIES);
        assertEq(agent.endpoint, ENDPOINT);
        assertTrue(agent.active);
        assertEq(registry.activeCount(), 1);
    }

    function testRegisterMultipleAgents() public {
        vm.prank(alice);
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);

        vm.prank(bob);
        registry.register(bobDid, CAPABILITIES, ENDPOINT);

        assertEq(registry.activeCount(), 2);
        assertEq(registry.getTotalAgents(), 2);
    }

    function testRevertRegisterNotController() public {
        vm.prank(bob);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeRegistry.NotDidController.selector, aliceDid, bob)
        );
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);
    }

    function testRevertRegisterEmptyCapabilities() public {
        vm.prank(alice);
        vm.expectRevert(NeunodeRegistry.EmptyCapabilities.selector);
        registry.register(aliceDid, "", ENDPOINT);
    }

    function testRevertRegisterEmptyEndpoint() public {
        vm.prank(alice);
        vm.expectRevert(NeunodeRegistry.EmptyEndpoint.selector);
        registry.register(aliceDid, CAPABILITIES, "");
    }

    function testRevertRegisterInactiveDid() public {
        vm.prank(alice);
        identity.deactivateDid(aliceDid);

        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(NeunodeRegistry.DidNotActive.selector, aliceDid));
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);
    }

    function testRevertRegisterDuplicate() public {
        vm.prank(alice);
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);

        vm.prank(alice);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeRegistry.AgentAlreadyRegistered.selector, aliceDid)
        );
        registry.register(aliceDid, CAPABILITIES_V2, ENDPOINT_V2);
    }

    // ─── Update ───────────────────────────────────────────────────────────

    function testUpdate() public {
        vm.prank(alice);
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);

        vm.prank(alice);
        registry.update(aliceDid, CAPABILITIES_V2, ENDPOINT_V2);

        NeunodeRegistry.AgentRegistration memory agent = registry.getAgent(aliceDid);
        assertEq(agent.capabilities, CAPABILITIES_V2);
        assertEq(agent.endpoint, ENDPOINT_V2);
    }

    function testRevertUpdateNotController() public {
        vm.prank(alice);
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);

        vm.prank(bob);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeRegistry.NotDidController.selector, aliceDid, bob)
        );
        registry.update(aliceDid, CAPABILITIES_V2, ENDPOINT_V2);
    }

    function testRevertUpdateNotActive() public {
        vm.prank(alice);
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);

        vm.prank(alice);
        registry.deregister(aliceDid);

        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(NeunodeRegistry.AgentNotActive.selector, aliceDid));
        registry.update(aliceDid, CAPABILITIES_V2, ENDPOINT_V2);
    }

    // ─── Deregister ───────────────────────────────────────────────────────

    function testDeregister() public {
        vm.prank(alice);
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);

        vm.prank(alice);
        registry.deregister(aliceDid);

        NeunodeRegistry.AgentRegistration memory agent = registry.getAgent(aliceDid);
        assertFalse(agent.active);
        assertEq(registry.activeCount(), 0);
    }

    function testRevertDeregisterNotController() public {
        vm.prank(alice);
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);

        vm.prank(bob);
        vm.expectRevert(
            abi.encodeWithSelector(NeunodeRegistry.NotDidController.selector, aliceDid, bob)
        );
        registry.deregister(aliceDid);
    }

    // ─── Get Active Agents ────────────────────────────────────────────────

    function testGetActiveAgents() public {
        vm.prank(alice);
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);

        vm.prank(bob);
        registry.register(bobDid, CAPABILITIES, ENDPOINT);

        bytes32[] memory active = registry.getActiveAgents();
        assertEq(active.length, 2);
    }

    function testGetActiveAgentsAfterDeregister() public {
        vm.prank(alice);
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);

        vm.prank(bob);
        registry.register(bobDid, CAPABILITIES, ENDPOINT);

        vm.prank(alice);
        registry.deregister(aliceDid);

        bytes32[] memory active = registry.getActiveAgents();
        assertEq(active.length, 1);
        assertEq(active[0], bobDid);
    }

    // ─── Revert Constructor ───────────────────────────────────────────────

    function testRevertConstructorZeroAddress() public {
        vm.expectRevert("invalid identity address");
        new NeunodeRegistry(address(0));
    }

    // ─── Events ───────────────────────────────────────────────────────────

    function testAgentRegisteredEvent() public {
        vm.prank(alice);
        vm.expectEmit(true, true, false, true);
        emit NeunodeRegistry.AgentRegistered(aliceDid, alice, block.timestamp);
        registry.register(aliceDid, CAPABILITIES, ENDPOINT);
    }
}
