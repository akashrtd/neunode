// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const neunodeGovernanceAbi = [
	{
		type: "constructor",
		inputs: [
			{
				name: "token_",
				type: "address",
				internalType: "address",
			},
			{
				name: "votingDelay_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "votingPeriod_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "proposalThreshold_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "quorumBps_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "timelock_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "executionWindow_",
				type: "uint256",
				internalType: "uint256",
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
		name: "MIN_TIMELOCK",
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
		name: "MIN_VOTING_DELAY",
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
		name: "MIN_VOTING_PERIOD",
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
		name: "allowedTargets",
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
		name: "cancel",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "castVote",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "support",
				type: "uint8",
				internalType: "uint8",
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "castVoteWithReason",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "support",
				type: "uint8",
				internalType: "uint8",
			},
			{
				name: "reason",
				type: "string",
				internalType: "string",
			},
		],
		outputs: [
			{
				name: "",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "checkpoint",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "execute",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "payable",
	},
	{
		type: "function",
		name: "executionWindow",
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
		name: "getProposal",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "proposer_",
				type: "address",
				internalType: "address",
			},
			{
				name: "voteStart",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "voteEnd",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "forVotes",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "againstVotes",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "abstainVotes",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "snapshotBlock_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "executed_",
				type: "bool",
				internalType: "bool",
			},
			{
				name: "cancelled_",
				type: "bool",
				internalType: "bool",
			},
			{
				name: "queuedAt",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getProposalActions",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "targets",
				type: "address[]",
				internalType: "address[]",
			},
			{
				name: "values",
				type: "uint256[]",
				internalType: "uint256[]",
			},
			{
				name: "calldatas",
				type: "bytes[]",
				internalType: "bytes[]",
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
		name: "getVotes",
		inputs: [
			{
				name: "account",
				type: "address",
				internalType: "address",
			},
			{
				name: "blockNumber",
				type: "uint256",
				internalType: "uint256",
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
		name: "hasVoted",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
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
		name: "proposalCount",
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
		name: "proposalThreshold",
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
		name: "propose",
		inputs: [
			{
				name: "targets",
				type: "address[]",
				internalType: "address[]",
			},
			{
				name: "values",
				type: "uint256[]",
				internalType: "uint256[]",
			},
			{
				name: "calldatas",
				type: "bytes[]",
				internalType: "bytes[]",
			},
			{
				name: "description",
				type: "string",
				internalType: "string",
			},
		],
		outputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "queue",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "quorumBps",
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
		name: "setAllowedTarget",
		inputs: [
			{
				name: "target",
				type: "address",
				internalType: "address",
			},
			{
				name: "allowed",
				type: "bool",
				internalType: "bool",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setExecutionWindow",
		inputs: [
			{
				name: "newWindow",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setProposalThreshold",
		inputs: [
			{
				name: "newThreshold",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setQuorumBps",
		inputs: [
			{
				name: "newQuorumBps",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setTimelock",
		inputs: [
			{
				name: "newTimelock",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setVotingDelay",
		inputs: [
			{
				name: "newVotingDelay",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setVotingPeriod",
		inputs: [
			{
				name: "newVotingPeriod",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "state",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
		outputs: [
			{
				name: "",
				type: "uint8",
				internalType: "enum IGovernance.ProposalState",
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
		name: "timelock",
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
		name: "unpause",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "votingDelay",
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
		name: "votingPeriod",
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
		type: "event",
		name: "AllowedTargetUpdated",
		inputs: [
			{
				name: "target",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "allowed",
				type: "bool",
				indexed: false,
				internalType: "bool",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "GovernanceParametersUpdated",
		inputs: [
			{
				name: "updater",
				type: "address",
				indexed: true,
				internalType: "address",
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
		name: "ProposalCancelled",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				indexed: true,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ProposalCreated",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				indexed: true,
				internalType: "uint256",
			},
			{
				name: "proposer",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "targets",
				type: "address[]",
				indexed: false,
				internalType: "address[]",
			},
			{
				name: "values",
				type: "uint256[]",
				indexed: false,
				internalType: "uint256[]",
			},
			{
				name: "calldatas",
				type: "bytes[]",
				indexed: false,
				internalType: "bytes[]",
			},
			{
				name: "descriptionHash",
				type: "bytes32",
				indexed: false,
				internalType: "bytes32",
			},
			{
				name: "voteStart",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "voteEnd",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ProposalExecuted",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				indexed: true,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ProposalQueued",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				indexed: true,
				internalType: "uint256",
			},
			{
				name: "eta",
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
		name: "VoteCast",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				indexed: true,
				internalType: "uint256",
			},
			{
				name: "voter",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "support",
				type: "uint8",
				indexed: false,
				internalType: "uint8",
			},
			{
				name: "weight",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "reason",
				type: "string",
				indexed: false,
				internalType: "string",
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
		name: "AlreadyVoted",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "voter",
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
		name: "BelowProposalThreshold",
		inputs: [
			{
				name: "proposer",
				type: "address",
				internalType: "address",
			},
			{
				name: "threshold",
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
		name: "EmptyProposal",
		inputs: [],
	},
	{
		type: "error",
		name: "EnforcedPause",
		inputs: [],
	},
	{
		type: "error",
		name: "ExecutionFailed",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ExpectedPause",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidVoteType",
		inputs: [
			{
				name: "support",
				type: "uint8",
				internalType: "uint8",
			},
		],
	},
	{
		type: "error",
		name: "NotAuthorized",
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
		name: "ProposalAlreadyCancelled",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ProposalAlreadyExecuted",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ProposalNotActive",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ProposalNotCancellable",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ProposalNotFound",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ProposalNotQueued",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ProposalNotReady",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "ProposalNotSucceeded",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "QuorumNotReached",
		inputs: [
			{
				name: "proposalId",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "TargetNotAllowed",
		inputs: [
			{
				name: "target",
				type: "address",
				internalType: "address",
			},
		],
	},
	{
		type: "error",
		name: "TimelockBelowMinimum",
		inputs: [
			{
				name: "provided",
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
		name: "VotingDelayBelowMinimum",
		inputs: [
			{
				name: "provided",
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
		name: "VotingPeriodBelowMinimum",
		inputs: [
			{
				name: "provided",
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
		name: "VotingPowerZero",
		inputs: [
			{
				name: "voter",
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
