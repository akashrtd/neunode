// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const agentPaymasterAbi = [
	{
		type: "constructor",
		inputs: [
			{
				name: "entryPoint_",
				type: "address",
				internalType: "address",
			},
			{
				name: "sponsorSigner_",
				type: "address",
				internalType: "address",
			},
			{
				name: "admin",
				type: "address",
				internalType: "address",
			},
		],
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
		name: "PAYMASTER_DATA_OFFSET",
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
		name: "PAYMASTER_SIG_MAGIC",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "bytes8",
				internalType: "bytes8",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "SPONSORSHIP_TYPEHASH",
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
		name: "SPONSOR_ADMIN_ROLE",
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
		name: "addStake",
		inputs: [
			{
				name: "unstakeDelaySec",
				type: "uint32",
				internalType: "uint32",
			},
		],
		outputs: [],
		stateMutability: "payable",
	},
	{
		type: "function",
		name: "deposit",
		inputs: [],
		outputs: [],
		stateMutability: "payable",
	},
	{
		type: "function",
		name: "depositBalance",
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
		name: "entryPoint",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "contract IEntryPoint",
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
		name: "getSponsorshipHash",
		inputs: [
			{
				name: "userOpHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "sponsorLimit",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "validUntil",
				type: "uint48",
				internalType: "uint48",
			},
			{
				name: "validAfter",
				type: "uint48",
				internalType: "uint48",
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
		name: "pause",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "paused",
		inputs: [],
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
		name: "postOp",
		inputs: [
			{
				name: "",
				type: "uint8",
				internalType: "uint8",
			},
			{
				name: "",
				type: "bytes",
				internalType: "bytes",
			},
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
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
		name: "setSponsorSigner",
		inputs: [
			{
				name: "newSigner",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "sponsorSigner",
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
		type: "function",
		name: "unlockStake",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "unpause",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "validatePaymasterUserOp",
		inputs: [
			{
				name: "userOp",
				type: "tuple",
				internalType: "struct PackedUserOperation",
				components: [
					{
						name: "sender",
						type: "address",
						internalType: "address",
					},
					{
						name: "nonce",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "initCode",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "callData",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "accountGasLimits",
						type: "bytes32",
						internalType: "bytes32",
					},
					{
						name: "preVerificationGas",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "gasFees",
						type: "bytes32",
						internalType: "bytes32",
					},
					{
						name: "paymasterAndData",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "signature",
						type: "bytes",
						internalType: "bytes",
					},
				],
			},
			{
				name: "userOpHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "maxCost",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "context",
				type: "bytes",
				internalType: "bytes",
			},
			{
				name: "validationData",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "withdrawDeposit",
		inputs: [
			{
				name: "recipient",
				type: "address",
				internalType: "address payable",
			},
			{
				name: "amount",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "withdrawStake",
		inputs: [
			{
				name: "recipient",
				type: "address",
				internalType: "address payable",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "event",
		name: "EIP712DomainChanged",
		inputs: [],
		anonymous: false,
	},
	{
		type: "event",
		name: "EntryPointDepositWithdrawn",
		inputs: [
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
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Paused",
		inputs: [
			{
				name: "account",
				type: "address",
				indexed: false,
				internalType: "address",
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
		name: "SponsorSignerUpdated",
		inputs: [
			{
				name: "previousSigner",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "newSigner",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Unpaused",
		inputs: [
			{
				name: "account",
				type: "address",
				indexed: false,
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
		name: "EnforcedPause",
		inputs: [],
	},
	{
		type: "error",
		name: "ExpectedPause",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidPaymasterData",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidShortString",
		inputs: [],
	},
	{
		type: "error",
		name: "OnlyEntryPoint",
		inputs: [],
	},
	{
		type: "error",
		name: "SponsorLimitExceeded",
		inputs: [
			{
				name: "maxCost",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "sponsorLimit",
				type: "uint256",
				internalType: "uint256",
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
		name: "ZeroAddress",
		inputs: [],
	},
] as const;
