/**
 * ModelRegistry ABI — Model lineage DAG with content-addressed models.
 * Source: contracts/src/royalty/ModelRegistry.sol
 * Inherited: AccessControl, IModelRegistry
 */

export const modelRegistryAbi = [
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

  {
    type: 'function' as const,
    name: 'REGISTRAR_ROLE',
    inputs: [],
    outputs: [{ name: '', type: 'bytes32' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'registerModel',
    inputs: [
      { name: 'cid', type: 'bytes32' },
      { name: 'parentCids', type: 'bytes32[]' },
      { name: 'contribution', type: 'uint8' },
      { name: 'metadataURI', type: 'string' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'function' as const,
    name: 'getModel',
    inputs: [{ name: 'cid', type: 'bytes32' }],
    outputs: [
      {
        name: '',
        type: 'tuple',
        components: [
          { name: 'cid', type: 'bytes32' },
          { name: 'contributor', type: 'address' },
          { name: 'contribution', type: 'uint8' },
          { name: 'metadataURI', type: 'string' },
          { name: 'registeredAt', type: 'uint256' },
          { name: 'exists', type: 'bool' },
        ],
      },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getParents',
    inputs: [{ name: 'cid', type: 'bytes32' }],
    outputs: [{ name: '', type: 'bytes32[]' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getChildren',
    inputs: [{ name: 'cid', type: 'bytes32' }],
    outputs: [{ name: '', type: 'bytes32[]' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getLineageDepth',
    inputs: [{ name: 'cid', type: 'bytes32' }],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'modelExists',
    inputs: [{ name: 'cid', type: 'bytes32' }],
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'getModelCount',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
  },

  // IModelRegistry events
  {
    type: 'event' as const,
    name: 'ModelRegistered',
    inputs: [
      { name: 'cid', type: 'bytes32', indexed: true },
      { name: 'contributor', type: 'address', indexed: true },
      { name: 'contribution', type: 'uint8', indexed: false },
      { name: 'parentCids', type: 'bytes32[]', indexed: false },
    ],
  },
  {
    type: 'event' as const,
    name: 'LineageExtended',
    inputs: [
      { name: 'parentCid', type: 'bytes32', indexed: true },
      { name: 'childCid', type: 'bytes32', indexed: true },
      { name: 'contributor', type: 'address', indexed: true },
    ],
  },

  // Errors
  {
    type: 'error' as const,
    name: 'ModelNotFound',
    inputs: [{ name: 'cid', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'ModelAlreadyExists',
    inputs: [{ name: 'cid', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'InvalidCid',
    inputs: [{ name: 'cid', type: 'bytes32' }],
  },
  {
    type: 'error' as const,
    name: 'ParentNotFound',
    inputs: [{ name: 'cid', type: 'bytes32' }],
  },
] as const;
