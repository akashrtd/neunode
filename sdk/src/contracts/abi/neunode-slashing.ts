// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const neunodeSlashingAbi = [
	{
		type: "constructor",
		inputs: [
			{
				name: "token_",
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
		name: "MAX_BPS",
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
		name: "REPORTER_ROLE",
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
		name: "REPUTATION_SLASH_MULTIPLIER",
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
		name: "SLASHING_ROLE",
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
		name: "getOffenseCount",
		inputs: [
			{
				name: "validator",
				type: "address",
				internalType: "address",
			},
			{
				name: "offense",
				type: "uint8",
				internalType: "enum NeunodeSlashing.OffenseType",
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
		name: "getPenaltySchedule",
		inputs: [
			{
				name: "offense",
				type: "uint8",
				internalType: "enum NeunodeSlashing.OffenseType",
			},
			{
				name: "offenseCount",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "",
				type: "tuple",
				internalType: "struct NeunodeSlashing.SlashingPenalty",
				components: [
					{
						name: "stakeSlashBps",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "reputationSlashBps",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "jailDurationBlocks",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "outcome",
						type: "uint8",
						internalType: "enum NeunodeSlashing.PenaltyOutcome",
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
		name: "getValidatorStatus",
		inputs: [
			{
				name: "validator",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [
			{
				name: "",
				type: "tuple",
				internalType: "struct NeunodeSlashing.ValidatorStatus",
				components: [
					{
						name: "isJailed",
						type: "bool",
						internalType: "bool",
					},
					{
						name: "jailReleaseBlock",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "offenseCount",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "isTombstoned",
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
		name: "isEvidenceSeen",
		inputs: [
			{
				name: "evidenceHash",
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
		name: "isJailed",
		inputs: [
			{
				name: "validator",
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
		name: "reportDoubleSign",
		inputs: [
			{
				name: "header1",
				type: "bytes",
				internalType: "bytes",
			},
			{
				name: "header2",
				type: "bytes",
				internalType: "bytes",
			},
			{
				name: "sig1",
				type: "bytes",
				internalType: "bytes",
			},
			{
				name: "sig2",
				type: "bytes",
				internalType: "bytes",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "reportDowntime",
		inputs: [
			{
				name: "validator",
				type: "address",
				internalType: "address",
			},
			{
				name: "missedBlocks",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "windowBlocks",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "reputation",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "contract IReputationPenalty",
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
		name: "setPenaltySchedule",
		inputs: [
			{
				name: "offense",
				type: "uint8",
				internalType: "enum NeunodeSlashing.OffenseType",
			},
			{
				name: "tier",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "stakeSlashBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "reputationSlashBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "jailDurationBlocks",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "outcome",
				type: "uint8",
				internalType: "enum NeunodeSlashing.PenaltyOutcome",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setReputationContract",
		inputs: [
			{
				name: "reputation_",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "slashingEventCount",
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
		name: "submitEvidence",
		inputs: [
			{
				name: "offense",
				type: "uint8",
				internalType: "enum NeunodeSlashing.OffenseType",
			},
			{
				name: "evidence",
				type: "tuple",
				internalType: "struct NeunodeSlashing.SlashingEvidence",
				components: [
					{
						name: "blockHash1",
						type: "bytes32",
						internalType: "bytes32",
					},
					{
						name: "blockHash2",
						type: "bytes32",
						internalType: "bytes32",
					},
					{
						name: "blockNumber",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "signature1",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "signature2",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "extraData",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "reporter",
						type: "address",
						internalType: "address",
					},
					{
						name: "timestamp",
						type: "uint256",
						internalType: "uint256",
					},
				],
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
		name: "token",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "contract INeunodeToken",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "unjail",
		inputs: [
			{
				name: "validator",
				type: "address",
				internalType: "address",
			},
		],
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
		type: "event",
		name: "EvidenceSubmitted",
		inputs: [
			{
				name: "validator",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "offense",
				type: "uint8",
				indexed: false,
				internalType: "enum NeunodeSlashing.OffenseType",
			},
			{
				name: "reporter",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "evidenceHash",
				type: "bytes32",
				indexed: false,
				internalType: "bytes32",
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
		name: "PenaltyScheduleUpdated",
		inputs: [
			{
				name: "offense",
				type: "uint8",
				indexed: true,
				internalType: "enum NeunodeSlashing.OffenseType",
			},
			{
				name: "tier",
				type: "uint256",
				indexed: true,
				internalType: "uint256",
			},
			{
				name: "stakeSlashBps",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "reputationSlashBps",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "jailDurationBlocks",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "outcome",
				type: "uint8",
				indexed: false,
				internalType: "enum NeunodeSlashing.PenaltyOutcome",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ReputationContractUpdated",
		inputs: [
			{
				name: "reputation",
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
		type: "event",
		name: "ValidatorJailed",
		inputs: [
			{
				name: "validator",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "releaseBlock",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ValidatorSlashed",
		inputs: [
			{
				name: "validator",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "offense",
				type: "uint8",
				indexed: false,
				internalType: "enum NeunodeSlashing.OffenseType",
			},
			{
				name: "stakeSlashed",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "reputationSlashed",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ValidatorTombstoned",
		inputs: [
			{
				name: "validator",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ValidatorUnjailed",
		inputs: [
			{
				name: "validator",
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
		name: "DowntimeThresholdNotMet",
		inputs: [
			{
				name: "missed",
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
		name: "DuplicateEvidence",
		inputs: [
			{
				name: "hash",
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
		name: "EvidenceExpired",
		inputs: [],
	},
	{
		type: "error",
		name: "ExpectedPause",
		inputs: [],
	},
	{
		type: "error",
		name: "InsufficientEvidence",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidEvidence",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidOffenseType",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidPenaltyBps",
		inputs: [
			{
				name: "bps",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "InvalidSignature",
		inputs: [],
	},
	{
		type: "error",
		name: "JailNotExpired",
		inputs: [],
	},
	{
		type: "error",
		name: "ReporterNotAuthorized",
		inputs: [],
	},
	{
		type: "error",
		name: "SameBlockHashes",
		inputs: [],
	},
	{
		type: "error",
		name: "SignaturesDoNotMatchValidator",
		inputs: [],
	},
	{
		type: "error",
		name: "ValidatorAlreadyTombstoned",
		inputs: [],
	},
	{
		type: "error",
		name: "ValidatorNotJailed",
		inputs: [],
	},
	{
		type: "error",
		name: "ZeroAddress",
		inputs: [],
	},
] as const;
