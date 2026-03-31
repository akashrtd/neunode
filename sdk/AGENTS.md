# SDK — @neunode/sdk TypeScript Package

## OVERVIEW

TypeScript SDK for Neunode. Dual ESM+CJS build (tsup), strict mode, branded types, 10 resource modules, 14 Solidity ABI bindings. Vitest for testing (3 tiers: unit/integration/e2e).

## STRUCTURE

```
sdk/
├── src/
│   ├── client/          # createNeunodeClient factory, NeunodeClient type
│   ├── transport/       # CliTransport (subprocess) + ViemTransport (direct chain)
│   ├── resources/       # 10 resources: identity, config, feed, mesh, model, train, bounty, token, reputation, inference
│   ├── types/           # Branded types (Did, CID, PeerId), enums, interfaces
│   ├── contracts/       # 14 ABI files + addresses + getContract helpers
│   │   └── abi/         # Auto-generated from Solidity — update when contracts change
│   ├── utils/           # (empty — planned)
│   └── index.ts         # Barrel re-export
├── tests/
│   ├── e2e/             # Anvil-based, sequential, 60s timeout (5 suites, 70 tests)
│   │   └── helpers/     # anvil.ts, deploy.ts, fixtures.ts
│   └── integration/     # CLI subprocess transport tests (1 suite, 16 tests)
├── package.json         # viem>=2.47 optional peer dep
├── tsconfig.json        # strict, ES2022, noUncheckedIndexedAccess
├── tsup.config.ts       # ESM+CJS+dts build
└── docs/api/            # TypeDoc generated — DO NOT edit
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add resource module | `src/resources/<name>.ts` + `<name>.test.ts` | Factory: `createXResource(transport)` |
| Add type | `src/types/<name>.ts` | Register in `types/index.ts` barrel |
| Add ABI binding | `src/contracts/abi/<name>.ts` | Export from `contracts/index.ts` |
| Change transport | `src/transport/cli-transport.ts` or `viem-transport.ts` | Two modes: CLI subprocess or Viem direct |
| Add contract helper | `src/contracts/contracts.ts` | `getXContract(address, client)` pattern |
| Change chain addresses | `src/contracts/addresses.ts` | Per-chain contract addresses |
| Write E2E test | `tests/e2e/<name>.e2e.ts` | Use helpers/ for Anvil lifecycle |
| Write integration test | `tests/integration/` | Tests against real `agnetd` binary |

## CONVENTIONS

- `strict: true`, `noUncheckedIndexedAccess`, ESM modules. No `as any`, no `@ts-ignore`.
- Branded types for IDs: `Did`, `CID`, `PeerId`, `BountyId` — never bare strings.
- Resource factory pattern: each resource is `createXResource(transport)` → `{ get, list, create, ... }`.
- Colocated tests: `*.test.ts` alongside source. E2E/integration in `tests/`.
- `viem>=2.47` optional peer dep — SDK works without it (CLI transport fallback).
- Build: `tsup` produces ESM (95KB) + CJS (100KB) + DTS (231KB). `viem` externalized.
- `@noble/ed25519 v3` for Ed25519 — **NEVER tweetnacl**.

## NOTES

- No SDK CI workflow — only Rust and Solidity are in CI. Run `npm test` + `npm run typecheck` locally.
- E2E tests require `anvil` binary installed. Sequential execution (single-fork, shared Anvil state).
- 1 known flaky test: `bounty.e2e.ts > Review System > submitReview` (Anvil snapshot isolation issue).
- `docs/api/` is 198 auto-generated TypeDoc files — never edit manually, regenerate with `npm run docs`.
- `utils/` directory exists but is empty.
- CLI transport spawns `agnetd` binary — discovers `target/release/agnetd` → `target/debug/agnetd` → null.
