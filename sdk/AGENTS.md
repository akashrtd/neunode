# SDK — @neunode/sdk TypeScript Package

## OVERVIEW

TypeScript SDK for Neunode. Dual ESM+CJS build (tsup), strict mode, branded types, **16 resource
modules**, **4 transports**, 19 Solidity ABI bindings. Vitest across three tiers (unit / integration
/ e2e). Node `>= 20.11`; CI runs Node 22.

Two files in this package are **generated** and gated by CI — never hand-edit them:
`src/types/protocol.generated.ts` and `src/contracts/abi/*.ts`.

## STRUCTURE

```
sdk/
├── src/
│   ├── client/          # createNeunodeClient factory, NeunodeClient type
│   ├── transport/       # http (primary) · cli (subprocess) · viem (on-chain) · mock (in-memory)
│   ├── resources/       # 16 resources, each createXResource(client) + colocated *.test.ts
│   ├── types/           # 18 modules: branded types, enums, interfaces, protocol.generated.ts
│   ├── contracts/       # abi/ (19 generated files) + addresses.ts + contracts.ts + paymaster.ts
│   ├── build.test.ts    # Guards the built artifact shape
│   └── index.ts         # Barrel re-export
├── tests/
│   ├── e2e/             # Anvil-based, sequential, single fork (5 suites + helpers/)
│   └── integration/     # Against a real agnetd binary: CLI transport + HTTP routes
├── scripts/
│   ├── protocol-types.mjs  # Rust → TS codegen + drift check
│   └── contract-abis.mjs   # Solidity → TS ABI codegen + drift check
├── package.json         # viem >= 2.47 optional peer dep
├── tsconfig.json        # strict, ES2022, noUncheckedIndexedAccess
├── tsup.config.ts       # ESM + CJS + dts
├── typedoc.json         # Output → docs/ (gitignored)
└── vitest.config.ts     # Unit tier
```

There is **no** `src/utils/` and **no** committed `docs/api/` — both were removed. `docs/`, `dist/`,
`node_modules/`, and `coverage/` are gitignored.

There is **no** `biome.json` — Biome runs on defaults (tabs, double quotes). Match surrounding style.

## TRANSPORTS

`createNeunodeClient(config)` takes any combination of:

| Transport | Config key | Behavior |
|---|---|---|
| HTTP | `http` | **Primary.** REST against a running `agnetd serve`, base URL + optional API key, 30s default timeout |
| CLI | `cli` | Spawns `agnetd --output json-compact`; resolves `target/release/agnetd` → `target/debug/agnetd` |
| Viem | `viem` | Direct on-chain reads/writes. Optional peer dep `viem >= 2.47`, externalized from the bundle |
| Mock | `mock` | In-memory, HTTP-compatible. For development and tests |

Both HTTP and CLI use the same envelope:
`{ data: T, success: true } | { error: { code, message }, success: false }`.

## RESOURCES

All are `createXResource(client)` — they receive the **client**, not a transport, and pick their
transport per call.

- **HTTP-first with CLI fallback:** `bounty`, `config`, `discovery`, `feed`, `inference`,
  `knowledge`, `mesh`, `model`, `reputation`, `token`, `train`, `turboquant`
- **HTTP-only** (throws `"HTTP transport required for … operations"` without one): `identity`,
  `lifecycle`, `lineage`, `verification`

When adding a method, prefer the HTTP path and only add a CLI branch if the resource already has
one. Every HTTP path must have a matching route in `crates/agnetd/src/api_routes.rs`.

## CODEGEN — CI-GATED

| Artifact | Generate | Check | Requires |
|---|---|---|---|
| `src/types/protocol.generated.ts` | `npm run generate:protocol` | `npm run check:protocol` | cargo |
| `src/contracts/abi/*.ts` | `npm run generate:abi` | `npm run check:abi` | `contracts/out` (`forge build`) |

`generate:protocol` shells out to `cargo run -p neunode-core --example emit_sdk_protocol`. The check
variants diff against the committed file and fail the build on drift. If you change an exported Rust
core type or a Solidity ABI, regenerate and commit the result in the same change.

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add resource module | `src/resources/<name>.ts` + `<name>.test.ts` | Register in `client/client.ts` (import, field, `transportMode`) |
| Add type | `src/types/<name>.ts` | Register in `types/index.ts` barrel |
| Change HTTP behavior | `src/transport/http-transport.ts` | Envelope unwrapping, timeout, auth header |
| Change CLI behavior | `src/transport/cli-transport.ts` | Binary discovery, arg building, JSON parsing |
| Add contract helper | `src/contracts/contracts.ts` | `getXContract(address, client)` pattern |
| ERC-4337 sponsorship | `src/contracts/paymaster.ts` | Backed by `AgentPaymaster.sol` |
| Change chain addresses | `src/contracts/addresses.ts` | Mirror `contracts/script/Deploy.s.sol` |
| Write E2E test | `tests/e2e/<name>.e2e.ts` | Use `helpers/anvil.ts`, `deploy.ts`, `fixtures.ts` |
| Write integration test | `tests/integration/` | Runs against a built `agnetd` (`helpers/agnetd.ts`) |

## CONVENTIONS

- `strict: true`, `noUncheckedIndexedAccess`, ESM. No `as any`, `@ts-ignore`, `@ts-expect-error`.
- Branded ID types: `Did`, `CID`, `PeerId`, `BountyId` — never bare strings.
- Colocated unit tests as `*.test.ts`; integration/e2e live under `tests/` with their own vitest configs.
- `viem >= 2.47` is an **optional** peer dep — non-viem features must work without it installed.
- `@noble/ed25519 v3` for Ed25519 — **never tweetnacl** (malleability bug).
- JS transport is TCP + Noise only. QUIC and TLS 1.3 are not supported in a JS context.

## COMMANDS

```bash
npm run build              # tsup → ESM + CJS + dts
npm test                   # Unit (vitest)
npm run test:integration   # Needs a built agnetd (cargo build -p agnetd)
npm run test:e2e           # Needs anvil + forge build
npm run typecheck          # tsc --noEmit
npm run lint               # biome check src/
npm run typecheck:examples # Typechecks ../examples (CI enforced)
npm run lint:examples      # Lints ../examples (CI enforced)
npm run check:protocol     # Rust → TS drift gate
npm run check:abi          # Solidity → TS drift gate
npm run docs               # TypeDoc → docs/ (gitignored)
```

## NOTES

- The SDK CI job (`sdk` in `.github/workflows/ci.yml`) builds `agnetd` and runs `forge build` before
  anything else, then: build → typecheck → typecheck/lint examples → protocol drift → ABI drift →
  lint → unit → integration → e2e. `../examples/` is linted and typechecked by this job, so changes
  to the SDK's public surface can break CI from outside this directory.
- E2E tests require the `anvil` binary and run sequentially against a single shared fork. Known
  flaky: `bounty.e2e.ts > Review System > submitReview`.
- Integration tests spawn a real `agnetd`; RocksDB's single-process lock means they cannot run
  concurrently with another daemon on the same DB path.
- `examples/` and `packages/mcp-server/` default to `http://127.0.0.1:41000`, while `agnetd serve`
  defaults to port 8080. Set the port explicitly when wiring them together.
