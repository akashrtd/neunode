/**
 * NeunodeBounty ABI — Bounty state machine for agent work coordination.
 * Source: contracts/src/NeunodeBounty.sol
 * Inherited: AccessControl (hasRole, getRoleAdmin, grantRole, revokeRole, renounceRole)
 *
 * 16 write functions: createBounty, createBountyWithDeadlines, claimBounty,
 * claimBountyWithBond, submitWork, acceptSubmission, rejectSubmission,
 * disputeBounty, resolveDispute, cancelBounty, checkExpiry, requestRevision,
 * payBounty, payBountyWithFees, startReview, processReviewResult
 */

export const neunodeBountyAbi = [
  // AccessControl
  {
    type: 'function' as const,
    name: 'hasRole',
    inputs: [
      { name: 'role', type: 'bytes32' },
      { name: 'account', type: 'address' },
    ],
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getRoleAdmin',
    inputs: [{ name: 'role', type: 'bytes32' }],
    outputs: [{ name: '', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'grantRole',
    inputs: [
      { name: 'role', type: 'bytes32' },
      { name: 'account', type: 'address' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'revokeRole',
    inputs: [
      { name: 'role', type: 'bytes32' },
      { name: 'account', type: 'address' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'renounceRole',
    inputs: [
      { name: 'role', type: 'bytes32' },
      { name: 'callerConfirmation', type: 'address' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'event' as const,
    name: 'RoleAdminChanged',
    inputs: [
      { name: 'role', type: 'bytes32', indexed: true },
      { name: 'previousAdminRole', type: 'bytes32', indexed: true },
      { name: 'newAdminRole', type: 'bytes32', indexed: true },
    ],
  },
  {
    type: 'event' as const,
    name: 'RoleGranted',
    inputs: [
      { name: 'role', type: 'bytes32', indexed: true },
      { name: 'account', type: 'address', indexed: true },
      { name: 'sender', type: 'address', indexed: true },
    ],
  },
  {
    type: 'event' as const,
    name: 'RoleRevoked',
    inputs: [
      { name: 'role', type: 'bytes32', indexed: true },
      { name: 'account', type: 'address', indexed: true },
      { name: 'sender', type: 'address', indexed: true },
    ],
  },

  // Storage getters
  {
    type: 'function' as const,
    name: 'bounties',
    inputs: [{ name: '', type: 'bytes32' }],
    outputs: [
      { name: 'id', type: 'bytes32' },
      { name: 'requester', type: 'address' },
      { name: 'provider', type: 'address' },
      { name: 'state', type: 'uint8' },
      { name: 'reward', type: 'uint256' },
      { name: 'rewardToken', type: 'address' },
      { name: 'claimDeadline', type: 'uint256' },
      { name: 'workDeadline', type: 'uint256' },
      { name: 'reviewDeadline', type: 'uint256' },
      { name: 'created', type: 'uint256' },
      { name: 'submissionHash', type: 'bytes32' },
      { name: 'revisionCount', type: 'uint256' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'bountyList',
    inputs: [{ name: '', type: 'uint256' }],
    outputs: [{ name: '', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'activeCount',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'revisionDeadlines',
    inputs: [{ name: '', type: 'bytes32' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'disputeDeadlines',
    inputs: [{ name: '', type: 'bytes32' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'useEscrowFlags',
    inputs: [{ name: '', type: 'bytes32' }],
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'providerBonds',
    inputs: [{ name: '', type: 'bytes32' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'feeConfig',
    inputs: [],
    outputs: [
      { name: 'protocolBps', type: 'uint256' },
      { name: 'reviewerBps', type: 'uint256' },
      { name: 'verificationBps', type: 'uint256' },
      { name: 'protocolFeeRecipient', type: 'address' },
      { name: 'reviewerFeeRecipient', type: 'address' },
      { name: 'verificationFeeRecipient', type: 'address' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'escrow',
    inputs: [],
    outputs: [{ name: '', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'reviewContract',
    inputs: [],
    outputs: [{ name: '', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'ADMIN_ROLE',
    inputs: [],
    outputs: [{ name: '', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'BOUNTY_MANAGER_ROLE',
    inputs: [],
    outputs: [{ name: '', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'MAX_REVISIONS',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },

  // Admin functions
  {
    type: 'function' as const,
    name: 'setFeeConfig',
    inputs: [
      { name: 'protocolBps', type: 'uint256' },
      { name: 'reviewerBps', type: 'uint256' },
      { name: 'verificationBps', type: 'uint256' },
      { name: 'protocolFeeRecipient', type: 'address' },
      { name: 'reviewerFeeRecipient', type: 'address' },
      { name: 'verificationFeeRecipient', type: 'address' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'setEscrow',
    inputs: [{ name: 'escrow_', type: 'address' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'setReviewContract',
    inputs: [{ name: 'review_', type: 'address' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // Create
  {
    type: 'function' as const,
    name: 'createBounty',
    inputs: [
      { name: 'id', type: 'bytes32' },
      { name: 'reward', type: 'uint256' },
      { name: 'rewardToken', type: 'address' },
      { name: 'claimDeadline', type: 'uint256' },
      { name: 'workDeadline', type: 'uint256' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'createBountyWithDeadlines',
    inputs: [
      { name: 'id', type: 'bytes32' },
      { name: 'reward', type: 'uint256' },
      { name: 'rewardToken', type: 'address' },
      { name: 'claimDeadline', type: 'uint256' },
      { name: 'workDeadline', type: 'uint256' },
      { name: 'reviewDeadline_', type: 'uint256' },
      { name: 'revisionDeadline_', type: 'uint256' },
      { name: 'disputeDeadline_', type: 'uint256' },
      { name: 'useEscrow_', type: 'bool' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // Claim
  {
    type: 'function' as const,
    name: 'claimBounty',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'claimBountyWithBond',
    inputs: [
      { name: 'id', type: 'bytes32' },
      { name: 'bondAmount', type: 'uint256' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // Submit
  {
    type: 'function' as const,
    name: 'submitWork',
    inputs: [
      { name: 'id', type: 'bytes32' },
      { name: 'submissionHash', type: 'bytes32' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // Accept / Reject
  {
    type: 'function' as const,
    name: 'acceptSubmission',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'rejectSubmission',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // Dispute
  {
    type: 'function' as const,
    name: 'disputeBounty',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'resolveDispute',
    inputs: [
      { name: 'id', type: 'bytes32' },
      { name: 'accept', type: 'bool' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // Cancel
  {
    type: 'function' as const,
    name: 'cancelBounty',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // Expiry
  {
    type: 'function' as const,
    name: 'checkExpiry',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // Revision
  {
    type: 'function' as const,
    name: 'requestRevision',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // Pay
  {
    type: 'function' as const,
    name: 'payBounty',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'payBountyWithFees',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // Review integration
  {
    type: 'function' as const,
    name: 'startReview',
    inputs: [
      { name: 'id', type: 'bytes32' },
      { name: 'reviewers', type: 'address[3]' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'processReviewResult',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // View functions
  {
    type: 'function' as const,
    name: 'getBountyState',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [{ name: '', type: 'uint8' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getTotalBounties',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getBountyFull',
    inputs: [{ name: 'id', type: 'bytes32' }],
    outputs: [
      { name: 'bountyId', type: 'bytes32' },
      { name: 'requester_', type: 'address' },
      { name: 'provider_', type: 'address' },
      { name: 'state', type: 'uint8' },
      { name: 'reward', type: 'uint256' },
      { name: 'rewardToken', type: 'address' },
      { name: 'claimDeadline_', type: 'uint256' },
      { name: 'workDeadline_', type: 'uint256' },
      { name: 'reviewDeadline_', type: 'uint256' },
      { name: 'created', type: 'uint256' },
      { name: 'submissionHash', type: 'bytes32' },
      { name: 'revisionCount_', type: 'uint256' },
      { name: 'revisionDeadline_', type: 'uint256' },
      { name: 'disputeDeadline_', type: 'uint256' },
      { name: 'useEscrow_', type: 'bool' },
      { name: 'providerBond_', type: 'uint256' },
    ],
    stateMutability: 'view',
  },

  // Events
  {
    type: 'event' as const,
    name: 'BountyCreated',
    inputs: [
      { name: 'id', type: 'bytes32', indexed: true },
      { name: 'requester', type: 'address', indexed: true },
      { name: 'reward', type: 'uint256', indexed: false },
      { name: 'rewardToken', type: 'address', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'BountyClaimed',
    inputs: [
      { name: 'id', type: 'bytes32', indexed: true },
      { name: 'provider', type: 'address', indexed: true },
    ],
  },
  {
    type: 'event' as const,
    name: 'BountySubmitted',
    inputs: [
      { name: 'id', type: 'bytes32', indexed: true },
      { name: 'submissionHash', type: 'bytes32', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'BountyReviewStarted',
    inputs: [{ name: 'id', type: 'bytes32', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'BountyRevisionRequested',
    inputs: [{ name: 'id', type: 'bytes32', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'BountyAccepted',
    inputs: [{ name: 'id', type: 'bytes32', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'BountyRejected',
    inputs: [{ name: 'id', type: 'bytes32', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'BountyDisputed',
    inputs: [{ name: 'id', type: 'bytes32', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'BountyPaid',
    inputs: [
      { name: 'id', type: 'bytes32', indexed: true },
      { name: 'provider', type: 'address', indexed: true },
      { name: 'amount', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'BountyCancelled',
    inputs: [{ name: 'id', type: 'bytes32', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'BountyExpired',
    inputs: [{ name: 'id', type: 'bytes32', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'FeeConfigUpdated',
    inputs: [{ name: 'admin', type: 'address', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'EscrowUpdated',
    inputs: [{ name: 'escrow', type: 'address', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'ReviewContractUpdated',
    inputs: [{ name: 'reviewContract', type: 'address', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'FeesCollected',
    inputs: [
      { name: 'bountyId', type: 'bytes32', indexed: true },
      { name: 'protocolFee', type: 'uint256', indexed: false },
      { name: 'reviewerFee', type: 'uint256', indexed: false },
      { name: 'verificationFee', type: 'uint256', indexed: false },
      { name: 'providerPayout', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'DisputeResolved',
    inputs: [
      { name: 'bountyId', type: 'bytes32', indexed: true },
      { name: 'accepted', type: 'bool', indexed: false },
    ],
  },

  // Errors
  {
    type: 'error' as const,
    name: 'BountyNotFound',
    inputs: [{ name: 'id', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'BountyAlreadyExists',
    inputs: [{ name: 'id', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'InvalidState',
    inputs: [
      { name: 'id', type: 'bytes32' },
      { name: 'current', type: 'uint8' },
      { name: 'required', type: 'uint8' },
    ],
  },
  {
    type: 'error' as const,
    name: 'NotRequester',
    inputs: [
      { name: 'id', type: 'bytes32' },
      { name: 'caller', type: 'address' },
    ],
  },
  {
    type: 'error' as const,
    name: 'NotProvider',
    inputs: [
      { name: 'id', type: 'bytes32' },
      { name: 'caller', type: 'address' },
    ],
  },
  {
    type: 'error' as const,
    name: 'NotClaimer',
    inputs: [
      { name: 'id', type: 'bytes32' },
      { name: 'caller', type: 'address' },
    ],
  },
  {
    type: 'error' as const,
    name: 'InvalidDeadline',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'InvalidReward',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'DeadlinePassed',
    inputs: [{ name: 'deadline', type: 'uint256' }],
  },
  {
    type: 'error' as const,
    name: 'MaxRevisionsReached',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'ReviewNotResolved',
    inputs: [{ name: 'id', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'ReviewNotAccepted',
    inputs: [{ name: 'id', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'InsufficientBond',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'TotalFeesExceed100',
    inputs: [],
  },
] as const;
