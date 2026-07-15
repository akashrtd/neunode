import {
	type Address,
	concat,
	encodeAbiParameters,
	type Hex,
	numberToHex,
	size,
} from "viem";

export const agentPaymasterSignatureMagic = "0x22e325a297439656" as const;

export interface AgentPaymasterData {
	readonly paymaster: Address;
	readonly verificationGasLimit: bigint;
	readonly postOpGasLimit: bigint;
	readonly validUntil: bigint;
	readonly validAfter: bigint;
	readonly sponsorLimit: bigint;
	readonly signature: Hex;
}

const MAX_UINT128 = (1n << 128n) - 1n;
const MAX_UINT48 = (1n << 48n) - 1n;
const MAX_SIGNATURE_BYTES = 65_535;

/** Encode ERC-4337 v0.8 paymasterAndData, including its signature-exclusion suffix. */
export function encodeAgentPaymasterData(data: AgentPaymasterData): Hex {
	assertUint(data.verificationGasLimit, MAX_UINT128, "verificationGasLimit");
	assertUint(data.postOpGasLimit, MAX_UINT128, "postOpGasLimit");
	assertUint(data.validUntil, MAX_UINT48, "validUntil");
	assertUint(data.validAfter, MAX_UINT48, "validAfter");
	assertUint(data.sponsorLimit, (1n << 256n) - 1n, "sponsorLimit");
	const signatureLength = size(data.signature);
	if (signatureLength > MAX_SIGNATURE_BYTES) {
		throw new RangeError("signature exceeds ERC-4337 uint16 length");
	}

	const policy = encodeAbiParameters(
		[{ type: "uint48" }, { type: "uint48" }, { type: "uint256" }],
		[Number(data.validUntil), Number(data.validAfter), data.sponsorLimit],
	);
	return concat([
		data.paymaster,
		numberToHex(data.verificationGasLimit, { size: 16 }),
		numberToHex(data.postOpGasLimit, { size: 16 }),
		policy,
		data.signature,
		numberToHex(signatureLength, { size: 2 }),
		agentPaymasterSignatureMagic,
	]);
}

export interface AgentSponsorshipTypedData {
	readonly chainId: number;
	readonly paymaster: Address;
	readonly userOpHash: Hex;
	readonly sponsorLimit: bigint;
	readonly validUntil: bigint;
	readonly validAfter: bigint;
}

/** Build the exact EIP-712 payload accepted by AgentPaymaster. */
export function getAgentSponsorshipTypedData(data: AgentSponsorshipTypedData) {
	assertUint(data.validUntil, MAX_UINT48, "validUntil");
	assertUint(data.validAfter, MAX_UINT48, "validAfter");
	assertUint(data.sponsorLimit, (1n << 256n) - 1n, "sponsorLimit");
	return {
		domain: {
			name: "Neunode Agent Paymaster",
			version: "1",
			chainId: data.chainId,
			verifyingContract: data.paymaster,
		},
		types: {
			Sponsorship: [
				{ name: "userOpHash", type: "bytes32" },
				{ name: "sponsorLimit", type: "uint256" },
				{ name: "validUntil", type: "uint48" },
				{ name: "validAfter", type: "uint48" },
			],
		},
		primaryType: "Sponsorship",
		message: {
			userOpHash: data.userOpHash,
			sponsorLimit: data.sponsorLimit,
			validUntil: data.validUntil,
			validAfter: data.validAfter,
		},
	} as const;
}

function assertUint(value: bigint, maximum: bigint, field: string): void {
	if (value < 0n || value > maximum) {
		throw new RangeError(`${field} is outside its unsigned integer range`);
	}
}
