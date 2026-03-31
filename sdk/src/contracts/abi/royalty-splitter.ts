/**
 * RoyaltySplitter ABI — ERC-2981 royalty distribution with BFS lineage traversal.
 * Source: contracts/src/royalty/RoyaltySplitter.sol
 * Inherited: AccessControl, IERC2981, IRoyaltySplitter
 */

export const royaltySplitterAbi = [
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
    name: 'registry',
    inputs: [],
    outputs: [{ name: '', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'protocolRoyaltyBps',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'defaultReceiver',
    inputs: [],
    outputs: [{ name: '', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'contributionTypeWeights',
    inputs: [{ name: '', type: 'uint256' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'accumulatedRoyalties',
    inputs: [
      { name: '', type: 'bytes32' },
      { name: '', type: 'address' },
    ],
    outputs: [{ name: '', type: 'uint256' }],
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
    name: 'DECAY_NUMERATOR',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'DECAY_DENOMINATOR',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'DEFAULT_SHAPLEY_SCORE',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },

  // ERC-165
  {
    type: 'function' as const,
    name: 'supportsInterface',
    inputs: [{ name: 'interfaceId', type: 'bytes4' }],
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'view',
  },

  // Admin
  {
    type: 'function' as const,
    name: 'setProtocolRoyaltyBps',
    inputs: [{ name: 'newBps', type: 'uint256' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'setDefaultReceiver',
    inputs: [{ name: 'receiver', type: 'address' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // ERC-2981
  {
    type: 'function' as const,
    name: 'royaltyInfo',
    inputs: [
      { name: 'tokenId', type: 'uint256' },
      { name: 'salePrice', type: 'uint256' },
    ],
    outputs: [
      { name: 'receiver', type: 'address' },
      { name: 'royaltyAmount', type: 'uint256' },
    ],
    stateMutability: 'view',
  },

  // Distribution
  {
    type: 'function' as const,
    name: 'distributeRoyalties',
    inputs: [
      { name: 'modelCid', type: 'bytes32' },
      { name: 'amount', type: 'uint256' },
      { name: 'token', type: 'address' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // View
  {
    type: 'function' as const,
    name: 'getRecipients',
    inputs: [{ name: 'modelCid', type: 'bytes32' }],
    outputs: [{ name: '', type: 'tuple[]' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getContributionTypeWeight',
    inputs: [{ name: 'contributionType', type: 'uint8' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'pure',
  },

  // IRoyaltySplitter events
  {
    type: 'event' as const,
    name: 'RoyaltyDistributed',
    inputs: [
      { name: 'modelCid', type: 'bytes32', indexed: true },
      { name: 'token', type: 'address', indexed: true },
      { name: 'totalAmount', type: 'uint256', indexed: false },
      { name: 'recipientCount', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'RecipientPaid',
    inputs: [
      { name: 'modelCid', type: 'bytes32', indexed: true },
      { name: 'recipient', type: 'address', indexed: true },
      { name: 'amount', type: 'uint256', indexed: false },
      { name: 'depth', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'ProtocolRoyaltyBpsUpdated',
    inputs: [
      { name: 'oldBps', type: 'uint256', indexed: false },
      { name: 'newBps', type: 'uint256', indexed: false },
    ],
  },

  // IRoyaltySplitter errors
  {
    type: 'error' as const,
    name: 'NoLineage',
    inputs: [{ name: 'cid', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'ZeroAmount',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'DistributionFailed',
    inputs: [
      { name: 'recipient', type: 'address' },
      { name: 'amount', type: 'uint256' },
    ],
  },
  {
    type: 'error' as const,
    name: 'ModelNotFound',
    inputs: [{ name: 'cid', type: 'bytes32' }],
  },
] as const;
