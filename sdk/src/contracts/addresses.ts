type Address = `0x${string}`;

export interface ContractAddresses {
	readonly computeToken: Address;
	readonly trainingToken: Address;
	readonly bandwidthToken: Address;
	readonly storageToken: Address;
	readonly neunodeIdentity: Address;
	readonly neunodeRegistry: Address;
	readonly neunodeBounty: Address;
	readonly neunodeEscrow: Address;
	readonly bountyReview: Address;
	readonly modelRegistry: Address;
	readonly royaltySplitter: Address;
	readonly neunodeGovernance: Address;
	readonly diamond: Address;
	readonly diamondCutFacet: Address;
	readonly diamondLoupeFacet: Address;
}

export const chainAddresses: Record<number, ContractAddresses> = {
	31337: {
		computeToken: "0x0000000000000000000000000000000000000001" as Address,
		trainingToken: "0x0000000000000000000000000000000000000002" as Address,
		bandwidthToken: "0x0000000000000000000000000000000000000003" as Address,
		storageToken: "0x0000000000000000000000000000000000000004" as Address,
		neunodeIdentity: "0x0000000000000000000000000000000000000005" as Address,
		neunodeRegistry: "0x0000000000000000000000000000000000000006" as Address,
		neunodeBounty: "0x0000000000000000000000000000000000000007" as Address,
		neunodeEscrow: "0x0000000000000000000000000000000000000008" as Address,
		bountyReview: "0x0000000000000000000000000000000000000009" as Address,
		modelRegistry: "0x0000000000000000000000000000000000000010" as Address,
		royaltySplitter: "0x0000000000000000000000000000000000000011" as Address,
		neunodeGovernance: "0x0000000000000000000000000000000000000012" as Address,
		diamond: "0x0000000000000000000000000000000000000013" as Address,
		diamondCutFacet: "0x0000000000000000000000000000000000000014" as Address,
		diamondLoupeFacet: "0x0000000000000000000000000000000000000015" as Address,
	},
};

export type ChainId = keyof typeof chainAddresses;

/** Get contract addresses for a chain, throwing if the chain is not configured. */
export function getContractAddresses(chainId: number): ContractAddresses {
	const addresses = chainAddresses[chainId];
	if (!addresses) {
		const available = Object.keys(chainAddresses).join(", ");
		throw new Error(
			`No contract addresses configured for chain ${chainId}. Available chains: ${available}`,
		);
	}
	return addresses;
}
