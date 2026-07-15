// Generated from Foundry artifacts by scripts/contract-abis.mjs.
// Do not edit manually.

export const neunodeBountyAbi = [
	{
		type: "constructor",
		inputs: [],
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
		name: "BOUNTY_MANAGER_ROLE",
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
		name: "CLAIM_COMMIT_TIMEOUT",
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
		name: "FEE_CHANGE_TIMELOCK",
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
		name: "MAX_REVISIONS",
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
		name: "acceptSubmission",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
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
		name: "bounties",
		inputs: [
			{
				name: "",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "id",
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
				name: "state",
				type: "uint8",
				internalType: "enum NeunodeBounty.BountyState",
			},
			{
				name: "reward",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "rewardToken",
				type: "address",
				internalType: "address",
			},
			{
				name: "claimDeadline",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "workDeadline",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "reviewDeadline",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "created",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "submissionHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "revisionCount",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "bountyList",
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
		name: "cancelBounty",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "cancelFeeConfigProposal",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "checkExpiry",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "claimBounty",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "claimBountyWithBond",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
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
		name: "commitClaim",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "commitment",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "createBounty",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "reward",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "rewardToken",
				type: "address",
				internalType: "address",
			},
			{
				name: "claimDeadline",
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
		name: "createBountyWithDeadlines",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "reward",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "rewardToken",
				type: "address",
				internalType: "address",
			},
			{
				name: "claimDeadline",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "workDeadline",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "reviewDeadline_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "revisionDeadline_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "disputeDeadline_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "useEscrow_",
				type: "bool",
				internalType: "bool",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "disputeBounty",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "disputeDeadlines",
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
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "escrow",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "contract IBountyEscrow",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "executeFeeConfig",
		inputs: [],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "expireCommitment",
		inputs: [
			{
				name: "claimer",
				type: "address",
				internalType: "address",
			},
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
		name: "feeConfig",
		inputs: [],
		outputs: [
			{
				name: "protocolBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "reviewerBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "verificationBps",
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
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getBountyFull",
		inputs: [
			{
				name: "id",
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
				name: "requester_",
				type: "address",
				internalType: "address",
			},
			{
				name: "provider_",
				type: "address",
				internalType: "address",
			},
			{
				name: "state",
				type: "uint8",
				internalType: "enum NeunodeBounty.BountyState",
			},
			{
				name: "reward",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "rewardToken",
				type: "address",
				internalType: "address",
			},
			{
				name: "claimDeadline_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "workDeadline_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "reviewDeadline_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "created",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "submissionHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "revisionCount_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "revisionDeadline_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "disputeDeadline_",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "useEscrow_",
				type: "bool",
				internalType: "bool",
			},
			{
				name: "providerBond_",
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "getBountyState",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [
			{
				name: "",
				type: "uint8",
				internalType: "enum NeunodeBounty.BountyState",
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
		name: "getTotalBounties",
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
		name: "payBounty",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "payBountyWithFees",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "pendingFeeConfig",
		inputs: [],
		outputs: [
			{
				name: "protocolBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "reviewerBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "verificationBps",
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
		stateMutability: "view",
	},
	{
		type: "function",
		name: "pendingFeeConfigTimestamp",
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
		name: "processReviewResult",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "proposeFeeConfig",
		inputs: [
			{
				name: "protocolBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "reviewerBps",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "verificationBps",
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
		name: "providerBonds",
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
				type: "uint256",
				internalType: "uint256",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "rejectSubmission",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
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
		name: "requestRevision",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "resolveDispute",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "accept",
				type: "bool",
				internalType: "bool",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "revealClaim",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "bondAmount",
				type: "uint256",
				internalType: "uint256",
			},
			{
				name: "nonce",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "revealWork",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "artifactHash",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "salt",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "reviewContract",
		inputs: [],
		outputs: [
			{
				name: "",
				type: "address",
				internalType: "contract IBountyReview",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "revisionDeadlines",
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
				type: "uint256",
				internalType: "uint256",
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
		name: "setEscrow",
		inputs: [
			{
				name: "escrow_",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "setReviewContract",
		inputs: [
			{
				name: "review_",
				type: "address",
				internalType: "address",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "startReview",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "reviewers",
				type: "address[3]",
				internalType: "address[3]",
			},
		],
		outputs: [],
		stateMutability: "nonpayable",
	},
	{
		type: "function",
		name: "submissionCommitments",
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
				type: "bytes32",
				internalType: "bytes32",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "submissionRevealed",
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
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "function",
		name: "submitWork",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "commitment",
				type: "bytes32",
				internalType: "bytes32",
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
		name: "useEscrowFlags",
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
				type: "bool",
				internalType: "bool",
			},
		],
		stateMutability: "view",
	},
	{
		type: "event",
		name: "BountyAccepted",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "BountyCancelled",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "BountyClaimed",
		inputs: [
			{
				name: "id",
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
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "BountyCreated",
		inputs: [
			{
				name: "id",
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
				name: "reward",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
			{
				name: "rewardToken",
				type: "address",
				indexed: false,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "BountyDisputed",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "BountyExpired",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "BountyPaid",
		inputs: [
			{
				name: "id",
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
		name: "BountyRejected",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "BountyReviewStarted",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "BountyRevisionRequested",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "BountySubmitted",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "commitment",
				type: "bytes32",
				indexed: false,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ClaimCommitted",
		inputs: [
			{
				name: "claimer",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ClaimRevealed",
		inputs: [
			{
				name: "claimer",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "DisputeResolved",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "accepted",
				type: "bool",
				indexed: false,
				internalType: "bool",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "EscrowUpdated",
		inputs: [
			{
				name: "escrow",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "FeeConfigCancelled",
		inputs: [
			{
				name: "admin",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "FeeConfigProposed",
		inputs: [
			{
				name: "admin",
				type: "address",
				indexed: true,
				internalType: "address",
			},
			{
				name: "executesAt",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "FeeConfigUpdated",
		inputs: [
			{
				name: "admin",
				type: "address",
				indexed: true,
				internalType: "address",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "FeesCollected",
		inputs: [
			{
				name: "bountyId",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
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
			{
				name: "providerPayout",
				type: "uint256",
				indexed: false,
				internalType: "uint256",
			},
		],
		anonymous: false,
	},
	{
		type: "event",
		name: "ReviewContractUpdated",
		inputs: [
			{
				name: "reviewContract",
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
		name: "WorkRevealed",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				indexed: true,
				internalType: "bytes32",
			},
			{
				name: "submissionHash",
				type: "bytes32",
				indexed: false,
				internalType: "bytes32",
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
		name: "AlreadyCommitted",
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
		name: "AlreadyRevealed",
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
		name: "BountyAlreadyExists",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
	{
		type: "error",
		name: "BountyNotFound",
		inputs: [
			{
				name: "id",
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
		name: "FeeChangeTimelockNotExpired",
		inputs: [
			{
				name: "expiresAt",
				type: "uint256",
				internalType: "uint256",
			},
		],
	},
	{
		type: "error",
		name: "InsufficientBond",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidDeadline",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidReveal",
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
		name: "InvalidReward",
		inputs: [],
	},
	{
		type: "error",
		name: "InvalidState",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
			{
				name: "current",
				type: "uint8",
				internalType: "enum NeunodeBounty.BountyState",
			},
			{
				name: "required",
				type: "uint8",
				internalType: "enum NeunodeBounty.BountyState",
			},
		],
	},
	{
		type: "error",
		name: "MaxRevisionsReached",
		inputs: [],
	},
	{
		type: "error",
		name: "NoPendingFeeChange",
		inputs: [],
	},
	{
		type: "error",
		name: "NotClaimer",
		inputs: [
			{
				name: "id",
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
		name: "NotCommitted",
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
		name: "NotProvider",
		inputs: [
			{
				name: "id",
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
				name: "id",
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
		name: "NotSubmitter",
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
		name: "ReviewContractNotSet",
		inputs: [],
	},
	{
		type: "error",
		name: "ReviewNotAccepted",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
	},
	{
		type: "error",
		name: "ReviewNotResolved",
		inputs: [
			{
				name: "id",
				type: "bytes32",
				internalType: "bytes32",
			},
		],
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
		name: "SubmissionNotRevealed",
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
		name: "TotalFeesExceed100",
		inputs: [],
	},
] as const;
