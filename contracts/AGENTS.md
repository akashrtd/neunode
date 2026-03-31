# CONTRACTS — Solidity / Foundry

## OVERVIEW

EVM smart contracts for Neunode. Foundry project, Solidity 0.8.24, EIP-2535 Diamond proxy pattern. 25 source contracts, 10 test files.

## STRUCTURE

```
contracts/
├── src/
│   ├── NeunodeBounty.sol      # Core bounty contract (724 lines)
│   ├── NeunodeEscrow.sol      # Escrow for bounty payments
│   ├── NeunodeIdentity.sol    # Agent identity registry
│   ├── NeunodeRegistry.sol    # General-purpose registry
│   ├── bounty/                # BountyReview, IBountyEscrow, IBountyReview
│   ├── diamond/               # EIP-2535: Diamond, Cut, Loupe, LibDiamond (6 files)
│   ├── governance/            # NeunodeGovernance, IGovernance (2 files)
│   ├── royalty/               # ModelRegistry, RoyaltySplitter + interfaces (4 files)
│   ├── tokens/                # nCompute, nTrain, nBandwidth, nStorage, NeunodeToken (5 files)
│   └── interfaces/            # INeunodeToken (1 file)
├── test/                      # Mirror of src/ structure + top-level contract tests
├── script/                    # Deploy.s.sol + deploy-testnet.sh
├── lib/                       # VENDORED: forge-std + openzeppelin-contracts — DO NOT MODIFY
├── foundry.toml               # solc 0.8.24, optimizer 200 runs, via_ir, fmt line_length=100
└── .env.example               # PRIVATE_KEY, RPC_URLs, ETHERSCAN_API_KEY
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add token contract | `src/tokens/` | Inherit from NeunodeToken or OpenZeppelin ERC-20 |
| Change governance | `src/governance/NeunodeGovernance.sol` | Diamond facet |
| Modify bounty logic | `src/NeunodeBounty.sol` | Largest contract (724 lines) |
| Change diamond facets | `src/diamond/` | LibDiamond.sol for storage, Cut/Loupe facets |
| Add royalty rule | `src/royalty/ModelRegistry.sol` | ERC-2981 + splitter |
| Write contract test | `test/<domain>/` | Mirror src/ directory structure |
| Deploy to testnet | `script/deploy-testnet.sh --chain holesky` | Uses .env for keys |

## CONVENTIONS

- Solidity 0.8.24, `via_ir = true`, optimizer 200 runs.
- Custom errors (not revert strings). NatSpec on all public/external functions.
- Line length 100 (`forge fmt`). 4-space indent for `.sol`.
- Upgradeable via Diamond proxy — new facets go in `src/<domain>/`, register cut in deployment.
- OpenZeppelin for tokens, governance, access control (imported from `lib/`).
- Gas snapshots enforced in CI (`forge snapshot --check`).

## NOTES

- `lib/` is vendored dependencies (forge-std, openzeppelin-contracts) — `git submodule update --init --recursive` on clone.
- `via_ir = true` increases compile time but optimizes complex contracts (necessary for Diamond).
- Deploy script supports `--chain` flag: `anvil`, `holesky`, `mainnet`. Auto-verifies on Etherscan.
- Bounty FSM states: Open→Claimed→Submitted→UnderReview→Revision→Accepted/Rejected/Disputed→Paid/Expired/Cancelled.
- Escrow pattern: full deposit upfront, block-by-block or milestone-based payout.
- ABI bindings in `sdk/src/contracts/abi/` must be updated when contracts change.
