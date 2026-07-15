import type { Client } from "viem";
import { getContract } from "viem";
import { agentPaymasterAbi } from "./abi/agent-paymaster.js";
import { bandwidthTokenAbi } from "./abi/bandwidth-token.js";
import { bountyReviewAbi } from "./abi/bounty-review.js";
import { computeTokenAbi } from "./abi/compute-token.js";
import {
	diamondAbi,
	diamondCutFacetAbi,
	diamondLoupeFacetAbi,
} from "./abi/diamond.js";
import { modelRegistryAbi } from "./abi/model-registry.js";
import { neunodeBountyAbi } from "./abi/neunode-bounty.js";
import { neunodeEscrowAbi } from "./abi/neunode-escrow.js";
import { neunodeGovernanceAbi } from "./abi/neunode-governance.js";
import { neunodeIdentityAbi } from "./abi/neunode-identity.js";
import { neunodeRegistryAbi } from "./abi/neunode-registry.js";
import { neunodeReputationAbi } from "./abi/neunode-reputation.js";
import { neunodeSlashingAbi } from "./abi/neunode-slashing.js";
import { neunodeTokenAbi } from "./abi/neunode-token.js";
import { resourceAmmAbi } from "./abi/resource-amm.js";
import { royaltySplitterAbi } from "./abi/royalty-splitter.js";
import { stakingEscrowAbi } from "./abi/staking-escrow.js";
import { storageTokenAbi } from "./abi/storage-token.js";
import { trainingTokenAbi } from "./abi/training-token.js";

type Addr = `0x${string}`;

/**
 * Workaround for TS7056: viem's generated contract types from large ABIs
 * exceed TypeScript's type recursion depth limit. When ABIs are small enough
 * to avoid this, individual getters return the fully-typed contract directly.
 * For large ABIs that hit the limit, we use GenericContract as a fallback.
 *
 * Consumers who need full type safety for specific contracts should call
 * `getContract({ address, abi, client })` directly with the imported ABI.
 */
interface GenericContract {
	read: Record<string, (...args: unknown[]) => Promise<unknown>>;
	write: Record<string, (...args: unknown[]) => Promise<unknown>>;
	estimateGas: Record<string, (...args: unknown[]) => Promise<bigint>>;
	events: Record<string, (...args: unknown[]) => Promise<unknown>>;
	address: Addr;
	abi: readonly unknown[];
}

function makeContract(
	abi: readonly unknown[],
	client: Client,
	address: Addr,
): GenericContract {
	return getContract({
		address,
		abi: abi as typeof neunodeTokenAbi,
		client,
	}) as unknown as GenericContract;
}

/** Get a contract instance for the ERC-4337 agent gas sponsor. */
export function getAgentPaymaster(client: Client, address: Addr) {
	return makeContract(agentPaymasterAbi, client, address);
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

/** Get a contract instance for the on-chain reputation registry. */
export function getNeunodeReputation(client: Client, address: Addr) {
	return makeContract(neunodeReputationAbi, client, address);
}

/** Get a contract instance for the slashing coordinator. */
export function getNeunodeSlashing(client: Client, address: Addr) {
	return makeContract(neunodeSlashingAbi, client, address);
}

/** Get a contract instance for the inactivity staking escrow. */
export function getStakingEscrow(client: Client, address: Addr) {
	return makeContract(stakingEscrowAbi, client, address);
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

/** Get a contract instance for the resource-token constant-product AMM. */
export function getResourceAmm(client: Client, address: Addr) {
	return makeContract(resourceAmmAbi, client, address);
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
