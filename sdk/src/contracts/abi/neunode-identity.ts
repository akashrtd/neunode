// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const neunodeIdentityAbi = [
	{
		type: "constructor",
		inputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "addressToDid",
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
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "createDid",
		inputs: [
			{
				name: "ed25519PubKeyHash",
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
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "deactivateDid",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "deregisterFromNetwork",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "documents",
		inputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "controller",
				type: "address",
				internalType: "address",
			},
			{
				name: "ed25519PublicKeyHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "created",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "updated",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "active",
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getController",
		inputs: [
			{
				name: "didHash",
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
		name: "getDidForAddress",
		inputs: [
			{
				name: "addr",
				type: "address",
				internalType: "address",
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
		name: "getDocument",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "",
				type: "tuple",
				internalType: "struct NeunodeIdentity.DidDocument",
				components: [
					{
						name: "controller",
						type: "address",
						internalType: "address",
					},
					{
						name: "ed25519PublicKeyHash",
						type: "bytes32",
						internalType: "bytes32",
					},
					{
						name: "created",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "updated",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "active",
						type: "bool",
						internalType: "bool",
					},
				],
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "isActive",
		inputs: [
			{
				name: "didHash",
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
		name: "isRegistered",
		inputs: [
			{
				name: "didHash",
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
		name: "minRegistrationStake",
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
		name: "registerForNetwork",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setMinRegistrationStake",
		inputs: [
			{
				name: "newMin",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setStakeSource",
		inputs: [
			{
				name: "source",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "stakeSource",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "contract IStakeSource",
			},
		],
		stateMutability: "view",
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
		name: "updateController",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "newController",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "updateEd25519Key",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "newPubKeyHash",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "verifySignature",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "messageHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "signature",
				type: "bytes",
				internalType: "bytes",
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
		name: "DidCreated",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "controller",
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
		name: "DidDeactivated",
		inputs: [
			{
				name: "didHash",
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
		name: "DidKeyRotated",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "newPubKeyHash",
				type: "bytes32",
				indexed: false,
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
		name: "DidUpdated",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "newController",
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
		name: "MinRegistrationStakeUpdated",
		inputs: [
			{
				name: "oldMin",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "newMin",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "NetworkDeregistered",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "controller",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "NetworkRegistered",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "controller",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "stake",
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
		name: "StakeSourceUpdated",
		inputs: [
			{
				name: "stakeSource",
				type: "address",
				indexed: false,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "error",
		name: "AddressAlreadyHasDid",
		inputs: [
			{
				name: "addr",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "DidAlreadyExists",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
	{
		type: "error",
		name: "DidNotActive",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
	{
		type: "error",
		name: "DidNotFound",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
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
		name: "InsufficientRegistrationStake",
		inputs: [
			{
				name: "controller",
				type: "address",
				internalType: "address",
			},
			{
				name: "staked",
				type: "uint256",
				internalType: "uint256",
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
		name: "InvalidOwner",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidPublicKeyHash",
		inputs: [],
	},
	{
		type: "error",
		name: "NotController",
		inputs: [
			{
				name: "didHash",
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
		name: "NotOwner",
		inputs: [
			{
				name: "caller",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "NotRegistered",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
] as const;
