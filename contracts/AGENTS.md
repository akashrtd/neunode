# CONTRACTS — Solidity / Foundry

## OVERVIEW

EVM smart contracts for Neunode. Foundry project, **Solidity 0.8.28**, EIP-2535 Diamond proxy.
31 source contracts, 17 test files (~395 test functions).

These contracts are the **on-chain migration target**, not the current source of truth — the
off-chain Rust ledger is canonical today (`docs/adr/0001-canonical-ledger-source-of-truth.md`).
There is no live chain; everything runs against Anvil.

## STRUCTURE

```
contracts/
├── src/
│   ├── NeunodeBounty.sol      # Core bounty contract (largest)
│   ├── NeunodeEscrow.sol      # Escrow for bounty payments
│   ├── NeunodeIdentity.sol    # Agent identity registry
│   ├── NeunodeRegistry.sol    # General-purpose registry
│   ├── account/               # AgentPaymaster (ERC-4337 gas sponsorship), IEntryPoint
│   ├── bounty/                # BountyReview, IBountyEscrow, IBountyReview
│   ├── diamond/               # EIP-2535: Diamond, CutFacet, LoupeFacet, LibDiamond, interfaces
│   ├── escrow/                # StakingEscrow
│   ├── exchange/              # ResourceAMM (constant-product, treasury-seeded)
│   ├── governance/            # NeunodeGovernance, IGovernance
│   ├── reputation/            # NeunodeReputation
│   ├── royalty/               # ModelRegistry, RoyaltySplitter + interfaces
│   ├── slashing/              # NeunodeSlashing
│   ├── tokens/                # NeunodeToken base + Compute/Training/Bandwidth/Storage
│   └── interfaces/            # INeunodeToken
├── test/                      # Mirrors src/ + top-level contract tests + DeploymentTopology.t.sol
├── script/                    # Deploy.s.sol + deploy-testnet.sh
├── lib/                       # VENDORED: forge-std + openzeppelin-contracts — DO NOT MODIFY
├── foundry.toml               # solc 0.8.28, optimizer 200, via_ir, fuzz runs 256, fmt line 100
├── remappings.txt             # @openzeppelin/ → lib/openzeppelin-contracts/, forge-std/ → lib/forge-std/src/
└── foundry.lock
```

Gas snapshots live in the repo root as `.gas-snapshot`.

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add token contract | `src/tokens/` | Inherit `NeunodeToken` (decay + staking) or OZ ERC-20 |
| Change governance | `src/governance/NeunodeGovernance.sol` | Diamond facet |
| Modify bounty logic | `src/NeunodeBounty.sol` | Commit-reveal *claim* scheme; see `test/bounty/CommitReveal.t.sol` |
| Change diamond facets | `src/diamond/` | Storage in `LibDiamond.sol`; Cut/Loupe facets |
| Add royalty rule | `src/royalty/ModelRegistry.sol` | ERC-2981 + `RoyaltySplitter` |
| Change slashing | `src/slashing/NeunodeSlashing.sol` | Paired with `escrow/StakingEscrow.sol` |
| Change AMM math | `src/exchange/ResourceAMM.sol` | Constant product; treasury-seeded liquidity |
| Change gas sponsorship | `src/account/AgentPaymaster.sol` | ERC-4337; SDK side in `sdk/src/contracts/paymaster.ts` |
| Write contract test | `test/<domain>/` | Mirror the `src/` directory structure |
| Change deploy topology | `script/Deploy.s.sol` | Guarded by `test/DeploymentTopology.t.sol` |
| Deploy to testnet | `script/deploy-testnet.sh --chain holesky` | Reads `.env` |

## CONVENTIONS

- Compiled with solc **0.8.28** (`foundry.toml`). Source pragmas are mostly `^0.8.24` (29 files) with
  2 at `^0.8.28` — the floor is historical; the compiler is 0.8.28. New files should use `^0.8.28`.
- `via_ir = true`, optimizer 200 runs, fuzz 256 runs.
- Custom errors, not revert strings. NatSpec on all public/external functions.
- `forge fmt`: line length 100, 4-space indent.
- Upgradeable via Diamond proxy — new facets go in `src/<domain>/` and get registered in the cut.
- OpenZeppelin for tokens, governance, access control (imported from `lib/` via `remappings.txt`).
- Gas snapshots enforced in CI (`forge snapshot --check`) — regenerate with `forge snapshot` and
  commit `.gas-snapshot` whenever gas legitimately changes.

## FOUNDRY VERSION — PINNED

CI pins Foundry to **v1.5.1**. Unpinned `stable` drifted fuzz-test μ/median gas and flaked
`forge snapshot --check`. Use the same version locally or your snapshot diffs will be noise:

```bash
foundryup --version v1.5.1
```

## COMMANDS

```bash
forge build --sizes                 # Build with size report
forge test -vvv                     # All tests
forge test --match-test testBounty  # Single test
forge fmt --check                   # CI enforced
forge snapshot --check              # Gas regression gate (CI enforced)
forge snapshot                      # Regenerate ../.gas-snapshot
```

`contracts-ci.yml` triggers only on `contracts/**` changes and checks out submodules recursively.

## NOTES

- `lib/` is vendored (forge-std, openzeppelin-contracts) — `git submodule update --init --recursive`
  after a fresh clone. Never modify it.
- `via_ir = true` slows compilation but is required for the Diamond contracts to fit.
- Deploy script supports `--chain anvil | holesky | mainnet` and auto-verifies on Etherscan.
  `foundry.toml` reads `RPC_URL_HOLESKY` / `RPC_URL_MAINNET` from the environment.
- **ABI changes propagate to TypeScript.** After changing any contract:
  `forge build && cd ../sdk && npm run generate:abi` — CI runs `npm run check:abi` and fails on
  drift. `contracts/out/` is gitignored, so the SDK job rebuilds contracts before checking.
- Deployed addresses are mirrored in `sdk/src/contracts/addresses.ts`; keep it in step with
  `script/Deploy.s.sol`.
- Bounty FSM (mirrors the Rust implementation): Open → Claimed → Submitted → UnderReview → Revision
  → Accepted/Rejected/Disputed → Paid/Expired/Cancelled.
- Escrow pattern: full deposit upfront, then block-by-block or milestone payout.
