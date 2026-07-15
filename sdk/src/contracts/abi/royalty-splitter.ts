// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const royaltySplitterAbi = [
	{
		type: "constructor",
		inputs: [
			{
				name: "registry_",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "ADMIN_ROLE",
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
		name: "DECAY_DENOMINATOR",
		inputs: [],
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
		name: "DECAY_NUMERATOR",
		inputs: [],
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
		name: "DEFAULT_SHAPLEY_SCORE",
		inputs: [],
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
		name: "accumulatedRoyalties",
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
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "contributionTypeWeights",
		inputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
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
		name: "defaultReceiver",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "distributeRoyalties",
		inputs: [
			{
				name: "modelCid",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "amount",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "token",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "getContributionTypeWeight",
		inputs: [
			{
				name: "contributionType",
				type: "uint8",
				internalType: "uint8",
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "pure",
	},
	{
		type: "function",
		name: "getRecipients",
		inputs: [
			{
				name: "modelCid",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "",
				type: "tuple[]",
				internalType: "struct IRoyaltySplitter.RecipientInfo[]",
				components: [
					{
						name: "contributor",
						type: "address",
						internalType: "address",
					},
					{
						name: "weight",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "depth",
						type: "uint256",
						internalType: "uint256",
					},
				],
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
		name: "maxLineageDepth",
		inputs: [],
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
		name: "protocolRoyaltyBps",
		inputs: [],
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
		name: "registry",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "contract IModelRegistry",
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
		name: "royaltyInfo",
		inputs: [
			{
				name: "tokenId",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "salePrice",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "receiver",
				type: "address",
				internalType: "address",
			},
			{
				name: "royaltyAmount",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "setDefaultReceiver",
		inputs: [
			{
				name: "receiver",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setMaxLineageDepth",
		inputs: [
			{
				name: "newMax",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setProtocolRoyaltyBps",
		inputs: [
			{
				name: "newBps",
				type: "uint256",
				internalType: "uint256",
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
		name: "ProtocolRoyaltyBpsUpdated",
		inputs: [
			{
				name: "oldBps",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "newBps",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "RecipientPaid",
		inputs: [
			{
				name: "modelCid",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "recipient",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "amount",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "depth",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
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
		type: "event",
		name: "RoyaltyDistributed",
		inputs: [
			{
				name: "modelCid",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "token",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "totalAmount",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "recipientCount",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
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
		name: "BpsExceedsMax",
		inputs: [
			{
				name: "bps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "max",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "DistributionFailed",
		inputs: [
			{
				name: "recipient",
				type: "address",
				internalType: "address",
			},
			{
				name: "amount",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "InvalidContributionType",
		inputs: [
			{
				name: "contributionType",
				type: "uint8",
				internalType: "uint8",
			},
		],
	},
	{
		type: "error",
		name: "InvalidMaxLineageDepth",
		inputs: [
			{
				name: "provided",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "LineageTooDeep",
		inputs: [
			{
				name: "nodes",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "max",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ModelNotFound",
		inputs: [
			{
				name: "cid",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
	{
		type: "error",
		name: "NoLineage",
		inputs: [
			{
				name: "cid",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
	{
		type: "error",
		name: "SafeERC20FailedOperation",
		inputs: [
			{
				name: "token",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "ZeroAddress",
		inputs: [],
	},
	{
		type: "error",
		name: "ZeroAmount",
		inputs: [],
	},
] as const;
