// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const neunodeReputationAbi = [
	{
		type: "constructor",
		inputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "ACTIVITY_ORACLE_ROLE",
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
		name: "ATTEST_ORACLE_ROLE",
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
		name: "EPOCH_FINALIZER_ROLE",
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
		name: "EPOCH_SIZE",
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
		name: "MAX_VALIDATORS",
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
		name: "MIN_REPUTATION_BPS",
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
		name: "PENALTY_DECAY_EPOCHS",
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
		name: "REPUTATION_ADMIN_ROLE",
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
		name: "SNAPSHOT_WINDOW",
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
		name: "STAKE_ORACLE_ROLE",
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
		name: "TENURE_ORACLE_ROLE",
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
		name: "TRANSITION_BLOCKS",
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
		name: "VERIFY_ORACLE_ROLE",
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
		name: "VOTING_POWER_SCALE",
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
		name: "applyPenalty",
		inputs: [
			{
				name: "validator",
				type: "address",
				internalType: "address",
			},
			{
				name: "reputationSlashBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "stakeSlashBps_",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "batchUpdateScores",
		inputs: [
			{
				name: "agents",
				type: "address[]",
				internalType: "address[]",
			},
			{
				name: "factorIndex",
				type: "uint8",
				internalType: "uint8",
			},
			{
				name: "scoresBps",
				type: "uint16[]",
				internalType: "uint16[]",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "currentEpoch",
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
		name: "deregisterValidator",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "deriveStakeFactor",
		inputs: [
			{
				name: "agent",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "finalizeEpoch",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "getActiveValidators",
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
		name: "getCompositeScore",
		inputs: [
			{
				name: "agent",
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
		name: "getCurrentEpoch",
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
		name: "getEpochInfo",
		inputs: [
			{
				name: "epoch",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "",
				type: "tuple",
				internalType: "struct NeunodeReputation.EpochInfo",
				components: [
					{
						name: "startBlock",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "endBlock",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "isFinalized",
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
		name: "getFactorScores",
		inputs: [
			{
				name: "agent",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [
			{
				name: "",
				type: "tuple",
				internalType: "struct NeunodeReputation.FactorScores",
				components: [
					{
						name: "stake",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "attest",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "activity",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "verify",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "tenure",
						type: "uint16",
						internalType: "uint16",
					},
				],
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getPenaltyDecay",
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
		name: "getTotalVotingPower",
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
		name: "getValidatorSetForEpoch",
		inputs: [
			{
				name: "epoch",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "validators",
				type: "address[]",
				internalType: "address[]",
			},
			{
				name: "votingPowers",
				type: "uint256[]",
				internalType: "uint256[]",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getVotingPower",
		inputs: [
			{
				name: "agent",
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
		name: "identityRegistry",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "contract IIdentityRegistry",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "isEligibleValidator",
		inputs: [
			{
				name: "agent",
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
		name: "maxValidators",
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
		name: "minReputationBps",
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
		name: "registerValidator",
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
		name: "setFactorWeights",
		inputs: [
			{
				name: "newWeights",
				type: "tuple",
				internalType: "struct NeunodeReputation.FactorWeights",
				components: [
					{
						name: "stake",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "attest",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "activity",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "verify",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "tenure",
						type: "uint16",
						internalType: "uint16",
					},
				],
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setIdentityRegistry",
		inputs: [
			{
				name: "registry",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setMaxValidators",
		inputs: [
			{
				name: "max",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setMinReputationThreshold",
		inputs: [
			{
				name: "bps",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setStakeFactorTarget",
		inputs: [
			{
				name: "target",
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
		name: "stakeFactorTarget",
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
		name: "updateFactorScore",
		inputs: [
			{
				name: "agent",
				type: "address",
				internalType: "address",
			},
			{
				name: "factorIndex",
				type: "uint8",
				internalType: "uint8",
			},
			{
				name: "scoreBps",
				type: "uint16",
				internalType: "uint16",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "weights",
		inputs: [],
		outputs: [
			{
				name: "stake",
				type: "uint16",
				internalType: "uint16",
			},
			{
				name: "attest",
				type: "uint16",
				internalType: "uint16",
			},
			{
				name: "activity",
				type: "uint16",
				internalType: "uint16",
			},
			{
				name: "verify",
				type: "uint16",
				internalType: "uint16",
			},
			{
				name: "tenure",
				type: "uint16",
				internalType: "uint16",
			},
		],
		stateMutability: "view",
	},
	{
		type: "event",
		name: "CompositeScoreUpdated",
		inputs: [
			{
				name: "agent",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "compositeScore",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "EpochFinalized",
		inputs: [
			{
				name: "epoch",
				type: "uint256",
				indexed: true,
				internalType: "uint256",
			},
			{
				name: "validatorCount",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "FactorScoreUpdated",
		inputs: [
			{
				name: "agent",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "factor",
				type: "uint8",
				indexed: true,
				internalType: "uint8",
			},
			{
				name: "scoreBps",
				type: "uint16",
				indexed: false,
				internalType: "uint16",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "IdentityRegistryUpdated",
		inputs: [
			{
				name: "registry",
				type: "address",
				indexed: false,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "PenaltyApplied",
		inputs: [
			{
				name: "validator",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "reputationSlashBps",
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
		name: "StakeFactorRecomputed",
		inputs: [
			{
				name: "agent",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "factorBps",
				type: "uint16",
				indexed: false,
				internalType: "uint16",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "StakeFactorTargetUpdated",
		inputs: [
			{
				name: "target",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "StakeSourceUpdated",
		inputs: [
			{
				name: "source",
				type: "address",
				indexed: false,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ValidatorDeregistered",
		inputs: [
			{
				name: "agent",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ValidatorRegistered",
		inputs: [
			{
				name: "agent",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "VotingPowerUpdated",
		inputs: [
			{
				name: "agent",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "votingPower",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "WeightsUpdated",
		inputs: [
			{
				name: "newWeights",
				type: "tuple",
				indexed: false,
				internalType: "struct NeunodeReputation.FactorWeights",
				components: [
					{
						name: "stake",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "attest",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "activity",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "verify",
						type: "uint16",
						internalType: "uint16",
					},
					{
						name: "tenure",
						type: "uint16",
						internalType: "uint16",
					},
				],
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
		name: "AgentNotFound",
		inputs: [
			{
				name: "agent",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "AlreadyValidator",
		inputs: [
			{
				name: "agent",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "ArrayLengthMismatch",
		inputs: [],
	},
	{
		type: "error",
		name: "EpochAlreadyFinalized",
		inputs: [
			{
				name: "epoch",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "EpochNotEnded",
		inputs: [
			{
				name: "epoch",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "currentBlock",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "endBlock",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "EpochNotFinalized",
		inputs: [
			{
				name: "epoch",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "InsufficientReputation",
		inputs: [
			{
				name: "agent",
				type: "address",
				internalType: "address",
			},
			{
				name: "score",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "minimum",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "InvalidFactorIndex",
		inputs: [
			{
				name: "index",
				type: "uint8",
				internalType: "uint8",
			},
		],
	},
	{
		type: "error",
		name: "InvalidWeightSum",
		inputs: [
			{
				name: "sum",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "MaxValidatorsReached",
		inputs: [
			{
				name: "max",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "NotAValidator",
		inputs: [
			{
				name: "agent",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "NotNetworkRegistered",
		inputs: [
			{
				name: "agent",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "PenaltyNotDecayed",
		inputs: [],
	},
	{
		type: "error",
		name: "ScoreOutOfBounds",
		inputs: [
			{
				name: "score",
				type: "uint16",
				internalType: "uint16",
			},
		],
	},
	{
		type: "error",
		name: "StakeDerivationNotConfigured",
		inputs: [],
	},
	{
		type: "error",
		name: "StakeFactorDerived",
		inputs: [],
	},
] as const;
