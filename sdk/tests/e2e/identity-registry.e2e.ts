import { describe, it, expect } from "vitest";
import { useE2E } from "./helpers/fixtures.js";
import { neunodeIdentityAbi } from "../../src/contracts/abi/neunode-identity.js";
import { neunodeRegistryAbi } from "../../src/contracts/abi/neunode-registry.js";
import {
  encodePacked,
  keccak256,
  createWalletClient,
  http,
  parseEventLogs,
} from "viem";
import { foundry } from "viem/chains";
import { privateKeyToAccount } from "viem/accounts";

// Anvil account 1 private key (well-known) for non-controller tests
const ACCOUNT1_PRIVATE_KEY =
  "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as const;

describe("NeunodeIdentity + NeunodeRegistry E2E", () => {
  const { getFixture } = useE2E();

  const ed25519PubKeyHash = keccak256(
    encodePacked(["string"], ["test-ed25519-key"]),
  );

  /** Helper: create a DID from the default deployer account, return didHash */
  async function createDid(): Promise<`0x${string}`> {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const txHash = await walletClient.writeContract({
      abi: neunodeIdentityAbi,
      address: addresses.neunodeIdentity,
      functionName: "createDid",
      args: [ed25519PubKeyHash],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: txHash });
    const didHash = await publicClient.readContract({
      abi: neunodeIdentityAbi,
      address: addresses.neunodeIdentity,
      functionName: "getDidForAddress",
      args: [account.address],
    });
    return didHash as `0x${string}`;
  }

  // ── Identity tests ─────────────────────────────────────────────────────

  describe("Identity", () => {
    it("createDid — creates DID, returns didHash, emits DidCreated", async () => {
      const { walletClient, publicClient, addresses, account } = getFixture();

      const txHash = await walletClient.writeContract({
        abi: neunodeIdentityAbi,
        address: addresses.neunodeIdentity,
        functionName: "createDid",
        args: [ed25519PubKeyHash],
        account: account.address,
      });

      const receipt = await publicClient.waitForTransactionReceipt({
        hash: txHash,
      });

      // Parse DidCreated event
      const logs = parseEventLogs({
        abi: neunodeIdentityAbi,
        logs: receipt.logs,
        eventName: "DidCreated",
      });
      expect(logs).toHaveLength(1);
      expect(logs[0].args.controller.toLowerCase()).toBe(
        account.address.toLowerCase(),
      );
      expect(logs[0].args.timestamp).toBeGreaterThan(0n);

      // Verify DID is retrievable by address
      const didHash = await publicClient.readContract({
        abi: neunodeIdentityAbi,
        address: addresses.neunodeIdentity,
        functionName: "getDidForAddress",
        args: [account.address],
      });
      expect(logs[0].args.didHash).toBe(didHash);
    });

    it("getDocument — returns correct controller, publicKeyHash, active=true", async () => {
      const { publicClient, addresses, account } = getFixture();
      const didHash = await createDid();

      const doc = (await publicClient.readContract({
        abi: neunodeIdentityAbi,
        address: addresses.neunodeIdentity,
        functionName: "getDocument",
        args: [didHash],
      })) as {
        controller: `0x${string}`;
        ed25519PublicKeyHash: `0x${string}`;
        created: bigint;
        updated: bigint;
        active: boolean;
      };

      expect(doc.controller.toLowerCase()).toBe(account.address.toLowerCase());
      expect(doc.ed25519PublicKeyHash).toBe(ed25519PubKeyHash);
      expect(doc.created).toBeGreaterThan(0n);
      expect(doc.updated).toBeGreaterThan(0n);
      expect(doc.active).toBe(true);
    });

    it("getDidForAddress — returns correct didHash for creator", async () => {
      const { publicClient, addresses, account } = getFixture();
      const didHash = await createDid();

      const result = await publicClient.readContract({
        abi: neunodeIdentityAbi,
        address: addresses.neunodeIdentity,
        functionName: "getDidForAddress",
        args: [account.address],
      });

      expect(result).toBe(didHash);
    });

    it("updateController — transfers control, emits DidUpdated", async () => {
      const { walletClient, publicClient, addresses, account } = getFixture();
      const didHash = await createDid();
      const newController =
        "0x70997970C51812dc3A010C7d01b50e0d17dc79C8" as `0x${string}`;

      const txHash = await walletClient.writeContract({
        abi: neunodeIdentityAbi,
        address: addresses.neunodeIdentity,
        functionName: "updateController",
        args: [didHash, newController],
        account: account.address,
      });

      const receipt = await publicClient.waitForTransactionReceipt({
        hash: txHash,
      });

      // Parse DidUpdated event
      const logs = parseEventLogs({
        abi: neunodeIdentityAbi,
        logs: receipt.logs,
        eventName: "DidUpdated",
      });
      expect(logs).toHaveLength(1);
      expect(logs[0].args.newController.toLowerCase()).toBe(
        newController.toLowerCase(),
      );

      // Verify controller changed in state
      const controller = await publicClient.readContract({
        abi: neunodeIdentityAbi,
        address: addresses.neunodeIdentity,
        functionName: "getController",
        args: [didHash],
      });
      expect((controller as string).toLowerCase()).toBe(
        newController.toLowerCase(),
      );
    });

    it("deactivateDid — sets active=false, emits DidDeactivated", async () => {
      const { walletClient, publicClient, addresses, account } = getFixture();
      const didHash = await createDid();

      const txHash = await walletClient.writeContract({
        abi: neunodeIdentityAbi,
        address: addresses.neunodeIdentity,
        functionName: "deactivateDid",
        args: [didHash],
        account: account.address,
      });

      const receipt = await publicClient.waitForTransactionReceipt({
        hash: txHash,
      });

      // Parse DidDeactivated event
      const logs = parseEventLogs({
        abi: neunodeIdentityAbi,
        logs: receipt.logs,
        eventName: "DidDeactivated",
      });
      expect(logs).toHaveLength(1);
      expect(logs[0].args.didHash).toBe(didHash);

      // Verify active=false
      const active = await publicClient.readContract({
        abi: neunodeIdentityAbi,
        address: addresses.neunodeIdentity,
        functionName: "isActive",
        args: [didHash],
      });
      expect(active).toBe(false);
    });

    it("non-controller cannot update DID — reverts", async () => {
      const { publicClient, addresses } = getFixture();
      const didHash = await createDid();

      // Second wallet client using Anvil account 1
      const otherAccount = privateKeyToAccount(ACCOUNT1_PRIVATE_KEY);
      const otherWallet = createWalletClient({
        account: otherAccount,
        chain: foundry,
        transport: http("http://127.0.0.1:8545"),
      });

      await expect(
        otherWallet.writeContract({
          abi: neunodeIdentityAbi,
          address: addresses.neunodeIdentity,
          functionName: "deactivateDid",
          args: [didHash],
          account: otherAccount,
        }),
      ).rejects.toThrow(/NotController|revert/i);
    });
  });

  // ── Registry tests ─────────────────────────────────────────────────────

  describe("Registry", () => {
    it("register — registers agent, emits AgentRegistered", async () => {
      const { walletClient, publicClient, addresses, account } = getFixture();
      const didHash = await createDid();

      const capabilities = '{"gpu":true,"model":"llama-3b"}';
      const endpoint = "/ip4/127.0.0.1/tcp/4001";

      const txHash = await walletClient.writeContract({
        abi: neunodeRegistryAbi,
        address: addresses.neunodeRegistry,
        functionName: "register",
        args: [didHash, capabilities, endpoint],
        account: account.address,
      });

      const receipt = await publicClient.waitForTransactionReceipt({
        hash: txHash,
      });

      // Parse AgentRegistered event
      const logs = parseEventLogs({
        abi: neunodeRegistryAbi,
        logs: receipt.logs,
        eventName: "AgentRegistered",
      });
      expect(logs).toHaveLength(1);
      expect(logs[0].args.didHash).toBe(didHash);
      expect(logs[0].args.controller.toLowerCase()).toBe(
        account.address.toLowerCase(),
      );

      // Verify activeCount incremented
      const count = await publicClient.readContract({
        abi: neunodeRegistryAbi,
        address: addresses.neunodeRegistry,
        functionName: "activeCount",
      });
      expect(count).toBe(1n);
    });

    it("getAgent — returns correct capabilities, endpoint, active=true", async () => {
      const { walletClient, publicClient, addresses, account } = getFixture();
      const didHash = await createDid();

      const capabilities = '{"gpu":true}';
      const endpoint = "https://agent.example.com";

      const regHash = await walletClient.writeContract({
        abi: neunodeRegistryAbi,
        address: addresses.neunodeRegistry,
        functionName: "register",
        args: [didHash, capabilities, endpoint],
        account: account.address,
      });
      await publicClient.waitForTransactionReceipt({ hash: regHash });

      const agent = (await publicClient.readContract({
        abi: neunodeRegistryAbi,
        address: addresses.neunodeRegistry,
        functionName: "getAgent",
        args: [didHash],
      })) as {
        capabilities: string;
        endpoint: string;
        stakeAmount: bigint;
        registeredAt: bigint;
        updatedAt: bigint;
        active: boolean;
      };

      expect(agent.capabilities).toBe(capabilities);
      expect(agent.endpoint).toBe(endpoint);
      expect(agent.active).toBe(true);
      expect(agent.registeredAt).toBeGreaterThan(0n);
      expect(agent.stakeAmount).toBe(0n);
    });

    it("update — updates agent capabilities/endpoint, emits AgentUpdated", async () => {
      const { walletClient, publicClient, addresses, account } = getFixture();
      const didHash = await createDid();

      // Register first
      const regHash = await walletClient.writeContract({
        abi: neunodeRegistryAbi,
        address: addresses.neunodeRegistry,
        functionName: "register",
        args: [didHash, '{"old":true}', "/ip4/0.0.0.0/tcp/4001"],
        account: account.address,
      });
      await publicClient.waitForTransactionReceipt({ hash: regHash });

      // Update with new values
      const newCapabilities = '{"gpu":true,"updated":true}';
      const newEndpoint = "https://updated.example.com";

      const txHash = await walletClient.writeContract({
        abi: neunodeRegistryAbi,
        address: addresses.neunodeRegistry,
        functionName: "update",
        args: [didHash, newCapabilities, newEndpoint],
        account: account.address,
      });

      const receipt = await publicClient.waitForTransactionReceipt({
        hash: txHash,
      });

      // Parse AgentUpdated event
      const logs = parseEventLogs({
        abi: neunodeRegistryAbi,
        logs: receipt.logs,
        eventName: "AgentUpdated",
      });
      expect(logs).toHaveLength(1);
      expect(logs[0].args.didHash).toBe(didHash);

      // Verify updated state
      const agent = (await publicClient.readContract({
        abi: neunodeRegistryAbi,
        address: addresses.neunodeRegistry,
        functionName: "getAgent",
        args: [didHash],
      })) as { capabilities: string; endpoint: string };

      expect(agent.capabilities).toBe(newCapabilities);
      expect(agent.endpoint).toBe(newEndpoint);
    });

    it("deregister — sets active=false, emits AgentDeregistered", async () => {
      const { walletClient, publicClient, addresses, account } = getFixture();
      const didHash = await createDid();

      // Register first
      const regHash = await walletClient.writeContract({
        abi: neunodeRegistryAbi,
        address: addresses.neunodeRegistry,
        functionName: "register",
        args: [didHash, '{"gpu":true}', "/ip4/0.0.0.0/tcp/4001"],
        account: account.address,
      });
      await publicClient.waitForTransactionReceipt({ hash: regHash });

      // Deregister
      const txHash = await walletClient.writeContract({
        abi: neunodeRegistryAbi,
        address: addresses.neunodeRegistry,
        functionName: "deregister",
        args: [didHash],
        account: account.address,
      });

      const receipt = await publicClient.waitForTransactionReceipt({
        hash: txHash,
      });

      // Parse AgentDeregistered event
      const logs = parseEventLogs({
        abi: neunodeRegistryAbi,
        logs: receipt.logs,
        eventName: "AgentDeregistered",
      });
      expect(logs).toHaveLength(1);
      expect(logs[0].args.didHash).toBe(didHash);

      // Verify active=false
      const agent = (await publicClient.readContract({
        abi: neunodeRegistryAbi,
        address: addresses.neunodeRegistry,
        functionName: "getAgent",
        args: [didHash],
      })) as { active: boolean };

      expect(agent.active).toBe(false);

      // Verify activeCount decremented
      const count = await publicClient.readContract({
        abi: neunodeRegistryAbi,
        address: addresses.neunodeRegistry,
        functionName: "activeCount",
      });
      expect(count).toBe(0n);
    });
  });
});
