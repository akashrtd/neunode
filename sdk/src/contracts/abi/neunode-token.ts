/**
 * NeunodeToken ABI — Base ERC-20 for resource-backed tokens
 * Source: contracts/src/tokens/NeunodeToken.sol
 *
 * Inherited: ERC20 (name, symbol, totalSupply, balanceOf, transfer, allowance, approve, transferFrom),
 *            Ownable (owner, transferOwnership, renounceOwnership),
 *            AccessControl (hasRole, getRoleAdmin, grantRole, revokeRole, renounceRole)
 *            INeunodeToken (custom staking/decay/seed)
 */

// ─── ERC-20 core ────────────────────────────────────────────────────────────

export const neunodeTokenAbi = [
  // ERC-20
  {
    type: 'function' as const,
    name: 'name',
    inputs: [],
    outputs: [{ name: '', type: 'string' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'symbol',
    inputs: [],
    outputs: [{ name: '', type: 'string' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'decimals',
    inputs: [],
    outputs: [{ name: '', type: 'uint8' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'totalSupply',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'balanceOf',
    inputs: [{ name: 'account', type: 'address' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'transfer',
    inputs: [
      { name: 'to', type: 'address' },
      { name: 'amount', type: 'uint256' },
    ],
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'allowance',
    inputs: [
      { name: 'owner', type: 'address' },
      { name: 'spender', type: 'address' },
    ],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'approve',
    inputs: [
      { name: 'spender', type: 'address' },
      { name: 'amount', type: 'uint256' },
    ],
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'transferFrom',
    inputs: [
      { name: 'from', type: 'address' },
      { name: 'to', type: 'address' },
      { name: 'amount', type: 'uint256' },
    ],
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'nonpayable',
  },

  // ERC-20 events
  {
    type: 'event' as const,
    name: 'Transfer',
    inputs: [
      { name: 'from', type: 'address', indexed: true },
      { name: 'to', type: 'address', indexed: true },
      { name: 'value', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'Approval',
    inputs: [
      { name: 'owner', type: 'address', indexed: true },
      { name: 'spender', type: 'address', indexed: true },
      { name: 'value', type: 'uint256', indexed: false },
    ],
  },

  // ─── Ownable ──────────────────────────────────────────────────────────────

  {
    type: 'function' as const,
    name: 'owner',
    inputs: [],
    outputs: [{ name: '', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'transferOwnership',
    inputs: [{ name: 'newOwner', type: 'address' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'renounceOwnership',
    inputs: [],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // Ownable events
  {
    type: 'event' as const,
    name: 'OwnershipTransferred',
    inputs: [
      { name: 'previousOwner', type: 'address', indexed: true },
      { name: 'newOwner', type: 'address', indexed: true },
    ],
  },

  // ─── AccessControl ────────────────────────────────────────────────────────

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

  // AccessControl events
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

  // ─── NeunodeToken custom roles ────────────────────────────────────────────

  {
    type: 'function' as const,
    name: 'MINTER_ROLE',
    inputs: [],
    outputs: [{ name: '', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'BURNER_ROLE',
    inputs: [],
    outputs: [{ name: '', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'GOVERNANCE_ROLE',
    inputs: [],
    outputs: [{ name: '', type: 'bytes32' }],
    stateMutability: 'view',
  },

  // ─── Mint / Burn ──────────────────────────────────────────────────────────

  {
    type: 'function' as const,
    name: 'mint',
    inputs: [
      { name: 'to', type: 'address' },
      { name: 'amount', type: 'uint256' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'burn',
    inputs: [
      { name: 'from', type: 'address' },
      { name: 'amount', type: 'uint256' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // ─── Staking ──────────────────────────────────────────────────────────────

  {
    type: 'function' as const,
    name: 'stake',
    inputs: [{ name: 'amount', type: 'uint256' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'unstake',
    inputs: [{ name: 'amount', type: 'uint256' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'stakedBalanceOf',
    inputs: [{ name: 'account', type: 'address' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },

  // ─── Activity Tracking ────────────────────────────────────────────────────

  {
    type: 'function' as const,
    name: 'updateActivity',
    inputs: [{ name: 'account', type: 'address' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'lastActivity',
    inputs: [{ name: 'account', type: 'address' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getActivityLevel',
    inputs: [{ name: 'account', type: 'address' }],
    outputs: [{ name: '', type: 'uint8' }],
    stateMutability: 'view',
  },

  // ─── Decay ────────────────────────────────────────────────────────────────

  {
    type: 'function' as const,
    name: 'decayConfig',
    inputs: [],
    outputs: [
      { name: 'treasury', type: 'address' },
      { name: 'stakingRewards', type: 'address' },
      { name: 'devFund', type: 'address' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'decayRatesBps',
    inputs: [{ name: '', type: 'uint256' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'setDecayConfig',
    inputs: [
      { name: 'treasury', type: 'address' },
      { name: 'stakingRewards', type: 'address' },
      { name: 'devFund', type: 'address' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'computeDecay',
    inputs: [{ name: 'account', type: 'address' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'executeDecay',
    inputs: [{ name: 'account', type: 'address' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },

  // ─── Seed Tokens ──────────────────────────────────────────────────────────

  {
    type: 'function' as const,
    name: 'mintSeed',
    inputs: [
      { name: 'to', type: 'address' },
      { name: 'amount', type: 'uint256' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'activateSeed',
    inputs: [{ name: 'account', type: 'address' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'seedBalanceOf',
    inputs: [{ name: 'account', type: 'address' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },

  // ─── INeunodeToken events ─────────────────────────────────────────────────

  {
    type: 'event' as const,
    name: 'Staked',
    inputs: [
      { name: 'account', type: 'address', indexed: true },
      { name: 'amount', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'Unstaked',
    inputs: [
      { name: 'account', type: 'address', indexed: true },
      { name: 'amount', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'DecayExecuted',
    inputs: [
      { name: 'account', type: 'address', indexed: true },
      { name: 'amount', type: 'uint256', indexed: false },
      { name: 'treasuryPortion', type: 'uint256', indexed: false },
      { name: 'stakingPortion', type: 'uint256', indexed: false },
      { name: 'burnPortion', type: 'uint256', indexed: false },
      { name: 'devPortion', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'SeedMinted',
    inputs: [
      { name: 'to', type: 'address', indexed: true },
      { name: 'amount', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'SeedActivated',
    inputs: [{ name: 'account', type: 'address', indexed: true }],
  },
  {
    type: 'event' as const,
    name: 'ActivityUpdated',
    inputs: [
      { name: 'account', type: 'address', indexed: true },
      { name: 'timestamp', type: 'uint256', indexed: false },
    ],
  },

  // ─── Ownable errors (used by onlyOwnerOrRole modifier) ────────────────────

  {
    type: 'error' as const,
    name: 'OwnableUnauthorizedAccount',
    inputs: [{ name: 'account', type: 'address' }],
  },
  {
    type: 'error' as const,
    name: 'OwnableInvalidOwner',
    inputs: [{ name: 'owner', type: 'address' }],
  },
] as const;
