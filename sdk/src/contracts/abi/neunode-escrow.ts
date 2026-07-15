// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const neunodeEscrowAbi = [
	{
		type: "constructor",
		inputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "BOUNTY_CONTRACT_ROLE",
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
		name: "ESCROW_ADMIN_ROLE",
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
		name: "PROVIDER_BOND_BPS",
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
		name: "autoRefund",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "timeoutSeconds",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "bondProvider",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "provider_",
				type: "address",
				internalType: "address",
			},
			{
				name: "bondAmount",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "createBountyEscrow",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "requester_",
				type: "address",
				internalType: "address",
			},
			{
				name: "token",
				type: "address",
				internalType: "address",
			},
			{
				name: "amount",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "workDeadline",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "createEscrow",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "token",
				type: "address",
				internalType: "address",
			},
			{
				name: "amount",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "deadline",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "dispute",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "escrowBountyContracts",
		inputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
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
		name: "escrows",
		inputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "requester",
				type: "address",
				internalType: "address",
			},
			{
				name: "provider",
				type: "address",
				internalType: "address",
			},
			{
				name: "token",
				type: "address",
				internalType: "address",
			},
			{
				name: "amount",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "providerBond",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "created",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "deadline",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "state",
				type: "uint8",
				internalType: "enum NeunodeEscrow.EscrowState",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "fundEscrow",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "providerBond",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "getEscrowState",
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
				type: "uint8",
				internalType: "enum NeunodeEscrow.EscrowState",
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
		name: "isEscrowFunded",
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
		name: "refund",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "refundRequester",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "registerBountyContract",
		inputs: [
			{
				name: "bountyContract",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "release",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "releaseWithFees",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "provider_",
				type: "address",
				internalType: "address",
			},
			{
				name: "protocolFeeBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "reviewerFeeBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "verificationFeeBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "protocolFeeRecipient",
				type: "address",
				internalType: "address",
			},
			{
				name: "reviewerFeeRecipient",
				type: "address",
				internalType: "address",
			},
			{
				name: "verificationFeeRecipient",
				type: "address",
				internalType: "address",
			},
		],
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
		name: "BountyContractRegistered",
		inputs: [
			{
				name: "bountyContract",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "EscrowCreated",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "requester",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "token",
				type: "address",
				indexed: false,
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
		name: "EscrowDisputed",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
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
		name: "EscrowFunded",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "provider",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "bond",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "EscrowRefunded",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "requester",
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
		name: "EscrowReleased",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "provider",
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
		name: "EscrowReleasedWithFees",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "provider",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "providerPayout",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "protocolFee",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "reviewerFee",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "verificationFee",
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
		name: "DeadlinePassed",
		inputs: [
			{
				name: "deadline",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "EscrowAlreadyExists",
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
		name: "EscrowNotCreated",
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
		name: "EscrowNotFound",
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
		name: "EscrowNotFunded",
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
		name: "FeeBpsExceeds100Pct",
		inputs: [
			{
				name: "totalBps",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "InvalidAmount",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidToken",
		inputs: [],
	},
	{
		type: "error",
		name: "NotProvider",
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
		name: "NotRequester",
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
		name: "Unauthorized",
		inputs: [],
	},
	{
		type: "error",
		name: "ZeroAddressFeeRecipient",
		inputs: [],
	},
] as const;
