// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const resourceAmmAbi = [
	{
		type: "constructor",
		inputs: [
			{
				name: "tokens",
				type: "address[4]",
				internalType: "address[4]",
			},
			{
				name: "treasury",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "BPS_DENOMINATOR",
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
		name: "SWAP_FEE_BPS",
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
		name: "TREASURY_ROLE",
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
		name: "addLiquidity",
		inputs: [
			{
				name: "tokenA",
				type: "address",
				internalType: "address",
			},
			{
				name: "tokenB",
				type: "address",
				internalType: "address",
			},
			{
				name: "amountA",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "amountB",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "allowedTokens",
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
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getReserves",
		inputs: [
			{
				name: "tokenA",
				type: "address",
				internalType: "address",
			},
			{
				name: "tokenB",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [
			{
				name: "reserveA",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "reserveB",
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
		name: "quoteExactInput",
		inputs: [
			{
				name: "tokenIn",
				type: "address",
				internalType: "address",
			},
			{
				name: "tokenOut",
				type: "address",
				internalType: "address",
			},
			{
				name: "amountIn",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "amountOut",
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
		name: "seedPool",
		inputs: [
			{
				name: "tokenA",
				type: "address",
				internalType: "address",
			},
			{
				name: "tokenB",
				type: "address",
				internalType: "address",
			},
			{
				name: "amountA",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "amountB",
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
		type: "function",
		name: "swapExactInput",
		inputs: [
			{
				name: "tokenIn",
				type: "address",
				internalType: "address",
			},
			{
				name: "tokenOut",
				type: "address",
				internalType: "address",
			},
			{
				name: "amountIn",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "minimumOut",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "recipient",
				type: "address",
				internalType: "address",
			},
			{
				name: "deadline",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "amountOut",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "event",
		name: "LiquidityAdded",
		inputs: [
			{
				name: "token0",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "token1",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "amount0",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "amount1",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "PoolSeeded",
		inputs: [
			{
				name: "token0",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "token1",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "amount0",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "amount1",
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
		name: "Swap",
		inputs: [
			{
				name: "sender",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "tokenIn",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "tokenOut",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "amountIn",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "amountOut",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "recipient",
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
		name: "DeadlineExpired",
		inputs: [
			{
				name: "deadline",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "timestamp",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "IdenticalTokens",
		inputs: [],
	},
	{
		type: "error",
		name: "InsufficientLiquidity",
		inputs: [],
	},
	{
		type: "error",
		name: "InsufficientOutput",
		inputs: [
			{
				name: "minimum",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "actual",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "InvalidRecipient",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidToken",
		inputs: [],
	},
	{
		type: "error",
		name: "PoolAlreadyInitialized",
		inputs: [],
	},
	{
		type: "error",
		name: "PoolNotInitialized",
		inputs: [],
	},
	{
		type: "error",
		name: "ReentrancyGuardReentrantCall",
		inputs: [],
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
		name: "ZeroAmount",
		inputs: [],
	},
] as const;
