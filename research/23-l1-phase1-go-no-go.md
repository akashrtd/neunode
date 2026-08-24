# Neunode L1 Phase 1 — Reth + Malachite GO/NO-GO

**Decision date:** 2026-08-24  
**Decision:** GO, with the upstream channel interface pinned until a controlled upgrade is tested.

## Evidence

The reference implementation is Circle's Apache-2.0
[`malaketh-layered`](https://github.com/circlefin/malaketh-layered), commit
`63ac0589972412cb856a633d708a3d83bdada9aa`. It is an Engine API shim using
Malachite's channel interface and Reth. The repository documents a three-node
deployment sustaining a 1,000 transaction-per-second input load and producing
roughly six blocks per second. It also records an unresolved limitation: many
blocks were empty and transaction load above 1,000 TPS filled the mempools.
These figures are upstream PoC measurements, not Neunode production claims.

The pinned reference workspace builds locally on Apple Silicon with Rust 1.93
after installing `protoc`:

```text
cargo check --workspace
Finished `dev` profile [optimized + debuginfo]
```

The full upstream recipe was reproduced locally with Colima/Docker after the
compile check. Three independent Reth containers and three Malachite processes
reached consensus. After approximately 41 seconds, the RPC heads were
`0x178`, `0x179`, and `0x179`; node logs showed consensus deciding height 378
and entering height 379. The test containers were then shut down cleanly.

This proves the pinned reference integration works on the development host. It
does not replace Neunode's own Phase 2 conformance and recovery tests.

## Pinned integration surface

| Component | Pin | License |
|---|---|---|
| malaketh-layered reference | `63ac0589972412cb856a633d708a3d83bdada9aa` | Apache-2.0 |
| Malachite channel crates | `0968a34ba747130467569b1d10b2b1ef18f4b69b` | Apache-2.0 |
| Reth reference dependencies | `v1.2.0` / `1e965caf` | Apache-2.0/MIT |
| Neunode Rust toolchain | `1.93` | project pin |

The production bridge must use the exact Malachite commit above until an
upgrade passes the same compile, three-node liveness, restart, and Engine API
conformance gates. Git dependencies must remain revision-pinned, never branch-
pinned.

## Running a network validator

Build `malachitebft-eth-app` from the pinned `malaketh-layered` revision recorded above and generate
one validator home per node with its `testnet` command. Each home must contain the same genesis
validator set and a distinct private validator key; its Engine RPC configuration must point to that
node's Reth instance. Start each node through agnetd:

```bash
agnetd serve --chain-mode sovereign \
  --consensus-mode malachite \
  --malachite-path /opt/neunode/malachitebft-eth-app \
  --malachite-home /var/lib/neunode/validator-0 \
  --malachite-working-dir /opt/neunode/malaketh-layered \
  --external-engine \
  --engine-api-endpoint http://127.0.0.1:8551 \
  --jwt-secret-path /var/lib/neunode/reth/jwt.hex
```

In this mode agnetd supervises the external Malachite process, which supplies Tendermint networking,
crash recovery, proposal propagation, commit certificates, and decided-value synchronization. The
default `single` consensus mode is explicitly development-only and does not claim BFT finality.

`cargo metadata` also found MPL-2.0 transitive crates (`attohttpc`, `fastrlp`,
`option-ext`, and `webpki-roots`) plus `ring 0.16.20`, whose Cargo metadata has
no SPDX expression. MPL-2.0 is OSI-approved weak copyleft, so this is not a
non-FOSS dependency, but it contradicts an Apache/MIT-only policy. Before a
production release, `cargo-deny` must explicitly allow the reviewed MPL
packages and verify `ring`'s ISC/OpenSSL-style licensing from its distributed
license files, or dependency upgrades must remove them. No GPL, AGPL,
proprietary, or source-unavailable dependency was identified in the resolved
graph.

## Required event mapping

The Phase 2 bridge will follow the reference flow rather than the temporary
single-node timer driver:

1. `ConsensusReady` loads the latest committed EL block with
   `eth_getBlockByNumber` and initializes application state.
2. `GetValue` calls `forkchoiceUpdatedV3` with payload attributes, then
   `getPayloadV3`, persists the proposal, and replies with proposal parts.
3. `ReceivedProposalPart` reassembles and persists a candidate block.
4. `Decided` submits non-local payloads with `newPayloadV3`, finalizes the
   decided hash with `forkchoiceUpdatedV3`, persists the certificate/state,
   and advances height.
5. Any EL rejection stalls the consensus response. The bridge must not vote
   for or advance past an unvalidated execution payload.

## Crate layout

The repository keeps the existing flat-source convention:

- `neunode-engine-api-client`: JWT-authenticated Engine API transport and types.
- `neunode-chain-spec`: genesis, deterministic predeploys, and chain constants.
- `neunode-consensus-bridge`: Malachite channel handler, proposal cache, and WAL.
- `agnetd`: process orchestration and operational CLI only.

No nested `mod.rs` trees are permitted.

## GO rationale and exit gates

The integration is tractable because the reference project implements the
same channel-to-Engine-API boundary and compiles against revision-pinned
Malachite. Neunode should proceed, but Phase 2 is not complete until all of the
following are demonstrated locally or in CI:

- three Malachite validators finalize blocks through independent Reth nodes;
- restart recovery resumes from the persisted height/proposal/certificate;
- invalid execution payloads prevent voting/finalization;
- the Neunode genesis loads and every predeploy is callable;
- an EIP-1559 transaction spends `neu` as the native gas asset;
- dependency license and advisory checks pass for the shipped graph.

If the pinned channel API cannot pass those gates, the decision changes to
NO-GO and the fallback is Substrate + Frontier. Performance optimization is
explicitly secondary to correctness, recovery, and finality evidence.
