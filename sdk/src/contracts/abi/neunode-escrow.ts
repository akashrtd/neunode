/**
 * NeunodeEscrow ABI — Bilateral escrow for bounty payments.
 * Source: contracts/src/NeunodeEscrow.sol
 * Inherited: AccessControl, IBountyEscrow
 */

export const neunodeEscrowAbi = [
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
    name: 'escrows',
    inputs: [{ name: '', type: 'bytes32' }],
    outputs: [
      { name: 'bountyId', type: 'bytes32' },
      { name: 'requester', type: 'address' },
      { name: 'provider', type: 'address' },
      { name: 'token', type: 'address' },
      { name: 'amount', type: 'uint256' },
      { name: 'providerBond', type: 'uint256' },
      { name: 'created', type: 'uint256' },
      { name: 'deadline', type: 'uint256' },
      { name: 'state', type: 'uint8' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'escrowBountyContracts',
    inputs: [{ name: '', type: 'bytes32' }],
    outputs: [{ name: '', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'PROVIDER_BOND_BPS',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'ESCROW_ADMIN_ROLE',
    inputs: [],
    outputs: [{ name: '', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'BOUNTY_CONTRACT_ROLE',
    inputs: [],
    outputs: [{ name: '', type: 'bytes32' }],
    stateMutability: 'view',
  },

  // Admin
  {
    type: 'function' as const,
    name: 'registerBountyContract',
    inputs: [{ name: 'bountyContract', type: 'address' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // IBountyEscrow
  {
    type: 'function' as const,
    name: 'createBountyEscrow',
    inputs: [
      { name: 'bountyId', type: 'bytes32' },
      { name: 'requester_', type: 'address' },
      { name: 'token', type: 'address' },
      { name: 'amount', type: 'uint256' },
      { name: 'workDeadline', type: 'uint256' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'bondProvider',
    inputs: [
      { name: 'bountyId', type: 'bytes32' },
      { name: 'provider_', type: 'address' },
      { name: 'bondAmount', type: 'uint256' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'releaseWithFees',
    inputs: [
      { name: 'bountyId', type: 'bytes32' },
      { name: 'provider_', type: 'address' },
      { name: 'protocolFeeBps', type: 'uint256' },
      { name: 'reviewerFeeBps', type: 'uint256' },
      { name: 'verificationFeeBps', type: 'uint256' },
      { name: 'protocolFeeRecipient', type: 'address' },
      { name: 'reviewerFeeRecipient', type: 'address' },
      { name: 'verificationFeeRecipient', type: 'address' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'refundRequester',
    inputs: [{ name: 'bountyId', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'isEscrowFunded',
    inputs: [{ name: 'bountyId', type: 'bytes32' }],
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'view',
  },

  // Direct escrow (backward-compatible)
  {
    type: 'function' as const,
    name: 'createEscrow',
    inputs: [
      { name: 'bountyId', type: 'bytes32' },
      { name: 'token', type: 'address' },
      { name: 'amount', type: 'uint256' },
      { name: 'deadline', type: 'uint256' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'fundEscrow',
    inputs: [
      { name: 'bountyId', type: 'bytes32' },
      { name: 'providerBond', type: 'uint256' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'release',
    inputs: [{ name: 'bountyId', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'refund',
    inputs: [{ name: 'bountyId', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'dispute',
    inputs: [{ name: 'bountyId', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'getEscrowState',
    inputs: [{ name: 'bountyId', type: 'bytes32' }],
    outputs: [{ name: '', type: 'uint8' }],
    stateMutability: 'view',
  },

  // Events
  {
    type: 'event' as const,
    name: 'EscrowCreated',
    inputs: [
      { name: 'bountyId', type: 'bytes32', indexed: true },
      { name: 'requester', type: 'address', indexed: true },
      { name: 'token', type: 'address', indexed: false },
      { name: 'amount', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'EscrowFunded',
    inputs: [
      { name: 'bountyId', type: 'bytes32', indexed: true },
      { name: 'provider', type: 'address', indexed: true },
      { name: 'bond', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'EscrowReleased',
    inputs: [
      { name: 'bountyId', type: 'bytes32', indexed: true },
      { name: 'provider', type: 'address', indexed: true },
      { name: 'amount', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'EscrowRefunded',
    inputs: [
      { name: 'bountyId', type: 'bytes32', indexed: true },
      { name: 'requester', type: 'address', indexed: true },
      { name: 'amount', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'EscrowDisputed',
    inputs: [
      { name: 'bountyId', type: 'bytes32', indexed: true },
      { name: 'timestamp', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'EscrowReleasedWithFees',
    inputs: [
      { name: 'bountyId', type: 'bytes32', indexed: true },
      { name: 'provider', type: 'address', indexed: true },
      { name: 'providerPayout', type: 'uint256', indexed: false },
      { name: 'protocolFee', type: 'uint256', indexed: false },
      { name: 'reviewerFee', type: 'uint256', indexed: false },
      { name: 'verificationFee', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'BountyContractRegistered',
    inputs: [{ name: 'bountyContract', type: 'address', indexed: true }],
  },

  // Errors
  {
    type: 'error' as const,
    name: 'EscrowNotFound',
    inputs: [{ name: 'bountyId', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'EscrowNotCreated',
    inputs: [{ name: 'bountyId', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'EscrowNotFunded',
    inputs: [{ name: 'bountyId', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'NotRequester',
    inputs: [
      { name: 'bountyId', type: 'bytes32' },
      { name: 'caller', type: 'address' },
    ],
  },
  {
    type: 'error' as const,
    name: 'NotProvider',
    inputs: [
      { name: 'bountyId', type: 'bytes32' },
      { name: 'caller', type: 'address' },
    ],
  },
  {
    type: 'error' as const,
    name: 'InvalidAmount',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'InvalidToken',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'DeadlinePassed',
    inputs: [{ name: 'deadline', type: 'uint256' }],
  },
  {
    type: 'error' as const,
    name: 'TransferFailed',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'Unauthorized',
    inputs: [],
  },
] as const;
