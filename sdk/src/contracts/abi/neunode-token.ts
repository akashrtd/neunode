// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const neunodeTokenAbi = [
	{
		type: "function",
		name: "BURNER_ROLE",
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
		name: "GOVERNANCE_ROLE",
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
		name: "MINTER_ROLE",
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
		name: "activateSeed",
		inputs: [
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
		name: "allowance",
		inputs: [
			{
				name: "owner",
				type: "address",
				internalType: "address",
			},
			{
				name: "spender",
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
		name: "approve",
		inputs: [
			{
				name: "spender",
				type: "address",
				internalType: "address",
			},
			{
				name: "value",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "balanceOf",
		inputs: [
			{
				name: "account",
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
		name: "burn",
		inputs: [
			{
				name: "from",
				type: "address",
				internalType: "address",
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
		name: "decimals",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "uint8",
				internalType: "uint8",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getActivityLevel",
		inputs: [
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [
			{
				name: "",
				type: "uint8",
				internalType: "uint8",
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
		name: "lastActivity",
		inputs: [
			{
				name: "account",
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
		name: "maxSupplyCap",
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
		name: "mint",
		inputs: [
			{
				name: "to",
				type: "address",
				internalType: "address",
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
		name: "mintSeed",
		inputs: [
			{
				name: "to",
				type: "address",
				internalType: "address",
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
		name: "name",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "string",
				internalType: "string",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "owner",
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
		name: "renounceOwnership",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
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
		name: "seedBalanceOf",
		inputs: [
			{
				name: "account",
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
		name: "setSupplyCap",
		inputs: [
			{
				name: "newCap",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "slashStake",
		inputs: [
			{
				name: "account",
				type: "address",
				internalType: "address",
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
		name: "stake",
		inputs: [
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
		name: "stakedBalanceOf",
		inputs: [
			{
				name: "account",
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
		name: "supplyCap",
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
		name: "symbol",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "string",
				internalType: "string",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "totalSupply",
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
		name: "transfer",
		inputs: [
			{
				name: "to",
				type: "address",
				internalType: "address",
			},
			{
				name: "value",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "transferFrom",
		inputs: [
			{
				name: "from",
				type: "address",
				internalType: "address",
			},
			{
				name: "to",
				type: "address",
				internalType: "address",
			},
			{
				name: "value",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "transferOwnership",
		inputs: [
			{
				name: "newOwner",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "unstake",
		inputs: [
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
		name: "updateActivity",
		inputs: [
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
		type: "event",
		name: "ActivityUpdated",
		inputs: [
			{
				name: "account",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "timestamp",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Approval",
		inputs: [
			{
				name: "owner",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "spender",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "value",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "OwnershipTransferred",
		inputs: [
			{
				name: "previousOwner",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "newOwner",
				type: "address",
				indexed: true,
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
		name: "SeedActivated",
		inputs: [
			{
				name: "account",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "SeedMinted",
		inputs: [
			{
				name: "to",
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
		name: "StakeSlashed",
		inputs: [
			{
				name: "account",
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
		name: "Staked",
		inputs: [
			{
				name: "account",
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
		name: "SupplyCapUpdated",
		inputs: [
			{
				name: "previousCap",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "newCap",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Transfer",
		inputs: [
			{
				name: "from",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "to",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "value",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "Unstaked",
		inputs: [
			{
				name: "account",
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
		name: "CannotUnstakeSeed",
		inputs: [],
	},
	{
		type: "error",
		name: "ERC20InsufficientAllowance",
		inputs: [
			{
				name: "spender",
				type: "address",
				internalType: "address",
			},
			{
				name: "allowance",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "needed",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ERC20InsufficientBalance",
		inputs: [
			{
				name: "sender",
				type: "address",
				internalType: "address",
			},
			{
				name: "balance",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "needed",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ERC20InvalidApprover",
		inputs: [
			{
				name: "approver",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "ERC20InvalidReceiver",
		inputs: [
			{
				name: "receiver",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "ERC20InvalidSender",
		inputs: [
			{
				name: "sender",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "ERC20InvalidSpender",
		inputs: [
			{
				name: "spender",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "InsufficientBalance",
		inputs: [
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
			{
				name: "required",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "InsufficientStake",
		inputs: [
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
			{
				name: "required",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "OwnableInvalidOwner",
		inputs: [
			{
				name: "owner",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "OwnableUnauthorizedAccount",
		inputs: [
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "SupplyCapAboveMaximum",
		inputs: [
			{
				name: "requestedCap",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "maximumCap",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "SupplyCapBelowCurrentSupply",
		inputs: [
			{
				name: "requestedCap",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "currentSupply",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "SupplyCapExceeded",
		inputs: [
			{
				name: "requestedSupply",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "cap",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "UnauthorizedActivityUpdate",
		inputs: [
			{
				name: "caller",
				type: "address",
				internalType: "address",
			},
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
		],
	},
] as const;
