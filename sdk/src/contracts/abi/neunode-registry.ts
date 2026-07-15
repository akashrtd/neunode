// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const neunodeRegistryAbi = [
	{
		type: "constructor",
		inputs: [
			{
				name: "identity_",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "activeCount",
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
		name: "agentList",
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
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "agents",
		inputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "capabilities",
				type: "string",
				internalType: "string",
			},
			{
				name: "endpoint",
				type: "string",
				internalType: "string",
			},
			{
				name: "stakeAmount",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "registeredAt",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "updatedAt",
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
		name: "deregister",
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
		name: "getActiveAgents",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "bytes32[]",
				internalType: "bytes32[]",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getAgent",
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
				internalType: "struct NeunodeRegistry.AgentRegistration",
				components: [
					{
						name: "didHash",
						type: "bytes32",
						internalType: "bytes32",
					},
					{
						name: "capabilities",
						type: "string",
						internalType: "string",
					},
					{
						name: "endpoint",
						type: "string",
						internalType: "string",
					},
					{
						name: "stakeAmount",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "registeredAt",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "updatedAt",
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
		name: "getAgents",
		inputs: [
			{
				name: "offset",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "limit",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "",
				type: "bytes32[]",
				internalType: "bytes32[]",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getTotalAgents",
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
		name: "identity",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "contract INeunodeIdentity",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "register",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "capabilities",
				type: "string",
				internalType: "string",
			},
			{
				name: "endpoint",
				type: "string",
				internalType: "string",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "update",
		inputs: [
			{
				name: "didHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "capabilities",
				type: "string",
				internalType: "string",
			},
			{
				name: "endpoint",
				type: "string",
				internalType: "string",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "event",
		name: "AgentDeregistered",
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
		name: "AgentRegistered",
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
		name: "AgentUpdated",
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
		type: "error",
		name: "AgentAlreadyRegistered",
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
		name: "AgentNotActive",
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
		name: "AgentNotFound",
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
		name: "EmptyCapabilities",
		inputs: [],
	},
	{
		type: "error",
		name: "EmptyEndpoint",
		inputs: [],
	},
	{
		type: "error",
		name: "NotDidController",
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
		name: "ZeroAddress",
		inputs: [],
	},
] as const;
