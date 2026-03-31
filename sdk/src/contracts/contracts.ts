import { getContract } from 'viem';
import type { Client } from 'viem';
import { neunodeTokenAbi } from './abi/neunode-token.js';
import { computeTokenAbi } from './abi/compute-token.js';
import { trainingTokenAbi } from './abi/training-token.js';
import { bandwidthTokenAbi } from './abi/bandwidth-token.js';
import { storageTokenAbi } from './abi/storage-token.js';
import { neunodeIdentityAbi } from './abi/neunode-identity.js';
import { neunodeRegistryAbi } from './abi/neunode-registry.js';
import { neunodeBountyAbi } from './abi/neunode-bounty.js';
import { neunodeEscrowAbi } from './abi/neunode-escrow.js';
import { bountyReviewAbi } from './abi/bounty-review.js';
import { modelRegistryAbi } from './abi/model-registry.js';
import { royaltySplitterAbi } from './abi/royalty-splitter.js';
import { neunodeGovernanceAbi } from './abi/neunode-governance.js';
import { diamondAbi, diamondCutFacetAbi, diamondLoupeFacetAbi } from './abi/diamond.js';

type Addr = `0x${string}`;

// TS7056 workaround: large ABIs exceed TS inference serialization limits.
// We cast through unknown to prevent the compiler from expanding the full type.
interface GenericContract {
  read: Record<string, (...args: unknown[]) => Promise<unknown>>;
  write: Record<string, (...args: unknown[]) => Promise<unknown>>;
  estimateGas: Record<string, (...args: unknown[]) => Promise<bigint>>;
  events: Record<string, (...args: unknown[]) => Promise<unknown>>;
  address: Addr;
  abi: readonly unknown[];
}

function makeContract(abi: readonly unknown[], client: Client, address: Addr): GenericContract {
  return getContract({ address, abi: abi as typeof neunodeTokenAbi, client }) as unknown as GenericContract;
}

/** Get a typed contract instance for the nCompute ERC-20 token. */
export function getComputeToken(client: Client, address: Addr) {
  return makeContract(computeTokenAbi, client, address);
}

/** Get a typed contract instance for the nTrain ERC-20 token. */
export function getTrainingToken(client: Client, address: Addr) {
  return makeContract(trainingTokenAbi, client, address);
}

/** Get a typed contract instance for the nBandwidth ERC-20 token. */
export function getBandwidthToken(client: Client, address: Addr) {
  return makeContract(bandwidthTokenAbi, client, address);
}

/** Get a typed contract instance for the nStorage ERC-20 token. */
export function getStorageToken(client: Client, address: Addr) {
  return makeContract(storageTokenAbi, client, address);
}

/** Get a typed contract instance for the Neunode governance token. */
export function getNeunodeToken(client: Client, address: Addr) {
  return makeContract(neunodeTokenAbi, client, address);
}

/** Get a typed contract instance for the Neunode DID identity registry. */
export function getNeunodeIdentity(client: Client, address: Addr) {
  return makeContract(neunodeIdentityAbi, client, address);
}

/** Get a typed contract instance for the Neunode agent registry. */
export function getNeunodeRegistry(client: Client, address: Addr) {
  return makeContract(neunodeRegistryAbi, client, address);
}

/** Get a typed contract instance for the Neunode bounty contract. */
export function getNeunodeBounty(client: Client, address: Addr) {
  return makeContract(neunodeBountyAbi, client, address);
}

/** Get a typed contract instance for the Neunode escrow contract. */
export function getNeunodeEscrow(client: Client, address: Addr) {
  return makeContract(neunodeEscrowAbi, client, address);
}

/** Get a typed contract instance for the bounty peer review contract. */
export function getBountyReview(client: Client, address: Addr) {
  return makeContract(bountyReviewAbi, client, address);
}

/** Get a typed contract instance for the model registry contract. */
export function getModelRegistry(client: Client, address: Addr) {
  return makeContract(modelRegistryAbi, client, address);
}

/** Get a typed contract instance for the royalty splitter contract. */
export function getRoyaltySplitter(client: Client, address: Addr) {
  return makeContract(royaltySplitterAbi, client, address);
}

/** Get a typed contract instance for the Neunode governance contract. */
export function getNeunodeGovernance(client: Client, address: Addr) {
  return makeContract(neunodeGovernanceAbi, client, address);
}

/** Get a typed contract instance for the EIP-2535 Diamond proxy. */
export function getDiamond(client: Client, address: Addr) {
  return makeContract(diamondAbi, client, address);
}

/** Get a typed contract instance for the DiamondCut facet. */
export function getDiamondCutFacet(client: Client, address: Addr) {
  return makeContract(diamondCutFacetAbi, client, address);
}

/** Get a typed contract instance for the DiamondLoupe facet. */
export function getDiamondLoupeFacet(client: Client, address: Addr) {
  return makeContract(diamondLoupeFacetAbi, client, address);
}
