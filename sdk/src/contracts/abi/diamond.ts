/**
 * Diamond ABIs — EIP-2535 Diamond proxy, Cut facet, and Loupe facet.
 * Source: contracts/src/diamond/Diamond.sol, DiamondCutFacet.sol, DiamondLoupeFacet.sol
 * Also includes LibDiamond errors.
 */

export const diamondCutFacetAbi = [
  // IDiamondCut
  {
    type: 'function' as const,
    name: 'diamondCut',
    inputs: [
      {
        name: '_diamondCut',
        type: 'tuple[]',
        components: [
          { name: 'facetAddress', type: 'address' },
          { name: 'action', type: 'uint8' },
          { name: 'functionSelectors', type: 'bytes4[]' },
        ],
      },
      { name: '_init', type: 'address' },
      { name: '_calldata', type: 'bytes' },
    ],
    outputs: [],
    stateMutability: 'nonpayable',
  },
  {
    type: 'event' as const,
    name: 'DiamondCut',
    inputs: [
      {
        name: '_diamondCut',
        type: 'tuple[]',
        components: [
          { name: 'facetAddress', type: 'address' },
          { name: 'action', type: 'uint8' },
          { name: 'functionSelectors', type: 'bytes4[]' },
        ],
        indexed: false,
      },
      { name: '_init', type: 'address', indexed: false },
      { name: '_calldata', type: 'bytes', indexed: false },
    ],
  },
] as const;

export const diamondLoupeFacetAbi = [
  // IDiamondLoupe
  {
    type: 'function' as const,
    name: 'facets',
    inputs: [],
    outputs: [
      {
        name: '',
        type: 'tuple[]',
        components: [
          { name: 'facetAddress', type: 'address' },
          { name: 'functionSelectors', type: 'bytes4[]' },
        ],
      },
    ],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'facetFunctionSelectors',
    inputs: [{ name: '_facet', type: 'address' }],
    outputs: [{ name: '', type: 'bytes4[]' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'facetAddresses',
    inputs: [],
    outputs: [{ name: '', type: 'address[]' }],
    stateMutability: 'view',
  },
  {
    type: 'function' as const,
    name: 'facetAddress',
    inputs: [{ name: '_functionSelector', type: 'bytes4' }],
    outputs: [{ name: '', type: 'address' }],
    stateMutability: 'view',
  },
] as const;

export const diamondAbi = [...diamondCutFacetAbi, ...diamondLoupeFacetAbi] as const;

export const libDiamondErrors = [
  {
    type: 'error' as const,
    name: 'NotContractOwner',
    inputs: [
      { name: 'caller', type: 'address' },
      { name: 'owner', type: 'address' },
    ],
  },
  {
    type: 'error' as const,
    name: 'NoSelectorsProvided',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'FacetAddressZeroForAdd',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'FacetAddressNotZeroForRemove',
    inputs: [],
  },
  {
    type: 'error' as const,
    name: 'SelectorAlreadyExists',
    inputs: [{ name: 'selector', type: 'bytes4' }],
  },
  {
    type: 'error' as const,
    name: 'SelectorNotFound',
    inputs: [{ name: 'selector', type: 'bytes4' }],
  },
  {
    type: 'error' as const,
    name: 'SameFacetForReplace',
    inputs: [{ name: 'selector', type: 'bytes4' }],
  },
  {
    type: 'error' as const,
    name: 'InitReverted',
    inputs: [],
  },
] as const;
