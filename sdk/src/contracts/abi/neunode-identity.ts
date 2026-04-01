/**
 * NeunodeIdentity ABI — DID Registry for AI agents.
 * Source: contracts/src/NeunodeIdentity.sol
 */

export const neunodeIdentityAbi = [
	{
		type: "function" as const,
		name: "documents",
		inputs: [{ name: "", type: "bytes32" }],
		outputs: [
			{ name: "controller", type: "address" },
			{ name: "ed25519PublicKeyHash", type: "bytes32" },
			{ name: "created", type: "uint256" },
			{ name: "updated", type: "uint256" },
			{ name: "active", type: "bool" },
		],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "addressToDid",
		inputs: [{ name: "", type: "address" }],
		outputs: [{ name: "", type: "bytes32" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "createDid",
		inputs: [{ name: "ed25519PubKeyHash", type: "bytes32" }],
		outputs: [{ name: "", type: "bytes32" }],
		stateMutability: "nonpayable",
	},
	{
		type: "function" as const,
		name: "updateController",
		inputs: [
			{ name: "didHash", type: "bytes32" },
			{ name: "newController", type: "address" },
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function" as const,
		name: "deactivateDid",
		inputs: [{ name: "didHash", type: "bytes32" }],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function" as const,
		name: "getController",
		inputs: [{ name: "didHash", type: "bytes32" }],
		outputs: [{ name: "", type: "address" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "isActive",
		inputs: [{ name: "didHash", type: "bytes32" }],
		outputs: [{ name: "", type: "bool" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "verifySignature",
		inputs: [
			{ name: "didHash", type: "bytes32" },
			{ name: "messageHash", type: "bytes32" },
			{ name: "signature", type: "bytes" },
		],
		outputs: [{ name: "", type: "bool" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "getDidForAddress",
		inputs: [{ name: "addr", type: "address" }],
		outputs: [{ name: "", type: "bytes32" }],
		stateMutability: "view",
	},
	{
		type: "function" as const,
		name: "getDocument",
		inputs: [{ name: "didHash", type: "bytes32" }],
		outputs: [
			{
				name: "",
				type: "tuple",
				components: [
					{ name: "controller", type: "address" },
					{ name: "ed25519PublicKeyHash", type: "bytes32" },
					{ name: "created", type: "uint256" },
					{ name: "updated", type: "uint256" },
					{ name: "active", type: "bool" },
				],
			},
		],
		stateMutability: "view",
	},

	// Events
	{
		type: "event" as const,
		name: "DidCreated",
		inputs: [
			{ name: "didHash", type: "bytes32", indexed: true },
			{ name: "controller", type: "address", indexed: true },
			{ name: "timestamp", type: "uint256", indexed: false },
		],
	},
	{
		type: "event" as const,
		name: "DidUpdated",
		inputs: [
			{ name: "didHash", type: "bytes32", indexed: true },
			{ name: "newController", type: "address", indexed: true },
			{ name: "timestamp", type: "uint256", indexed: false },
		],
	},
	{
		type: "event" as const,
		name: "DidDeactivated",
		inputs: [
			{ name: "didHash", type: "bytes32", indexed: true },
			{ name: "timestamp", type: "uint256", indexed: false },
		],
	},

	// Errors
	{
		type: "error" as const,
		name: "DidAlreadyExists",
		inputs: [{ name: "didHash", type: "bytes32" }],
	},
	{
		type: "error" as const,
		name: "DidNotFound",
		inputs: [{ name: "didHash", type: "bytes32" }],
	},
	{
		type: "error" as const,
		name: "DidNotActive",
		inputs: [{ name: "didHash", type: "bytes32" }],
	},
	{
		type: "error" as const,
		name: "NotController",
		inputs: [
			{ name: "didHash", type: "bytes32" },
			{ name: "caller", type: "address" },
		],
	},
	{
		type: "error" as const,
		name: "AddressAlreadyHasDid",
		inputs: [{ name: "addr", type: "address" }],
	},
	{
		type: "error" as const,
		name: "InvalidPublicKeyHash",
		inputs: [],
	},
] as const;
