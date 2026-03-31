/**
 * E2E tests for NeunodeGovernance proposal lifecycle and EIP-2535 Diamond proxy.
 *
 * Governance: propose → vote → succeed → queue → execute + cancel + parameter updates.
 * Diamond:    loupe queries (facets, facetAddresses, facetFunctionSelectors, facetAddress)
 *             and diamondCut to add new selectors.
 */

import { describe, it, expect } from "vitest";
import {
  parseEventLogs,
  encodeFunctionData,
  getFunctionSelector,
  createWalletClient,
  http,
} from "viem";
import { foundry } from "viem/chains";
import { privateKeyToAccount } from "viem/accounts";
import { useE2E } from "./helpers/fixtures.js";
import { neunodeGovernanceAbi } from "../../src/contracts/abi/neunode-governance.js";
import {
  diamondCutFacetAbi,
  diamondLoupeFacetAbi,
} from "../../src/contracts/abi/diamond.js";
import { neunodeTokenAbi } from "../../src/contracts/abi/neunode-token.js";

const { getFixture } = useE2E();

const ACCOUNT_1_PK =
  "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as const;

const PROPOSAL_THRESHOLD = 100000000000000000000n; // 100e18

// ─── Helpers ─────────────────────────────────────────────────────────────

/** Mint + stake tokens, then checkpoint voting power. */
async function stakeAndCheckpoint(
  accountAddress: `0x${string}`,
  amount: bigint,
) {
  const f = getFixture();

  const mintHash = await f.walletClient.writeContract({
    abi: neunodeTokenAbi,
    address: f.addresses.computeToken,
    functionName: "mint",
    args: [accountAddress, amount],
    account: f.account,
  });
  await f.publicClient.waitForTransactionReceipt({ hash: mintHash });

  const stakeHash = await f.walletClient.writeContract({
    abi: neunodeTokenAbi,
    address: f.addresses.computeToken,
    functionName: "stake",
    args: [amount],
    account: f.account,
  });
  await f.publicClient.waitForTransactionReceipt({ hash: stakeHash });

  const cpHash = await f.walletClient.writeContract({
    abi: neunodeGovernanceAbi,
    address: f.addresses.neunodeGovernance,
    functionName: "checkpoint",
    args: [],
    account: f.account,
  });
  await f.publicClient.waitForTransactionReceipt({ hash: cpHash });
}

/** Create a no-op proposal (empty targets/values/calldatas). Returns proposalId. */
async function createProposal(description: string): Promise<bigint> {
  const f = getFixture();

  const hash = await f.walletClient.writeContract({
    abi: neunodeGovernanceAbi,
    address: f.addresses.neunodeGovernance,
    functionName: "propose",
    args: [
      [f.addresses.neunodeGovernance],
      [0n],
      ["0x" as `0x${string}`],
      description,
    ],
    account: f.account,
  });
  const receipt = await f.publicClient.waitForTransactionReceipt({ hash });

  const logs = parseEventLogs({
    abi: neunodeGovernanceAbi,
    logs: receipt.logs,
    eventName: "ProposalCreated",
  });
  return logs[0].args.proposalId;
}

/** Advance Anvil time past a target timestamp. */
async function advanceTo(targetTimestamp: bigint) {
  const f = getFixture();
  await f.testClient.setNextBlockTimestamp({ timestamp: Number(targetTimestamp) });
  await f.testClient.mine({ blocks: 1 });
}

/** Read voteStart and voteEnd for a proposal. */
async function getProposalTiming(proposalId: bigint) {
  const f = getFixture();
  const result = await f.publicClient.readContract({
    abi: neunodeGovernanceAbi,
    address: f.addresses.neunodeGovernance,
    functionName: "getProposal",
    args: [proposalId],
  });
  const fields = result as unknown as [
    `0x${string}`, // proposer_
    bigint,        // voteStart
    bigint,        // voteEnd
    bigint,        // forVotes
    bigint,        // againstVotes
    bigint,        // abstainVotes
    bigint,        // snapshotBlock_
    boolean,       // executed_
    boolean,       // cancelled_
    bigint,        // queuedAt
  ];
  return { voteStart: fields[1], voteEnd: fields[2] };
}

// ══════════════════════════════════════════════════════════════════════════
// Governance Tests
// ══════════════════════════════════════════════════════════════════════════

describe("NeunodeGovernance", () => {
  it("propose — creates proposal when proposer has staked >= threshold", async () => {
    const f = getFixture();
    await stakeAndCheckpoint(f.account.address, PROPOSAL_THRESHOLD);

    const hash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "propose",
      args: [
        [f.addresses.neunodeGovernance],
        [0n],
        ["0x" as `0x${string}`],
        "Test proposal",
      ],
      account: f.account,
    });
    const receipt = await f.publicClient.waitForTransactionReceipt({ hash });

    const logs = parseEventLogs({
      abi: neunodeGovernanceAbi,
      logs: receipt.logs,
      eventName: "ProposalCreated",
    });
    expect(logs).toHaveLength(1);
    expect(logs[0].args.proposer.toLowerCase()).toBe(
      f.account.address.toLowerCase(),
    );
    expect(logs[0].args.voteStart).toBeGreaterThan(0n);
    expect(logs[0].args.voteEnd).toBeGreaterThan(logs[0].args.voteStart);

    const proposalId = logs[0].args.proposalId;
    const state = await f.publicClient.readContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "state",
      args: [proposalId],
    });
    expect(state).toBe(0); // Pending
  });

  it("castVote — records For vote and updates vote counts", async () => {
    const f = getFixture();
    await stakeAndCheckpoint(f.account.address, PROPOSAL_THRESHOLD);
    const proposalId = await createProposal("Vote test");

    const { voteStart } = await getProposalTiming(proposalId);
    await advanceTo(voteStart + 1n);

    const voteHash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "castVote",
      args: [proposalId, 1], // For
      account: f.account,
    });
    const voteReceipt = await f.publicClient.waitForTransactionReceipt({
      hash: voteHash,
    });

    const logs = parseEventLogs({
      abi: neunodeGovernanceAbi,
      logs: voteReceipt.logs,
      eventName: "VoteCast",
    });
    expect(logs).toHaveLength(1);
    expect(logs[0].args.support).toBe(1);
    expect(logs[0].args.weight).toBe(PROPOSAL_THRESHOLD);

    const result = await f.publicClient.readContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "getProposal",
      args: [proposalId],
    });
    const fields = result as unknown as [
      `0x${string}`, bigint, bigint, bigint, bigint, bigint,
      bigint, boolean, boolean, bigint,
    ];
    expect(fields[3]).toBe(PROPOSAL_THRESHOLD); // forVotes
  });

  it("proposal succeeds after voting period with For majority", async () => {
    const f = getFixture();
    await stakeAndCheckpoint(f.account.address, PROPOSAL_THRESHOLD);
    const proposalId = await createProposal("Succeed test");

    const { voteStart } = await getProposalTiming(proposalId);
    await advanceTo(voteStart + 1n);

    const voteHash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "castVote",
      args: [proposalId, 1],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: voteHash });

    const { voteEnd } = await getProposalTiming(proposalId);
    await advanceTo(voteEnd + 1n);

    const state = await f.publicClient.readContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "state",
      args: [proposalId],
    });
    expect(state).toBe(2); // Succeeded
  });

  it("queue — moves succeeded proposal to Queued", async () => {
    const f = getFixture();
    await stakeAndCheckpoint(f.account.address, PROPOSAL_THRESHOLD);
    const proposalId = await createProposal("Queue test");

    const { voteStart } = await getProposalTiming(proposalId);
    await advanceTo(voteStart + 1n);

    const voteHash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "castVote",
      args: [proposalId, 1],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: voteHash });

    const { voteEnd } = await getProposalTiming(proposalId);
    await advanceTo(voteEnd + 1n);

    const queueHash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "queue",
      args: [proposalId],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: queueHash });

    const state = await f.publicClient.readContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "state",
      args: [proposalId],
    });
    expect(state).toBe(4); // Queued
  });

  it("execute — executes queued proposal after timelock", async () => {
    const f = getFixture();
    await stakeAndCheckpoint(f.account.address, PROPOSAL_THRESHOLD);

    const GOVERNANCE_ROLE = await f.publicClient.readContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "GOVERNANCE_ROLE",
    });

    const grantHash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "grantRole",
      args: [GOVERNANCE_ROLE, f.addresses.neunodeGovernance],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: grantHash });

    const newVotingDelay = 172800n;
    const calldata = encodeFunctionData({
      abi: neunodeGovernanceAbi,
      functionName: "setVotingDelay",
      args: [newVotingDelay],
    });

    const hash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "propose",
      args: [
        [f.addresses.neunodeGovernance],
        [0n],
        [calldata],
        "Set voting delay via governance",
      ],
      account: f.account,
    });
    const receipt = await f.publicClient.waitForTransactionReceipt({ hash });
    const proposalLogs = parseEventLogs({
      abi: neunodeGovernanceAbi,
      logs: receipt.logs,
      eventName: "ProposalCreated",
    });
    const proposalId = proposalLogs[0].args.proposalId;

    const { voteStart } = await getProposalTiming(proposalId);
    await advanceTo(voteStart + 1n);

    const voteHash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "castVote",
      args: [proposalId, 1],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: voteHash });

    const { voteEnd } = await getProposalTiming(proposalId);
    await advanceTo(voteEnd + 1n);

    const queueHash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "queue",
      args: [proposalId],
      account: f.account,
    });
    const queueReceipt = await f.publicClient.waitForTransactionReceipt({
      hash: queueHash,
    });
    const queuedLogs = parseEventLogs({
      abi: neunodeGovernanceAbi,
      logs: queueReceipt.logs,
      eventName: "ProposalQueued",
    });
    const eta = queuedLogs[0].args.eta;
    await advanceTo(eta + 1n);

    const execHash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "execute",
      args: [proposalId],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: execHash });

    const state = await f.publicClient.readContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "state",
      args: [proposalId],
    });
    expect(state).toBe(5); // Executed

    const updatedDelay = await f.publicClient.readContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "votingDelay",
    });
    expect(updatedDelay).toBe(newVotingDelay);
  });

  it("cancel — proposer cancels while Pending", async () => {
    const f = getFixture();
    await stakeAndCheckpoint(f.account.address, PROPOSAL_THRESHOLD);
    const proposalId = await createProposal("Cancel test");

    const cancelHash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "cancel",
      args: [proposalId],
      account: f.account,
    });
    const cancelReceipt = await f.publicClient.waitForTransactionReceipt({
      hash: cancelHash,
    });

    const logs = parseEventLogs({
      abi: neunodeGovernanceAbi,
      logs: cancelReceipt.logs,
      eventName: "ProposalCancelled",
    });
    expect(logs).toHaveLength(1);
    expect(logs[0].args.proposalId).toBe(proposalId);

    const state = await f.publicClient.readContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "state",
      args: [proposalId],
    });
    expect(state).toBe(7); // Cancelled
  });

  it("parameter setters update governance config", async () => {
    const f = getFixture();

    const setDelayHash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "setVotingDelay",
      args: [50000n],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: setDelayHash });

    const setThresholdHash = await f.walletClient.writeContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "setProposalThreshold",
      args: [200000000000000000000n], // 200e18
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({
      hash: setThresholdHash,
    });

    const delay = await f.publicClient.readContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "votingDelay",
    });
    expect(delay).toBe(50000n);

    const threshold = await f.publicClient.readContract({
      abi: neunodeGovernanceAbi,
      address: f.addresses.neunodeGovernance,
      functionName: "proposalThreshold",
    });
    expect(threshold).toBe(200000000000000000000n);
  });

  it("propose reverts when proposer is below threshold", async () => {
    const f = getFixture();

    const otherAccount = privateKeyToAccount(ACCOUNT_1_PK);
    const otherClient = createWalletClient({
      account: otherAccount,
      chain: foundry,
      transport: http("http://127.0.0.1:8545"),
    });

    await expect(
      otherClient.writeContract({
        abi: neunodeGovernanceAbi,
        address: f.addresses.neunodeGovernance,
        functionName: "propose",
        args: [
          [f.addresses.neunodeGovernance],
          [0n],
          ["0x" as `0x${string}`],
          "Should fail",
        ],
        account: otherAccount,
      }),
    ).rejects.toThrow();
  });
});

// ══════════════════════════════════════════════════════════════════════════
// Diamond Tests
// ══════════════════════════════════════════════════════════════════════════

describe("Diamond Proxy (EIP-2535)", () => {
  it("facets() returns registered facets with selectors", async () => {
    const f = getFixture();

    const facets = (await f.publicClient.readContract({
      abi: diamondLoupeFacetAbi,
      address: f.addresses.diamond,
      functionName: "facets",
    })) as unknown as { facetAddress: `0x${string}`; functionSelectors: `0x${string}`[] }[];

    expect(facets.length).toBeGreaterThanOrEqual(2);

    const allSelectors = facets.flatMap((fac) => fac.functionSelectors);
    expect(allSelectors.length).toBeGreaterThanOrEqual(5);

    const hasCut = allSelectors.some(
      (s) => s === getFunctionSelector("diamondCut((address,uint8,bytes4[])[],address,bytes)"),
    );
    const hasFacets = allSelectors.some(
      (s) => s === getFunctionSelector("facets()"),
    );
    expect(hasCut).toBe(true);
    expect(hasFacets).toBe(true);
  });

  it("facetAddresses() returns all facet addresses", async () => {
    const f = getFixture();

    const addresses = (await f.publicClient.readContract({
      abi: diamondLoupeFacetAbi,
      address: f.addresses.diamond,
      functionName: "facetAddresses",
    })) as unknown as `0x${string}`[];

    expect(addresses.length).toBeGreaterThanOrEqual(2);

    const lowerAddresses = addresses.map((a) => a.toLowerCase());
    expect(lowerAddresses).toContain(f.addresses.diamondCutFacet.toLowerCase());
    expect(lowerAddresses).toContain(f.addresses.diamondLoupeFacet.toLowerCase());
  });

  it("facetFunctionSelectors(address) returns selectors for each facet", async () => {
    const f = getFixture();

    const cutSelectors = (await f.publicClient.readContract({
      abi: diamondLoupeFacetAbi,
      address: f.addresses.diamond,
      functionName: "facetFunctionSelectors",
      args: [f.addresses.diamondCutFacet],
    })) as unknown as `0x${string}`[];

    expect(cutSelectors).toHaveLength(1);
    expect(cutSelectors[0]).toBe(
      getFunctionSelector("diamondCut((address,uint8,bytes4[])[],address,bytes)"),
    );

    const loupeSelectors = (await f.publicClient.readContract({
      abi: diamondLoupeFacetAbi,
      address: f.addresses.diamond,
      functionName: "facetFunctionSelectors",
      args: [f.addresses.diamondLoupeFacet],
    })) as unknown as `0x${string}`[];

    expect(loupeSelectors).toHaveLength(4);
  });

  it("facetAddress(bytes4) resolves to correct facet", async () => {
    const f = getFixture();

    const cutSelector = getFunctionSelector(
      "diamondCut((address,uint8,bytes4[])[],address,bytes)",
    );
    const cutFacet = await f.publicClient.readContract({
      abi: diamondLoupeFacetAbi,
      address: f.addresses.diamond,
      functionName: "facetAddress",
      args: [cutSelector],
    });
    expect((cutFacet as string).toLowerCase()).toBe(
      f.addresses.diamondCutFacet.toLowerCase(),
    );

    const facetsSelector = getFunctionSelector("facets()");
    const loupeFacet = await f.publicClient.readContract({
      abi: diamondLoupeFacetAbi,
      address: f.addresses.diamond,
      functionName: "facetAddress",
      args: [facetsSelector],
    });
    expect((loupeFacet as string).toLowerCase()).toBe(
      f.addresses.diamondLoupeFacet.toLowerCase(),
    );
  });

  it("diamondCut adds new selector and loupe reflects the change", async () => {
    const f = getFixture();

    const dummySelector = "0xdeadbeef" as `0x${string}`;

    const facetsBefore = (await f.publicClient.readContract({
      abi: diamondLoupeFacetAbi,
      address: f.addresses.diamond,
      functionName: "facetFunctionSelectors",
      args: [f.addresses.diamondCutFacet],
    })) as unknown as `0x${string}`[];
    expect(facetsBefore).not.toContain(dummySelector);

    const cutHash = await f.walletClient.writeContract({
      abi: diamondCutFacetAbi,
      address: f.addresses.diamond,
      functionName: "diamondCut",
      args: [
        [
          {
            facetAddress: f.addresses.diamondCutFacet,
            action: 0, // Add
            functionSelectors: [dummySelector],
          },
        ],
        "0x0000000000000000000000000000000000000000",
        "0x",
      ],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: cutHash });

    const resolvedFacet = await f.publicClient.readContract({
      abi: diamondLoupeFacetAbi,
      address: f.addresses.diamond,
      functionName: "facetAddress",
      args: [dummySelector],
    });
    expect((resolvedFacet as string).toLowerCase()).toBe(
      f.addresses.diamondCutFacet.toLowerCase(),
    );

    const facetsAfter = (await f.publicClient.readContract({
      abi: diamondLoupeFacetAbi,
      address: f.addresses.diamond,
      functionName: "facetFunctionSelectors",
      args: [f.addresses.diamondCutFacet],
    })) as unknown as `0x${string}`[];
    expect(facetsAfter).toContain(dummySelector);
  });

  it("loupe functions through Diamond proxy match direct facet calls", async () => {
    const f = getFixture();

    const proxyFacets = (await f.publicClient.readContract({
      abi: diamondLoupeFacetAbi,
      address: f.addresses.diamond,
      functionName: "facets",
    })) as unknown as { facetAddress: `0x${string}`; functionSelectors: `0x${string}`[] }[];

    const proxyAddresses = (await f.publicClient.readContract({
      abi: diamondLoupeFacetAbi,
      address: f.addresses.diamond,
      functionName: "facetAddresses",
    })) as unknown as `0x${string}`[];

    expect(proxyFacets.length).toBeGreaterThanOrEqual(2);
    expect(proxyAddresses.length).toBeGreaterThanOrEqual(2);
    expect(proxyFacets.length).toBe(proxyAddresses.length);

    const proxyAddressSet = new Set(
      proxyAddresses.map((a) => a.toLowerCase()),
    );
    for (const addr of proxyAddresses) {
      expect(proxyAddressSet.has(addr.toLowerCase())).toBe(true);
    }

    for (const facet of proxyFacets) {
      for (const selector of facet.functionSelectors) {
        const resolved = await f.publicClient.readContract({
          abi: diamondLoupeFacetAbi,
          address: f.addresses.diamond,
          functionName: "facetAddress",
          args: [selector],
        });
        expect((resolved as string).toLowerCase()).toBe(
          facet.facetAddress.toLowerCase(),
        );
      }
    }
  });
});
