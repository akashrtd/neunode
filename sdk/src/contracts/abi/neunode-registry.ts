/**
 * NeunodeRegistry ABI — Agent capability and endpoint registry.
 * Source: contracts/src/NeunodeRegistry.sol
 */

export const neunodeRegistryAbi = [
  {
    type: 'function' as const,
    name: 'identity',
    inputs: [],
    outputs: [{ name: '', type: 'address' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'agents',
    inputs: [{ name: '', type: 'bytes32' }],
    outputs: [
      { name: 'didHash', type: 'bytes32' },
      { name: 'capabilities', type: 'string' },
      { name: 'endpoint', type: 'string' },
      { name: 'stakeAmount', type: 'uint256' },
      { name: 'registeredAt', type: 'uint256' },
      { name: 'updatedAt', type: 'uint256' },
      { name: 'active', type: 'bool' },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'agentList',
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
    name: 'register',
    inputs: [
      { name: 'didHash', type: 'bytes32' },
      { name: 'capabilities', type: 'string' },
      { name: 'endpoint', type: 'string' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'update',
    inputs: [
      { name: 'didHash', type: 'bytes32' },
      { name: 'capabilities', type: 'string' },
      { name: 'endpoint', type: 'string' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'deregister',
    inputs: [{ name: 'didHash', type: 'bytes32' }],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'getAgent',
    inputs: [{ name: 'didHash', type: 'bytes32' }],
    outputs: [
      {
        name: '',
        type: 'tuple',
        components: [
          { name: 'didHash', type: 'bytes32' },
          { name: 'capabilities', type: 'string' },
          { name: 'endpoint', type: 'string' },
          { name: 'stakeAmount', type: 'uint256' },
          { name: 'registeredAt', type: 'uint256' },
          { name: 'updatedAt', type: 'uint256' },
          { name: 'active', type: 'bool' },
        ],
      },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getActiveAgents',
    inputs: [],
    outputs: [{ name: '', type: 'bytes32[]' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getTotalAgents',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },

  // Events
  {
    type: 'event' as const,
    name: 'AgentRegistered',
    inputs: [
      { name: 'didHash', type: 'bytes32', indexed: true },
      { name: 'controller', type: 'address', indexed: true },
      { name: 'timestamp', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'AgentUpdated',
    inputs: [
      { name: 'didHash', type: 'bytes32', indexed: true },
      { name: 'controller', type: 'address', indexed: true },
      { name: 'timestamp', type: 'uint256', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'AgentDeregistered',
    inputs: [
      { name: 'didHash', type: 'bytes32', indexed: true },
      { name: 'controller', type: 'address', indexed: true },
      { name: 'timestamp', type: 'uint256', indexed: false },
    ],
  },

  // Errors
  {
    type: 'error' as const,
    name: 'NotDidController',
    inputs: [
      { name: 'didHash', type: 'bytes32' },
      { name: 'caller', type: 'address' },
    ],
  },
  {
    type: 'error' as const,
    name: 'DidNotActive',
    inputs: [{ name: 'didHash', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'AgentNotFound',
    inputs: [{ name: 'didHash', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'AgentAlreadyRegistered',
    inputs: [{ name: 'didHash', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'AgentNotActive',
    inputs: [{ name: 'didHash', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'EmptyCapabilities',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'EmptyEndpoint',
    inputs: [],
  },
] as const;
