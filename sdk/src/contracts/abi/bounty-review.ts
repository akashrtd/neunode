/**
 * BountyReview ABI — 2-of-3 review committee for bounty submissions.
 * Source: contracts/src/bounty/BountyReview.sol
 * Inherited: AccessControl, EIP712
 */

export const bountyReviewAbi = [
	// AccessControl
	{
		type: "function" as const,
		name: "hasRole",
		inputs: [
			{ name: "role", type: "bytes32" },
			{ name: "account", type: "address" },
		],
		outputs: [{ name: "", type: "bool" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "getRoleAdmin",
		inputs: [{ name: "role", type: "bytes32" }],
		outputs: [{ name: "", type: "bytes32" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "grantRole",
		inputs: [
			{ name: "role", type: "bytes32" },
			{ name: "account", type: "address" },
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function" as const,
		name: "revokeRole",
		inputs: [
			{ name: "role", type: "bytes32" },
			{ name: "account", type: "address" },
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function" as const,
		name: "renounceRole",
		inputs: [
			{ name: "role", type: "bytes32" },
			{ name: "callerConfirmation", type: "address" },
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "event" as const,
		name: "RoleAdminChanged",
		inputs: [
			{ name: "role", type: "bytes32", indexed: true },
			{ name: "previousAdminRole", type: "bytes32", indexed: true },
			{ name: "newAdminRole", type: "bytes32", indexed: true },
		],
	},
	{
		type: "event" as const,
		name: "RoleGranted",
		inputs: [
			{ name: "role", type: "bytes32", indexed: true },
			{ name: "account", type: "address", indexed: true },
			{ name: "sender", type: "address", indexed: true },
		],
	},
	{
		type: "event" as const,
		name: "RoleRevoked",
		inputs: [
			{ name: "role", type: "bytes32", indexed: true },
			{ name: "account", type: "address", indexed: true },
			{ name: "sender", type: "address", indexed: true },
		],
	},

	// EIP-712
	{
		type: "function" as const,
		name: "eip712Domain",
		inputs: [],
		outputs: [
			{ name: "fields", type: "bytes1" },
			{ name: "name", type: "string" },
			{ name: "version", type: "string" },
			{ name: "chainId", type: "uint256" },
			{ name: "verifyingContract", type: "address" },
			{ name: "salt", type: "bytes32" },
			{ name: "extensions", type: "uint256[]" },
		],
		stateMutability: "view",
	},

	// Storage getters
	{
		type: "function" as const,
		name: "committees",
		inputs: [{ name: "", type: "bytes32" }],
		outputs: [
			{ name: "reviewers", type: "address[3]" },
			{ name: "acceptCount", type: "uint8" },
			{ name: "rejectCount", type: "uint8" },
			{ name: "resolved", type: "bool" },
			{ name: "assigned", type: "bool" },
		],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "hasReviewed",
		inputs: [
			{ name: "", type: "bytes32" },
			{ name: "", type: "address" },
		],
		outputs: [{ name: "", type: "bool" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "REVIEWER_ROLE",
		inputs: [],
		outputs: [{ name: "", type: "bytes32" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "REVIEW_TYPEHASH",
		inputs: [],
		outputs: [{ name: "", type: "bytes32" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "nonces",
		inputs: [{ name: "", type: "address" }],
		outputs: [{ name: "", type: "uint256" }],
		stateMutability: "view",
	},

	// Functions
	{
		type: "function" as const,
		name: "assignCommittee",
		inputs: [
			{ name: "bountyId", type: "bytes32" },
			{ name: "reviewers", type: "address[3]" },
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function" as const,
		name: "submitReview",
		inputs: [
			{ name: "bountyId", type: "bytes32" },
			{ name: "score", type: "uint8" },
			{ name: "feedback", type: "string" },
			{ name: "signature", type: "bytes" },
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function" as const,
		name: "isAccepted",
		inputs: [{ name: "bountyId", type: "bytes32" }],
		outputs: [{ name: "", type: "bool" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "isResolved",
		inputs: [{ name: "bountyId", type: "bytes32" }],
		outputs: [{ name: "", type: "bool" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "getReviewCount",
		inputs: [{ name: "bountyId", type: "bytes32" }],
		outputs: [{ name: "", type: "uint256" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "getReview",
		inputs: [
			{ name: "bountyId", type: "bytes32" },
			{ name: "index", type: "uint256" },
		],
		outputs: [
			{ name: "reviewer", type: "address" },
			{ name: "score", type: "uint8" },
			{ name: "feedback", type: "string" },
		],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "getCommittee",
		inputs: [{ name: "bountyId", type: "bytes32" }],
		outputs: [
			{ name: "reviewers", type: "address[3]" },
			{ name: "acceptCount", type: "uint8" },
			{ name: "rejectCount", type: "uint8" },
			{ name: "resolved", type: "bool" },
			{ name: "assigned", type: "bool" },
		],
		stateMutability: "view",
	},

	// Events
	{
		type: "event" as const,
		name: "ReviewSubmitted",
		inputs: [
			{ name: "bountyId", type: "bytes32", indexed: true },
			{ name: "reviewer", type: "address", indexed: true },
			{ name: "score", type: "uint8", indexed: false },
			{ name: "accepted", type: "bool", indexed: false },
		],
	},
	{
		type: "event" as const,
		name: "CommitteeAssigned",
		inputs: [
			{ name: "bountyId", type: "bytes32", indexed: true },
			{ name: "reviewers", type: "address", indexed: false },
		],
	},
	{
		type: "event" as const,
		name: "ReviewCompleted",
		inputs: [
			{ name: "bountyId", type: "bytes32", indexed: true },
			{ name: "accepted", type: "bool", indexed: false },
		],
	},
] as const;
