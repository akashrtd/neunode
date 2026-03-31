/**
 * NeunodeGovernance ABI — On-chain governance with staked token voting.
 * Source: contracts/src/governance/NeunodeGovernance.sol
 * Inherited: AccessControl, IGovernance
 *
 * Proposal lifecycle: Pending → Active → Succeeded/Defeated → Queued → Executed/Expired
 */

export const neunodeGovernanceAbi = [
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

  // Storage
  {
    type: 'function' as const,
    name: 'token',
    inputs: [],
    outputs: [{ name: '', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'proposalCount',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'votingDelay',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'votingPeriod',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'proposalThreshold',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'quorumBps',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'timelock',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'executionWindow',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'GOVERNANCE_ROLE',
    inputs: [],
    outputs: [{ name: '', type: 'bytes32' }],
    stateMutability: 'view',
  },

  // Checkpoint
  {
    type: 'function' as const,
    name: 'checkpoint',
    inputs: [],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'getVotes',
    inputs: [
      { name: 'account', type: 'address' },
      { name: 'blockNumber', type: 'uint256' },
    ],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },

  // Propose
  {
    type: 'function' as const,
    name: 'propose',
    inputs: [
      { name: 'targets', type: 'address[]' },
      { name: 'values', type: 'uint256[]' },
      { name: 'calldatas', type: 'bytes[]' },
      { name: 'description', type: 'string' },
    ],
    outputs: [{ name: 'proposalId', type: 'uint256' }],
    stateMutability: 'nonpayable',
  },

  // Vote
  {
    type: 'function' as const,
    name: 'castVote',
    inputs: [
      { name: 'proposalId', type: 'uint256' },
      { name: 'support', type: 'uint8' },
    ],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'castVoteWithReason',
    inputs: [
      { name: 'proposalId', type: 'uint256' },
      { name: 'support', type: 'uint8' },
      { name: 'reason', type: 'string' },
    ],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'nonpayable',
  },

  // Queue / Execute / Cancel
  {
    type: 'function' as const,
    name: 'queue',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'execute',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
    outputs: [],
    stateMutability: 'payable',
  },
  {
    type: 'function' as const,
    name: 'cancel',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // State
  {
    type: 'function' as const,
    name: 'state',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
    outputs: [{ name: '', type: 'uint8' }],
    stateMutability: 'view',
  },

  // Parameter updates
  {
    type: 'function' as const,
    name: 'setVotingDelay',
    inputs: [{ name: 'newVotingDelay', type: 'uint256' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'setVotingPeriod',
    inputs: [{ name: 'newVotingPeriod', type: 'uint256' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'setProposalThreshold',
    inputs: [{ name: 'newThreshold', type: 'uint256' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'setQuorumBps',
    inputs: [{ name: 'newQuorumBps', type: 'uint256' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'setTimelock',
    inputs: [{ name: 'newTimelock', type: 'uint256' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'setExecutionWindow',
    inputs: [{ name: 'newWindow', type: 'uint256' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // View helpers
  {
    type: 'function' as const,
    name: 'getProposal',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
    outputs: [
      { name: 'proposer_', type: 'address' },
      { name: 'voteStart', type: 'uint256' },
      { name: 'voteEnd', type: 'uint256' },
      { name: 'forVotes', type: 'uint256' },
      { name: 'againstVotes', type: 'uint256' },
      { name: 'abstainVotes', type: 'uint256' },
      { name: 'snapshotBlock_', type: 'uint256' },
      { name: 'executed_', type: 'bool' },
      { name: 'cancelled_', type: 'bool' },
      { name: 'queuedAt', type: 'uint256' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'hasVoted',
    inputs: [
      { name: 'proposalId', type: 'uint256' },
      { name: 'account', type: 'address' },
    ],
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getProposalActions',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
    outputs: [
      { name: 'targets', type: 'address[]' },
      { name: 'values', type: 'uint256[]' },
      { name: 'calldatas', type: 'bytes[]' },
    ],
    stateMutability: 'view',
  },

  // IGovernance events
  {
    type: 'event' as const,
    name: 'ProposalCreated',
    inputs: [
      { name: 'proposalId', type: 'uint256', indexed: true },
      { name: 'proposer', type: 'address', indexed: true },
      { name: 'targets', type: 'address[]', indexed: false },
      { name: 'values', type: 'uint256[]', indexed: false },
      { name: 'calldatas', type: 'bytes[]', indexed: false },
      { name: 'descriptionHash', type: 'bytes32', indexed: false },
      { name: 'voteStart', type: 'uint256', indexed: false },
      { name: 'voteEnd', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'VoteCast',
    inputs: [
      { name: 'proposalId', type: 'uint256', indexed: true },
      { name: 'voter', type: 'address', indexed: true },
      { name: 'support', type: 'uint8', indexed: false },
      { name: 'weight', type: 'uint256', indexed: false },
      { name: 'reason', type: 'string', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'ProposalQueued',
    inputs: [
      { name: 'proposalId', type: 'uint256', indexed: true },
      { name: 'eta', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'ProposalExecuted',
    inputs: [{ name: 'proposalId', type: 'uint256', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'ProposalCancelled',
    inputs: [{ name: 'proposalId', type: 'uint256', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'GovernanceParametersUpdated',
    inputs: [{ name: 'updater', type: 'address', indexed: true }],
  },

  // Errors
  {
    type: 'error' as const,
    name: 'ProposalNotFound',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
  },
  {
    type: 'error' as const,
    name: 'ProposalNotActive',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
  },
  {
    type: 'error' as const,
    name: 'AlreadyVoted',
    inputs: [
      { name: 'proposalId', type: 'uint256' },
      { name: 'voter', type: 'address' },
    ],
  },
  {
    type: 'error' as const,
    name: 'VotingPowerZero',
    inputs: [{ name: 'voter', type: 'address' }],
  },
  {
    type: 'error' as const,
    name: 'BelowProposalThreshold',
    inputs: [
      { name: 'proposer', type: 'address' },
      { name: 'threshold', type: 'uint256' },
      { name: 'actual', type: 'uint256' },
    ],
  },
  {
    type: 'error' as const,
    name: 'QuorumNotReached',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
  },
  {
    type: 'error' as const,
    name: 'ProposalNotSucceeded',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
  },
  {
    type: 'error' as const,
    name: 'ProposalNotQueued',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
  },
  {
    type: 'error' as const,
    name: 'ProposalNotReady',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
  },
  {
    type: 'error' as const,
    name: 'ProposalAlreadyExecuted',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
  },
  {
    type: 'error' as const,
    name: 'ProposalAlreadyCancelled',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
  },
  {
    type: 'error' as const,
    name: 'ProposalNotCancellable',
    inputs: [{ name: 'proposalId', type: 'uint256' }],
  },
  {
    type: 'error' as const,
    name: 'ArrayLengthMismatch',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'EmptyProposal',
    inputs: [],
  },
] as const;
