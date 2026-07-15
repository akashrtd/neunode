/**
 * E2E tests for ModelRegistry + RoyaltySplitter (model lineage DAG & royalty distribution).
 *
 * Covers: model registration, lineage traversal, ERC-2981, proportional splits, events.
 * All tests run against Anvil via `npm run test:e2e`.
 */

import { describe, it, expect } from "vitest";
import { keccak256, encodePacked, parseEventLogs } from "viem";
import { useE2E } from "./helpers/fixtures.js";
import { modelRegistryAbi } from "../../src/contracts/abi/model-registry.js";
import { royaltySplitterAbi } from "../../src/contracts/abi/royalty-splitter.js";
import { neunodeTokenAbi } from "../../src/contracts/abi/neunode-token.js";

const { getFixture } = useE2E();

// ─── Helpers ─────────────────────────────────────────────────────────────

function cid(label: string): `0x${string}` {
  return keccak256(encodePacked(["string"], [label]));
}

const ZERO_ADDR = "0x0000000000000000000000000000000000000000" as const;
const ZERO_HASH = `0x${"00".repeat(32)}` as const;

/** Contribution type enum */
const ContributionType = {
  PreTraining: 0,
  FineTune: 1,
  RL: 2,
  Data: 3,
  Compute: 4,
  Serving: 5,
} as const;

/** Mint tokens to deployer and approve spender. */
async function mintAndApprove(amount: bigint, spender: `0x${string}`) {
  const { walletClient, publicClient, addresses, account } = getFixture();

  const mintHash = await walletClient.writeContract({
    abi: neunodeTokenAbi,
    address: addresses.computeToken,
    functionName: "mint",
    args: [account.address, amount],
    account: account.address,
  });
  await publicClient.waitForTransactionReceipt({ hash: mintHash });

  const approveHash = await walletClient.writeContract({
    abi: neunodeTokenAbi,
    address: addresses.computeToken,
    functionName: "approve",
    args: [spender, amount],
    account: account.address,
  });
  await publicClient.waitForTransactionReceipt({ hash: approveHash });
}

/** Register a model and return its CID. */
async function registerModel(
  label: string,
  parentCids: `0x${string}`[],
  contribution: number,
  metadataURI = "",
): Promise<`0x${string}`> {
  const { walletClient, publicClient, addresses, account } = getFixture();
  const modelCid = cid(label);
  const derivationProofHash = parentCids.length === 0 ? ZERO_HASH : cid(`${label}-proof`);

  const hash = await walletClient.writeContract({
    abi: modelRegistryAbi,
    address: addresses.modelRegistry,
    functionName: "registerModel",
    args: [modelCid, parentCids, contribution, metadataURI, derivationProofHash],
    account: account.address,
  });
  await publicClient.waitForTransactionReceipt({ hash });

  return modelCid;
}

// ═════════════════════════════════════════════════════════════════════════
// ModelRegistry
// ═════════════════════════════════════════════════════════════════════════

describe("ModelRegistry", () => {
  it("registerModel — root model emits ModelRegistered, modelExists returns true", async () => {
    const { publicClient, addresses, account } = getFixture();
    const modelCid = cid("root-model");

    const hash = await getFixture().walletClient.writeContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "registerModel",
      args: [modelCid, [], ContributionType.PreTraining, "ipfs://root", ZERO_HASH],
      account: account.address,
    });
    const receipt = await publicClient.waitForTransactionReceipt({ hash });

    const logs = parseEventLogs({
      abi: modelRegistryAbi,
      logs: receipt.logs,
      eventName: "ModelRegistered",
    });
    expect(logs).toHaveLength(1);
    expect(logs[0].args.cid).toBe(modelCid);
    expect(logs[0].args.contribution).toBe(ContributionType.PreTraining);
    expect(logs[0].args.parentCids).toHaveLength(0);

    const exists = await publicClient.readContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "modelExists",
      args: [modelCid],
    });
    expect(exists).toBe(true);
  });

  it("registerModel — child model links parent-child correctly", async () => {
    const { publicClient, addresses } = getFixture();
    const parentCid = await registerModel("parent-link", [], ContributionType.PreTraining);
    const childCid = await registerModel("child-link", [parentCid], ContributionType.FineTune);

    const parents = await publicClient.readContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "getParents",
      args: [childCid],
    });
    expect(parents).toHaveLength(1);
    expect(parents[0]).toBe(parentCid);

    const children = await publicClient.readContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "getChildren",
      args: [parentCid],
    });
    expect(children).toHaveLength(1);
    expect(children[0]).toBe(childCid);
  });

  it("getModel — returns correct tuple with named fields", async () => {
    const { publicClient, addresses, account } = getFixture();
    const modelCid = await registerModel("getmodel-test", [], ContributionType.RL, "ipfs://meta");

    const model = (await publicClient.readContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "getModel",
      args: [modelCid],
    })) as unknown as {
      cid: `0x${string}`;
      contributor: `0x${string}`;
      contribution: number;
      metadataURI: string;
      registeredAt: bigint;
      exists: boolean;
    };

    expect(model.cid).toBe(modelCid);
    expect(model.contributor.toLowerCase()).toBe(account.address.toLowerCase());
    expect(model.contribution).toBe(ContributionType.RL);
    expect(model.metadataURI).toBe("ipfs://meta");
    expect(model.exists).toBe(true);
    expect(model.registeredAt).toBeGreaterThan(0n);
  });

  it("getLineageDepth — returns 0 for root, 1 for child, 2 for grandchild", async () => {
    const { publicClient, addresses } = getFixture();

    const rootCid = await registerModel("depth-root", [], ContributionType.PreTraining);
    const childCid = await registerModel("depth-child", [rootCid], ContributionType.FineTune);
    const grandchildCid = await registerModel("depth-grandchild", [childCid], ContributionType.Data);

    const rootDepth = await publicClient.readContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "getLineageDepth",
      args: [rootCid],
    });
    expect(rootDepth).toBe(0n);

    const childDepth = await publicClient.readContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "getLineageDepth",
      args: [childCid],
    });
    expect(childDepth).toBe(1n);

    const grandchildDepth = await publicClient.readContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "getLineageDepth",
      args: [grandchildCid],
    });
    expect(grandchildDepth).toBe(2n);
  });

  it("registerModel — emits LineageExtended for child models", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const parentCid = cid("lineage-parent");
    const childCid = cid("lineage-child");

    const regHash = await walletClient.writeContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "registerModel",
      args: [parentCid, [], ContributionType.PreTraining, "", ZERO_HASH],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: regHash });

    const childHash = await walletClient.writeContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "registerModel",
      args: [childCid, [parentCid], ContributionType.FineTune, "", cid("lineage-proof")],
      account: account.address,
    });
    const receipt = await publicClient.waitForTransactionReceipt({ hash: childHash });

    const logs = parseEventLogs({
      abi: modelRegistryAbi,
      logs: receipt.logs,
      eventName: "LineageExtended",
    });
    expect(logs).toHaveLength(1);
    expect(logs[0].args.parentCid).toBe(parentCid);
    expect(logs[0].args.childCid).toBe(childCid);
  });

  it("getModelCount — tracks total registered models", async () => {
    const { publicClient, addresses } = getFixture();

    const countBefore = await publicClient.readContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "getModelCount",
    });

    await registerModel("count-a", [], ContributionType.PreTraining);
    await registerModel("count-b", [], ContributionType.Compute);

    const countAfter = await publicClient.readContract({
      abi: modelRegistryAbi,
      address: addresses.modelRegistry,
      functionName: "getModelCount",
    });
    expect(countAfter - countBefore).toBe(2n);
  });
});

// ═════════════════════════════════════════════════════════════════════════
// ERC-2981 Royalty Info
// ═════════════════════════════════════════════════════════════════════════

describe("ERC-2981 royaltyInfo", () => {
  it("returns correct receiver and amount for existing model", async () => {
    const { publicClient, addresses, account } = getFixture();
    const modelCid = await registerModel("royalty-existing", [], ContributionType.PreTraining);

    const [receiver, royaltyAmount] = (await publicClient.readContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "royaltyInfo",
      args: [BigInt(modelCid), 10_000n],
    })) as unknown as [`0x${string}`, bigint];

    expect(receiver.toLowerCase()).toBe(account.address.toLowerCase());
    expect(royaltyAmount).toBe(1_000n); // 10% of 10_000
  });

  it("returns (0x0, 0) for non-existent model", async () => {
    const { publicClient, addresses } = getFixture();
    const fakeCid = cid("nonexistent-model");

    const [receiver, royaltyAmount] = (await publicClient.readContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "royaltyInfo",
      args: [BigInt(fakeCid), 10_000n],
    })) as unknown as [`0x${string}`, bigint];

    expect(receiver).toBe(ZERO_ADDR);
    expect(royaltyAmount).toBe(0n);
  });
});

// ═════════════════════════════════════════════════════════════════════════
// Royalty Distribution
// ═════════════════════════════════════════════════════════════════════════

describe("Royalty Distribution", () => {
  it("distributeRoyalties — 2-level lineage distributes to ancestors proportionally", async () => {
    const { publicClient, addresses, account } = getFixture();
    const amount = 10_000n;

    // root → child → grandchild (serving model)
    const rootCid = await registerModel("dist-root", [], ContributionType.PreTraining);
    const childCid = await registerModel("dist-child", [rootCid], ContributionType.FineTune);
    const grandchildCid = await registerModel("dist-grandchild", [childCid], ContributionType.Serving);

    await mintAndApprove(amount, addresses.royaltySplitter);

    const hash = await getFixture().walletClient.writeContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "distributeRoyalties",
      args: [grandchildCid, amount, addresses.computeToken],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash });

    // Verify both ancestors received tokens
    const rootBalance = await publicClient.readContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "balanceOf",
      args: [account.address],
    });
    expect(rootBalance).toBeGreaterThan(0n);

    // Verify accumulated tracking
    const accumulated = await publicClient.readContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "accumulatedRoyalties",
      args: [grandchildCid, addresses.computeToken],
    });
    expect(accumulated).toBeGreaterThan(0n);
    expect(accumulated).toBeLessThanOrEqual(amount);
  });

  it("distributeRoyalties — emits RecipientPaid for each ancestor", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const amount = 10_000n;

    const rootCid = await registerModel("event-root", [], ContributionType.PreTraining);
    const childCid = await registerModel("event-child", [rootCid], ContributionType.FineTune);

    await mintAndApprove(amount, addresses.royaltySplitter);

    const hash = await walletClient.writeContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "distributeRoyalties",
      args: [childCid, amount, addresses.computeToken],
      account: account.address,
    });
    const receipt = await publicClient.waitForTransactionReceipt({ hash });

    const paidLogs = parseEventLogs({
      abi: royaltySplitterAbi,
      logs: receipt.logs,
      eventName: "RecipientPaid",
    });
    expect(paidLogs).toHaveLength(1); // 1 ancestor (root)
    expect(paidLogs[0].args.recipient.toLowerCase()).toBe(account.address.toLowerCase());
    expect(paidLogs[0].args.amount).toBeGreaterThan(0n);
    expect(paidLogs[0].args.depth).toBe(1n);

    const distLogs = parseEventLogs({
      abi: royaltySplitterAbi,
      logs: receipt.logs,
      eventName: "RoyaltyDistributed",
    });
    expect(distLogs).toHaveLength(1);
    expect(distLogs[0].args.totalAmount).toBeGreaterThan(0n);
    expect(distLogs[0].args.recipientCount).toBe(1n);
  });

  it("accumulatedRoyalties — tracks per-model per-token total", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const amount1 = 5_000n;
    const amount2 = 3_000n;

    const rootCid = await registerModel("accum-root", [], ContributionType.PreTraining);
    const childCid = await registerModel("accum-child", [rootCid], ContributionType.FineTune);

    // First distribution
    await mintAndApprove(amount1, addresses.royaltySplitter);
    const hash1 = await walletClient.writeContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "distributeRoyalties",
      args: [childCid, amount1, addresses.computeToken],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: hash1 });

    const afterFirst = await publicClient.readContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "accumulatedRoyalties",
      args: [childCid, addresses.computeToken],
    });

    // Second distribution
    await mintAndApprove(amount2, addresses.royaltySplitter);
    const hash2 = await walletClient.writeContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "distributeRoyalties",
      args: [childCid, amount2, addresses.computeToken],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: hash2 });

    const afterSecond = await publicClient.readContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "accumulatedRoyalties",
      args: [childCid, addresses.computeToken],
    });

    expect(afterSecond).toBeGreaterThan(afterFirst);
  });
});

// ═════════════════════════════════════════════════════════════════════════
// Contribution Type Weights & Admin
// ═════════════════════════════════════════════════════════════════════════

describe("Contribution weights & admin", () => {
  it("getContributionTypeWeight — returns correct weights for all types", async () => {
    const { publicClient, addresses } = getFixture();

    const expected = [
      [ContributionType.PreTraining, 100n],
      [ContributionType.FineTune, 80n],
      [ContributionType.RL, 70n],
      [ContributionType.Data, 60n],
      [ContributionType.Compute, 50n],
      [ContributionType.Serving, 30n],
    ] as const;

    for (const [typeIdx, expectedWeight] of expected) {
      const weight = await publicClient.readContract({
        abi: royaltySplitterAbi,
        address: addresses.royaltySplitter,
        functionName: "getContributionTypeWeight",
        args: [typeIdx],
      });
      expect(weight).toBe(expectedWeight);
    }
  });

  it("setProtocolRoyaltyBps — updates protocol royalty cap", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();

    const before = await publicClient.readContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "protocolRoyaltyBps",
    });
    expect(before).toBe(1000n);

    const hash = await walletClient.writeContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "setProtocolRoyaltyBps",
      args: [2000n],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash });

    const after = await publicClient.readContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "protocolRoyaltyBps",
    });
    expect(after).toBe(2000n);
  });

  it("setProtocolRoyaltyBps — emits ProtocolRoyaltyBpsUpdated event", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();

    const hash = await walletClient.writeContract({
      abi: royaltySplitterAbi,
      address: addresses.royaltySplitter,
      functionName: "setProtocolRoyaltyBps",
      args: [1500n],
      account: account.address,
    });
    const receipt = await publicClient.waitForTransactionReceipt({ hash });

    const logs = parseEventLogs({
      abi: royaltySplitterAbi,
      logs: receipt.logs,
      eventName: "ProtocolRoyaltyBpsUpdated",
    });
    expect(logs).toHaveLength(1);
    expect(logs[0].args.oldBps).toBe(1000n);
    expect(logs[0].args.newBps).toBe(1500n);
  });
});
