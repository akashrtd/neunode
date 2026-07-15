// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const bountyReviewAbi = [
	{
		type: "constructor",
		inputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "DEFAULT_ADMIN_ROLE",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "REVIEWER_ROLE",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "REVIEW_TYPEHASH",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "assignCommittee",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "reviewers",
				type: "address[3]",
				internalType: "address[3]",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "committees",
		inputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "acceptCount",
				type: "uint8",
				internalType: "uint8",
			},
			{
				name: "rejectCount",
				type: "uint8",
				internalType: "uint8",
			},
			{
				name: "resolved",
				type: "bool",
				internalType: "bool",
			},
			{
				name: "assigned",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "eip712Domain",
		inputs: [],
		outputs: [
			{
				name: "fields",
				type: "bytes1",
				internalType: "bytes1",
			},
			{
				name: "name",
				type: "string",
				internalType: "string",
			},
			{
				name: "version",
				type: "string",
				internalType: "string",
			},
			{
				name: "chainId",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "verifyingContract",
				type: "address",
				internalType: "address",
			},
			{
				name: "salt",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "extensions",
				type: "uint256[]",
				internalType: "uint256[]",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getCommittee",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "reviewers",
				type: "address[3]",
				internalType: "address[3]",
			},
			{
				name: "acceptCount",
				type: "uint8",
				internalType: "uint8",
			},
			{
				name: "rejectCount",
				type: "uint8",
				internalType: "uint8",
			},
			{
				name: "resolved",
				type: "bool",
				internalType: "bool",
			},
			{
				name: "assigned",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getReview",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "index",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "reviewer",
				type: "address",
				internalType: "address",
			},
			{
				name: "score",
				type: "uint8",
				internalType: "uint8",
			},
			{
				name: "feedback",
				type: "string",
				internalType: "string",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getReviewCount",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getRoleAdmin",
		inputs: [
			{
				name: "role",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "grantRole",
		inputs: [
			{
				name: "role",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "hasReviewed",
		inputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "hasRole",
		inputs: [
			{
				name: "role",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "isAccepted",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "isResolved",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "nonces",
		inputs: [
			{
				name: "",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "renounceRole",
		inputs: [
			{
				name: "role",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "callerConfirmation",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "reviews",
		inputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "reviewer",
				type: "address",
				internalType: "address",
			},
			{
				name: "score",
				type: "uint8",
				internalType: "uint8",
			},
			{
				name: "feedback",
				type: "string",
				internalType: "string",
			},
			{
				name: "signature",
				type: "bytes",
				internalType: "bytes",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "revokeRole",
		inputs: [
			{
				name: "role",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "submitReview",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "score",
				type: "uint8",
				internalType: "uint8",
			},
			{
				name: "feedback",
				type: "string",
				internalType: "string",
			},
			{
				name: "signature",
				type: "bytes",
				internalType: "bytes",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "supportsInterface",
		inputs: [
			{
				name: "interfaceId",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "event",
		name: "CommitteeAssigned",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "reviewers",
				type: "address[3]",
				indexed: false,
				internalType: "address[3]",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "EIP712DomainChanged",
		inputs: [],
		anonymous: false,
	},
	{
		type: "event",
		name: "ReviewCompleted",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "accepted",
				type: "bool",
				indexed: false,
				internalType: "bool",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ReviewSubmitted",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "reviewer",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "score",
				type: "uint8",
				indexed: false,
				internalType: "uint8",
			},
			{
				name: "accepted",
				type: "bool",
				indexed: false,
				internalType: "bool",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "RoleAdminChanged",
		inputs: [
			{
				name: "role",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "previousAdminRole",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "newAdminRole",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "RoleGranted",
		inputs: [
			{
				name: "role",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "account",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "sender",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "RoleRevoked",
		inputs: [
			{
				name: "role",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "account",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "sender",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "error",
		name: "AccessControlBadConfirmation",
		inputs: [],
	},
	{
		type: "error",
		name: "AccessControlUnauthorizedAccount",
		inputs: [
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
			{
				name: "neededRole",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
	{
		type: "error",
		name: "AlreadyReviewed",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "reviewer",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "CommitteeAlreadyAssigned",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
	{
		type: "error",
		name: "CommitteeAlreadyResolved",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
	{
		type: "error",
		name: "CommitteeNotAssigned",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
	{
		type: "error",
		name: "DuplicateReviewer",
		inputs: [],
	},
	{
		type: "error",
		name: "ECDSAInvalidSignature",
		inputs: [],
	},
	{
		type: "error",
		name: "ECDSAInvalidSignatureLength",
		inputs: [
			{
				name: "length",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ECDSAInvalidSignatureS",
		inputs: [
			{
				name: "s",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
	{
		type: "error",
		name: "IndexOutOfBounds",
		inputs: [
			{
				name: "index",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "length",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "InvalidShortString",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidSignature",
		inputs: [
			{
				name: "expected",
				type: "address",
				internalType: "address",
			},
			{
				name: "actual",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "NotReviewer",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "caller",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "StringTooLong",
		inputs: [
			{
				name: "str",
				type: "string",
				internalType: "string",
			},
		],
	},
	{
		type: "error",
		name: "ZeroAddressReviewer",
		inputs: [],
	},
] as const;
