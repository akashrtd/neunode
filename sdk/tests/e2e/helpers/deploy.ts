import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import type { WalletClient, PublicClient, Transport } from "viem";
import { foundry } from "viem/chains";
import { privateKeyToAccount } from "viem/accounts";
import { getFunctionSelector } from "viem";
import { deployContract } from "viem/actions";

import { computeTokenAbi } from "../../../src/contracts/abi/compute-token.js";
import { trainingTokenAbi } from "../../../src/contracts/abi/training-token.js";
import { bandwidthTokenAbi } from "../../../src/contracts/abi/bandwidth-token.js";
import { storageTokenAbi } from "../../../src/contracts/abi/storage-token.js";
import { neunodeIdentityAbi } from "../../../src/contracts/abi/neunode-identity.js";
import { neunodeRegistryAbi } from "../../../src/contracts/abi/neunode-registry.js";
import { neunodeBountyAbi } from "../../../src/contracts/abi/neunode-bounty.js";
import { neunodeEscrowAbi } from "../../../src/contracts/abi/neunode-escrow.js";
import { bountyReviewAbi } from "../../../src/contracts/abi/bounty-review.js";
import { modelRegistryAbi } from "../../../src/contracts/abi/model-registry.js";
import { royaltySplitterAbi } from "../../../src/contracts/abi/royalty-splitter.js";
import { neunodeGovernanceAbi } from "../../../src/contracts/abi/neunode-governance.js";
import {
  diamondCutFacetAbi,
  diamondLoupeFacetAbi,
} from "../../../src/contracts/abi/diamond.js";

type Addr = `0x${string}`;

export interface DeployedContracts {
  computeToken: Addr;
  trainingToken: Addr;
  bandwidthToken: Addr;
  storageToken: Addr;
  neunodeIdentity: Addr;
  neunodeRegistry: Addr;
  neunodeBounty: Addr;
  neunodeEscrow: Addr;
  bountyReview: Addr;
  modelRegistry: Addr;
  royaltySplitter: Addr;
  neunodeGovernance: Addr;
  diamond: Addr;
  diamondCutFacet: Addr;
  diamondLoupeFacet: Addr;
}

const CONTRACTS_OUT = resolve(__dirname, "../../../../contracts/out");

interface Artifact {
  abi: unknown[];
  bytecode: string;
}

function loadArtifact(contractName: string): Artifact {
  const filePath = join(
    CONTRACTS_OUT,
    `${contractName}.sol`,
    `${contractName}.json`,
  );
  const raw = JSON.parse(readFileSync(filePath, "utf-8"));
  return {
    abi: raw.abi as unknown[],
    bytecode: raw.bytecode.object as string,
  };
}

const DIAMOND_CUT_SELECTOR = getFunctionSelector(
  "diamondCut((address,uint8,bytes4[])[],address,bytes)",
);
const FACETS_SELECTOR = getFunctionSelector("facets()");
const FACET_FUNCTION_SELECTORS_SELECTOR = getFunctionSelector(
  "facetFunctionSelectors(address)",
);
const FACET_ADDRESSES_SELECTOR = getFunctionSelector("facetAddresses()");
const FACET_ADDRESS_SELECTOR = getFunctionSelector("facetAddress(bytes4)");

/**
 * Deploy all 15 Neunode contracts in the canonical order from Deploy.s.sol.
 * Returns addresses of all deployed contracts.
 */
export async function deployAll(
  walletClient: WalletClient<Transport, typeof foundry, ReturnType<typeof privateKeyToAccount>>,
  publicClient: PublicClient,
): Promise<DeployedContracts> {
  const deployer = walletClient.account!.address as Addr;

  const waitForTx = (hash: `0x${string}`) =>
    publicClient.waitForTransactionReceipt({ hash });

  // ── Phase 1: Deploy 4 resource-backed tokens ─────────────────────────
  const computeTokenArtifact = loadArtifact("ComputeToken");
  const computeTokenHash = await deployContract(walletClient, {
    abi: computeTokenArtifact.abi,
    bytecode: computeTokenArtifact.bytecode,
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(computeTokenHash);
  const { contractAddress: computeToken } = await publicClient.getTransactionReceipt({
    hash: computeTokenHash,
  });
  const computeTokenAddr = computeToken as Addr;

  const trainingTokenArtifact = loadArtifact("TrainingToken");
  const trainingTokenHash = await deployContract(walletClient, {
    abi: trainingTokenArtifact.abi,
    bytecode: trainingTokenArtifact.bytecode,
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(trainingTokenHash);
  const { contractAddress: trainingToken } = await publicClient.getTransactionReceipt({
    hash: trainingTokenHash,
  });
  const trainingTokenAddr = trainingToken as Addr;

  const bandwidthTokenArtifact = loadArtifact("BandwidthToken");
  const bandwidthTokenHash = await deployContract(walletClient, {
    abi: bandwidthTokenArtifact.abi,
    bytecode: bandwidthTokenArtifact.bytecode,
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(bandwidthTokenHash);
  const { contractAddress: bandwidthToken } = await publicClient.getTransactionReceipt(
    { hash: bandwidthTokenHash },
  );
  const bandwidthTokenAddr = bandwidthToken as Addr;

  const storageTokenArtifact = loadArtifact("StorageToken");
  const storageTokenHash = await deployContract(walletClient, {
    abi: storageTokenArtifact.abi,
    bytecode: storageTokenArtifact.bytecode,
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(storageTokenHash);
  const { contractAddress: storageToken } = await publicClient.getTransactionReceipt({
    hash: storageTokenHash,
  });
  const storageTokenAddr = storageToken as Addr;

  // ── Phase 2: Deploy identity ──────────────────────────────────────────
  const identityArtifact = loadArtifact("NeunodeIdentity");
  const identityHash = await deployContract(walletClient, {
    abi: identityArtifact.abi,
    bytecode: identityArtifact.bytecode,
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(identityHash);
  const { contractAddress: neunodeIdentity } = await publicClient.getTransactionReceipt(
    { hash: identityHash },
  );
  const neunodeIdentityAddr = neunodeIdentity as Addr;

  // ── Phase 3: Deploy registry (needs identity address) ─────────────────
  const registryArtifact = loadArtifact("NeunodeRegistry");
  const registryHash = await deployContract(walletClient, {
    abi: registryArtifact.abi,
    bytecode: registryArtifact.bytecode,
    args: [neunodeIdentityAddr],
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(registryHash);
  const { contractAddress: neunodeRegistry } = await publicClient.getTransactionReceipt(
    { hash: registryHash },
  );
  const neunodeRegistryAddr = neunodeRegistry as Addr;

  // ── Phase 4: Deploy bounty system ─────────────────────────────────────
  const bountyArtifact = loadArtifact("NeunodeBounty");
  const bountyHash = await deployContract(walletClient, {
    abi: bountyArtifact.abi,
    bytecode: bountyArtifact.bytecode,
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(bountyHash);
  const { contractAddress: neunodeBounty } = await publicClient.getTransactionReceipt({
    hash: bountyHash,
  });
  const neunodeBountyAddr = neunodeBounty as Addr;

  const escrowArtifact = loadArtifact("NeunodeEscrow");
  const escrowHash = await deployContract(walletClient, {
    abi: escrowArtifact.abi,
    bytecode: escrowArtifact.bytecode,
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(escrowHash);
  const { contractAddress: neunodeEscrow } = await publicClient.getTransactionReceipt({
    hash: escrowHash,
  });
  const neunodeEscrowAddr = neunodeEscrow as Addr;

  const reviewArtifact = loadArtifact("BountyReview");
  const reviewHash = await deployContract(walletClient, {
    abi: reviewArtifact.abi,
    bytecode: reviewArtifact.bytecode,
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(reviewHash);
  const { contractAddress: bountyReview } = await publicClient.getTransactionReceipt({
    hash: reviewHash,
  });
  const bountyReviewAddr = bountyReview as Addr;

  // Wire bounty ↔ escrow + review
  const setEscrowHash = await walletClient.writeContract({
    abi: neunodeBountyAbi,
    address: neunodeBountyAddr,
    functionName: "setEscrow",
    args: [neunodeEscrowAddr],
    account: deployer,
  });
  await waitForTx(setEscrowHash);

  const setReviewHash = await walletClient.writeContract({
    abi: neunodeBountyAbi,
    address: neunodeBountyAddr,
    functionName: "setReviewContract",
    args: [bountyReviewAddr],
    account: deployer,
  });
  await waitForTx(setReviewHash);

  const registerBountyHash = await walletClient.writeContract({
    abi: neunodeEscrowAbi,
    address: neunodeEscrowAddr,
    functionName: "registerBountyContract",
    args: [neunodeBountyAddr],
    account: deployer,
  });
  await waitForTx(registerBountyHash);

  // ── Phase 5: Deploy royalty system ────────────────────────────────────
  const modelRegistryArtifact = loadArtifact("ModelRegistry");
  const modelRegistryHash = await deployContract(walletClient, {
    abi: modelRegistryArtifact.abi,
    bytecode: modelRegistryArtifact.bytecode,
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(modelRegistryHash);
  const { contractAddress: modelRegistry } = await publicClient.getTransactionReceipt({
    hash: modelRegistryHash,
  });
  const modelRegistryAddr = modelRegistry as Addr;

  const royaltyArtifact = loadArtifact("RoyaltySplitter");
  const royaltyHash = await deployContract(walletClient, {
    abi: royaltyArtifact.abi,
    bytecode: royaltyArtifact.bytecode,
    args: [modelRegistryAddr],
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(royaltyHash);
  const { contractAddress: royaltySplitter } = await publicClient.getTransactionReceipt(
    { hash: royaltyHash },
  );
  const royaltySplitterAddr = royaltySplitter as Addr;

  // ── Phase 6: Deploy governance ────────────────────────────────────────
  const governanceArtifact = loadArtifact("NeunodeGovernance");
  const governanceHash = await deployContract(walletClient, {
    abi: governanceArtifact.abi,
    bytecode: governanceArtifact.bytecode,
    args: [
      computeTokenAddr, // voting token
      86400n,           // votingDelay (1 day)
      604800n,          // votingPeriod (7 days)
      100000000000000000000n, // proposalThreshold (100e18)
      400n,             // quorumBps (4%)
      172800n,          // timelock (2 days)
      1209600n,         // executionWindow (14 days)
    ],
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(governanceHash);
  const { contractAddress: neunodeGovernance } =
    await publicClient.getTransactionReceipt({ hash: governanceHash });
  const neunodeGovernanceAddr = neunodeGovernance as Addr;

  // ── Phase 7: Deploy diamond proxy ─────────────────────────────────────
  const cutFacetArtifact = loadArtifact("DiamondCutFacet");
  const cutFacetHash = await deployContract(walletClient, {
    abi: cutFacetArtifact.abi,
    bytecode: cutFacetArtifact.bytecode,
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(cutFacetHash);
  const { contractAddress: diamondCutFacet } =
    await publicClient.getTransactionReceipt({ hash: cutFacetHash });
  const diamondCutFacetAddr = diamondCutFacet as Addr;

  const loupeFacetArtifact = loadArtifact("DiamondLoupeFacet");
  const loupeFacetHash = await deployContract(walletClient, {
    abi: loupeFacetArtifact.abi,
    bytecode: loupeFacetArtifact.bytecode,
    account: deployer,
    gas: 5_000_000n,
  });
  await waitForTx(loupeFacetHash);
  const { contractAddress: diamondLoupeFacet } =
    await publicClient.getTransactionReceipt({ hash: loupeFacetHash });
  const diamondLoupeFacetAddr = diamondLoupeFacet as Addr;

  // Build facet cuts for Diamond constructor
  const cuts = [
    {
      facetAddress: diamondCutFacetAddr,
      action: 0 as const,
      functionSelectors: [DIAMOND_CUT_SELECTOR],
    },
    {
      facetAddress: diamondLoupeFacetAddr,
      action: 0 as const,
      functionSelectors: [
        FACETS_SELECTOR,
        FACET_FUNCTION_SELECTORS_SELECTOR,
        FACET_ADDRESSES_SELECTOR,
        FACET_ADDRESS_SELECTOR,
      ],
    },
  ];

  const diamondArtifact = loadArtifact("Diamond");
  const diamondHash = await deployContract(walletClient, {
    abi: diamondArtifact.abi,
    bytecode: diamondArtifact.bytecode,
    args: [cuts, "0x0000000000000000000000000000000000000000" as Addr, "0x", deployer],
    account: deployer,
    gas: 10_000_000n,
  });
  await waitForTx(diamondHash);
  const { contractAddress: diamond } = await publicClient.getTransactionReceipt({
    hash: diamondHash,
  });
  const diamondAddr = diamond as Addr;

  return {
    computeToken: computeTokenAddr,
    trainingToken: trainingTokenAddr,
    bandwidthToken: bandwidthTokenAddr,
    storageToken: storageTokenAddr,
    neunodeIdentity: neunodeIdentityAddr,
    neunodeRegistry: neunodeRegistryAddr,
    neunodeBounty: neunodeBountyAddr,
    neunodeEscrow: neunodeEscrowAddr,
    bountyReview: bountyReviewAddr,
    modelRegistry: modelRegistryAddr,
    royaltySplitter: royaltySplitterAddr,
    neunodeGovernance: neunodeGovernanceAddr,
    diamond: diamondAddr,
    diamondCutFacet: diamondCutFacetAddr,
    diamondLoupeFacet: diamondLoupeFacetAddr,
  };
}
