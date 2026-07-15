// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const diamondAbi = [
	{
		type: "constructor",
		inputs: [
			{
				name: "_diamondCut",
				type: "tuple[]",
				internalType: "struct IDiamondCut.FacetCut[]",
				components: [
					{
						name: "facetAddress",
						type: "address",
						internalType: "address",
					},
					{
						name: "action",
						type: "uint8",
						internalType: "enum IDiamondCut.FacetCutAction",
					},
					{
						name: "functionSelectors",
						type: "bytes4[]",
						internalType: "bytes4[]",
					},
				],
			},
			{
				name: "_init",
				type: "address",
				internalType: "address",
			},
			{
				name: "_calldata",
				type: "bytes",
				internalType: "bytes",
			},
			{
				name: "_owner",
				type: "address",
				internalType: "address",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "fallback",
		stateMutability: "payable",
	},
	{
		type: "receive",
		stateMutability: "payable",
	},
	{
		type: "event",
		name: "DiamondCut",
		inputs: [
			{
				name: "_diamondCut",
				type: "tuple[]",
				indexed: false,
				internalType: "struct IDiamondCut.FacetCut[]",
				components: [
					{
						name: "facetAddress",
						type: "address",
						internalType: "address",
					},
					{
						name: "action",
						type: "uint8",
						internalType: "enum IDiamondCut.FacetCutAction",
					},
					{
						name: "functionSelectors",
						type: "bytes4[]",
						internalType: "bytes4[]",
					},
				],
			},
			{
				name: "_init",
				type: "address",
				indexed: false,
				internalType: "address",
			},
			{
				name: "_calldata",
				type: "bytes",
				indexed: false,
				internalType: "bytes",
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
		type: "error",
		name: "FacetAddressNotZeroForRemove",
		inputs: [],
	},
	{
		type: "error",
		name: "FacetAddressZeroForAdd",
		inputs: [],
	},
	{
		type: "error",
		name: "FunctionNotFound",
		inputs: [
			{
				name: "selector",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
	},
	{
		type: "error",
		name: "InitReverted",
		inputs: [],
	},
	{
		type: "error",
		name: "NoSelectorsProvided",
		inputs: [],
	},
	{
		type: "error",
		name: "SameFacetForReplace",
		inputs: [
			{
				name: "selector",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
	},
	{
		type: "error",
		name: "SelectorAlreadyExists",
		inputs: [
			{
				name: "selector",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
	},
	{
		type: "error",
		name: "SelectorNotFound",
		inputs: [
			{
				name: "selector",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
	},
] as const;

export const diamondCutFacetAbi = [
	{
		type: "function",
		name: "diamondCut",
		inputs: [
			{
				name: "_diamondCut",
				type: "tuple[]",
				internalType: "struct IDiamondCut.FacetCut[]",
				components: [
					{
						name: "facetAddress",
						type: "address",
						internalType: "address",
					},
					{
						name: "action",
						type: "uint8",
						internalType: "enum IDiamondCut.FacetCutAction",
					},
					{
						name: "functionSelectors",
						type: "bytes4[]",
						internalType: "bytes4[]",
					},
				],
			},
			{
				name: "_init",
				type: "address",
				internalType: "address",
			},
			{
				name: "_calldata",
				type: "bytes",
				internalType: "bytes",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "event",
		name: "DiamondCut",
		inputs: [
			{
				name: "_diamondCut",
				type: "tuple[]",
				indexed: false,
				internalType: "struct IDiamondCut.FacetCut[]",
				components: [
					{
						name: "facetAddress",
						type: "address",
						internalType: "address",
					},
					{
						name: "action",
						type: "uint8",
						internalType: "enum IDiamondCut.FacetCutAction",
					},
					{
						name: "functionSelectors",
						type: "bytes4[]",
						internalType: "bytes4[]",
					},
				],
			},
			{
				name: "_init",
				type: "address",
				indexed: false,
				internalType: "address",
			},
			{
				name: "_calldata",
				type: "bytes",
				indexed: false,
				internalType: "bytes",
			},
		],
		anonymous: false,
	},
	{
		type: "error",
		name: "FacetAddressNotZeroForRemove",
		inputs: [],
	},
	{
		type: "error",
		name: "FacetAddressZeroForAdd",
		inputs: [],
	},
	{
		type: "error",
		name: "InitReverted",
		inputs: [],
	},
	{
		type: "error",
		name: "NoSelectorsProvided",
		inputs: [],
	},
	{
		type: "error",
		name: "NotContractOwner",
		inputs: [
			{
				name: "caller",
				type: "address",
				internalType: "address",
			},
			{
				name: "owner",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "SameFacetForReplace",
		inputs: [
			{
				name: "selector",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
	},
	{
		type: "error",
		name: "SelectorAlreadyExists",
		inputs: [
			{
				name: "selector",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
	},
	{
		type: "error",
		name: "SelectorNotFound",
		inputs: [
			{
				name: "selector",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
	},
] as const;

export const diamondLoupeFacetAbi = [
	{
		type: "function",
		name: "facetAddress",
		inputs: [
			{
				name: "_functionSelector",
				type: "bytes4",
				internalType: "bytes4",
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
		name: "facetAddresses",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address[]",
				internalType: "address[]",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "facetFunctionSelectors",
		inputs: [
			{
				name: "_facet",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [
			{
				name: "",
				type: "bytes4[]",
				internalType: "bytes4[]",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "facets",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "tuple[]",
				internalType: "struct IDiamondLoupe.Facet[]",
				components: [
					{
						name: "facetAddress",
						type: "address",
						internalType: "address",
					},
					{
						name: "functionSelectors",
						type: "bytes4[]",
						internalType: "bytes4[]",
					},
				],
			},
		],
		stateMutability: "view",
	},
] as const;

export const libDiamondErrors = [
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
		type: "error",
		name: "FacetAddressNotZeroForRemove",
		inputs: [],
	},
	{
		type: "error",
		name: "FacetAddressZeroForAdd",
		inputs: [],
	},
	{
		type: "error",
		name: "InitReverted",
		inputs: [],
	},
	{
		type: "error",
		name: "NoSelectorsProvided",
		inputs: [],
	},
	{
		type: "error",
		name: "NotContractOwner",
		inputs: [
			{
				name: "caller",
				type: "address",
				internalType: "address",
			},
			{
				name: "owner",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "SameFacetForReplace",
		inputs: [
			{
				name: "selector",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
	},
	{
		type: "error",
		name: "SelectorAlreadyExists",
		inputs: [
			{
				name: "selector",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
	},
	{
		type: "error",
		name: "SelectorNotFound",
		inputs: [
			{
				name: "selector",
				type: "bytes4",
				internalType: "bytes4",
			},
		],
	},
] as const;
