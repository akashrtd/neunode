# Spike: Reth + Malachite L1 Chain Feasibility Assessment

**Date:** 2026-05-29
**Status:** CONDITIONAL GO
**Author:** Research spike (automated)

---

## Executive Summary

Building a Neunode L1 chain using Reth (execution layer) + Malachite (consensus layer) is **technically feasible** but carries significant risk due to Malachite's pre-alpha maturity. The integration would require building a custom "bridge" between Malachite's channel-based consensus API and Reth's Engine API, plus modifying Reth's fee logic for a non-ETH gas token. The total effort is estimated at 12-18 months for a production-ready chain, assuming a team of 3-5 engineers with distributed systems and Ethereum internals experience.

**Recommendation: CONDITIONAL GO** -- proceed only if you accept Malachite's pre-alpha risk, or consider CometBFT (Go) as a fallback consensus layer. An alternative path using a Substrate-based chain should also be evaluated before committing.

---

## 1. Reth Assessment

### 1.1 Production Readiness

| Attribute | Detail |
|---|---|
| **Current Version** | Reth 2.0 (released April 2026) |
| **Team** | Paradigm (well-funded, active development) |
| **License** | Apache 2.0 / MIT (dual-licensed, no business restrictions) |
| **Audit** | Sigma Prime (developers of Lighthouse CL). REVM audited by Guido Vranken. |
| **MSRV** | Rust 1.93.0 |
| **GitHub** | github.com/paradigmxyz/reth (5.6k stars, 2.5k forks) |

Reth is production-ready for Ethereum mainnet use. Paradigm recommends it for mission-critical staking, RPC, MEV, and indexing workloads. Storage V2 is default in Reth 2.0.

**Verdict: Production-ready.** Actively maintained, well-documented, audited. No concerns.

### 1.2 Custom Chain Spec Support

Reth natively supports custom chain specifications via the `--chain <CHAIN_OR_PATH>` flag. You can provide a path to a custom chain spec JSON file that defines:

- Chain ID
- Genesis allocation
- Hard fork activation conditions
- Gas limit settings
- Block reward configuration

The `ChainSpecBuilder` API allows programmatic construction:

```rust
let chain_spec = Arc::new(
    ChainSpecBuilder::from(&*MAINNET)
        .shanghai_activated()
        .with_fork(EthereumHardfork::Cancun, ForkCondition::Timestamp(1))
        .build(),
);
```

The `EthEvmConfig` is generic over chain spec types that implement `EthereumHardforks`, so custom chain specs work at the type level.

**Verdict: Full support.** Reth was designed to support multiple EVM chains. The README explicitly states: "Support as many EVM chains as possible."

### 1.3 Engine API Exposure

Reth exposes the standard Engine API (specified in ethereum/execution-apis) on an authenticated HTTP endpoint:

- Default port: 8551
- JWT authentication required (`--authrpc.jwtsecret`)
- Standard methods: `engine_newPayloadV*`, `engine_forkchoiceUpdatedV*`, `engine_getPayloadV*`
- Fork-scoped versions: Paris (V1), Shanghai (V2), Cancun (V3), Prague (V4)

The Engine API tree handler (`EngineApiTreeHandler`) in `crates/engine/tree/src/tree/mod.rs` is a well-structured component that:
- Processes incoming payloads via `on_new_payload()`
- Manages fork choice state updates
- Handles block buffering and backfill sync
- Spawns in its own OS thread for performance
- Communicates via crossbeam channels internally

Reth also provides e2e test utilities for testing Engine API interactions with custom chain specs.

**Verdict: Standard-compliant, well-tested.** Any CL implementing the Engine API spec can communicate with Reth.

### 1.4 Modular Node Architecture

Reth's node builder pattern supports custom component injection:

```rust
builder
    .with_types::<EthereumNode>()
    .with_components(
        EthereumNode::components()
            .pool(CustomPoolBuilder::default())
            .network(CustomNetworkBuilder::default())
            .executor(CustomExecutorBuilder::default())
            .consensus(CustomConsensusBuilder::default())
            .payload(CustomPayloadBuilder::default())
    )
    .with_add_ons(EthereumAddOns::default())
    .launch()
```

This means we can swap out individual components (transaction pool, payload builder, consensus) without forking Reth.

**Verdict: Highly modular.** The builder pattern makes customization straightforward.

### 1.5 Known Limitations for Custom Chain Usage

1. **OP-Reth moved out**: Optimism's OP-Reth integration has moved to `ethereum-optimism/optimism`. This means the primary example of Reth + alternative consensus is no longer maintained in the main Reth repo.
2. **Engine API assumes Ethereum block structure**: The Engine API payloads follow Ethereum's block format. Custom block fields would require extending the API.
3. **Fee logic is ETH-native**: Transaction validation assumes gas fees are paid in the native token (ETH). Using a custom gas token requires modifications (see Section 4).
4. **No built-in support for custom consensus**: Reth expects a standard Ethereum CL (Lighthouse, Prysm, etc.) on the other end of the Engine API.

---

## 2. Malachite Assessment

### 2.1 What is Malachite?

Malachite is a Byzantine-fault tolerant (BFT) consensus engine implemented in Rust by Informal Systems (the team behind CometBFT/Tendermint). It provides a state-of-the-art Rust implementation of the Tendermint consensus algorithm.

| Attribute | Detail |
|---|---|
| **Status** | **Pre-alpha. Not for production use.** |
| **Team** | Informal Systems (CometBFT maintainers) |
| **License** | Apache 2.0 |
| **Language** | Rust (MSRV 1.82+) |
| **GitHub** | github.com/informalsystems/malachite |

### 2.2 Relationship to CometBFT/Tendermint

Malachite is a **from-scratch Rust reimplementation** of the Tendermint consensus algorithm (the same algorithm that powers CometBFT in Go). Key differences:

- **No shared code** with Go-based CometBFT
- Addresses technical debt from years of CometBFT maintenance
- Co-designed with formal specification and model checking
- More flexible API (not tied to ABCI)

### 2.3 Current Users

- **Starknet**: Malachite originated as the consensus core for the Starknet L2 decentralized sequencer. Used in Madara and Pathfinder Starknet clients.
- **Farcaster Snapchain**: Powers Farcaster's newest backend layer.
- Other teams building in private.

### 2.4 Performance

Early benchmarks (from README):
- Average finalization latency: **780ms** at 100 validators with 1MB blocks
- Throughput: up to **2.5 blocks/second** or **13.5 MB/s** (~50,000 TPS)

### 2.5 API Architecture

Malachite provides three integration levels:

#### Channel-based interface (recommended)
The `malachitebft-app-channel` crate provides a Tokio channel-based interface. The application handles messages via a simple event loop:

```
AppMsg::ConsensusReady     -> reply with start height + validator set
AppMsg::StartedRound       -> update internal state
AppMsg::GetValue           -> build and return a value to propose
AppMsg::ReceivedProposalPart -> process incoming proposal parts
AppMsg::Decided            -> commit value, start next height
AppMsg::GetValidatorSet    -> return validator set for a given height
AppMsg::ExtendVote         -> attach custom data to votes
AppMsg::VerifyVoteExtension -> validate vote extensions
```

#### Actor-based interface
Lower-level, allows swapping networking layer, sync protocol, etc.

#### Core consensus library
Pure, stateless consensus library with no I/O. Maximum flexibility.

### 2.6 Validator Set Customization

The `Context` trait allows full customization of validator types:

```rust
pub trait Context {
    type Validator: Validator<Self>;
    type ValidatorSet: ValidatorSet<Self>;
    // ...
    fn select_proposer(&self, validator_set: &Self::ValidatorSet,
                       height: Self::Height, round: Round) -> &Self::Validator;
}
```

The tutorial shows height-based validator set rotation, where `get_validator_set(height)` returns a rotating subset. This is exactly the pattern needed for **reputation-weighted validation** -- the application controls which validators are active at each height and their relative voting power.

**Verdict: Fully customizable validator sets.** Reputation-weighted validation is natively supported by the Context trait.

### 2.7 Risk Assessment

| Risk | Severity | Mitigation |
|---|---|---|
| Pre-alpha status | **HIGH** | Software has not been externally audited. API may change without notice. |
| Limited production track record | **HIGH** | Only Starknet sequencer and Snapchain in production. No L1 chain uses it yet. |
| Documentation gaps | **MEDIUM** | Channel tutorial is comprehensive. Actor/low-level APIs lack tutorials. |
| Small community | **MEDIUM** | ~10 stars on GitHub (the informalsystems fork). Primary development happens in the circlefin fork. |
| No slashing mechanism | **MEDIUM** | Must be built at the application layer. |

**Verdict: High-risk dependency.** The consensus algorithm is sound (Tendermint, well-understood), and the team is credible (Informal Systems), but the implementation is not production-ready.

---

## 3. Engine API Integration

### 3.1 Engine API Specification

The Engine API is defined in [ethereum/execution-apis](https://github.com/ethereum/execution-apis/tree/main/src/engine) and is the standard interface between execution and consensus layers post-Merge. It is fork-scoped:

- **Paris** (V1): `engine_newPayloadV1`, `engine_forkchoiceUpdatedV1`, `engine_getPayloadV1`
- **Shanghai** (V2): Adds withdrawal processing
- **Cancun** (V3): Adds blob transactions, EIP-4788 beacon roots
- **Prague** (V4): Adds execution layer requests

Core methods:

| Method | Purpose |
|---|---|
| `engine_newPayloadV*` | CL sends a new execution payload to EL for validation |
| `engine_forkchoiceUpdatedV*` | CL tells EL which block is head/safe/finalized; optionally triggers payload building |
| `engine_getPayloadV*` | CL retrieves a built payload from EL |

### 3.2 Authentication

The Engine API uses **JWT authentication** over HTTP:
- Shared JWT secret between CL and EL
- Configured via `--authrpc.jwtsecret` in Reth
- HMAC-SHA256, 24-hour expiry

### 3.3 Communication Architecture

For Neunode, the bridge between Malachite CL and Reth EL would look like:

```
Malachite CL (Rust)
    |
    | [Neunode Bridge Layer]
    |   - Translates Malachite AppMsg to Engine API calls
    |   - Manages block lifecycle
    |
    v
Reth EL (HTTP + JWT)
    engine_newPayloadV3
    engine_forkchoiceUpdatedV3
    engine_getPayloadV3
```

The bridge layer would:

1. On `AppMsg::GetValue`: Call `engine_forkchoiceUpdated` with payload attributes to trigger block building, then `engine_getPayload` to retrieve the built block.
2. On `AppMsg::Decided`: Call `engine_newPayload` with the decided block, then `engine_forkchoiceUpdated` to set it as head.
3. On `AppMsg::GetValidatorSet`: Return reputation-weighted validator set from Neunode state.

### 3.4 Existing Integrations

- **OP-Reth**: Previously integrated Reth with Optimism's consensus. Moved to `ethereum-optimism/optimism` repo. Demonstrates that Reth can work with alternative consensus layers.
- **Starknet + Malachite**: Malachite serves as consensus for Starknet sequencer (Madara, Pathfinder). However, Starknet uses a custom execution layer (Cairo VM), not an EVM, so this is not a direct reference for Reth integration.

**Key gap**: No existing project bridges Reth (EVM EL) with Malachite (BFT CL). This bridge layer is novel work.

---

## 4. Custom Gas Token Analysis

### 4.1 The Problem

Ethereum's EVM hardcodes ETH (address `0x0`) as the fee-paying token. Transaction validation in `revm` (the EVM used by Reth) checks that the sender has sufficient ETH balance to cover `gas_limit * max_fee_per_gas`.

### 4.2 Approaches to Non-ETH Gas Tokens

**Option A: Deploy `neu` as an ERC-20, keep ETH for gas**

Simplest approach. `neu` tokens are ERC-20 on the Neunode chain. Gas is still paid in the native token (which we call `neu-wei` but is functionally identical to ETH at the protocol level).

- Zero modifications to Reth
- Users still need the native token for gas (but we can airdrop it freely)
- Token economics (decay, membrane) implemented at the ERC-20 level via smart contracts
- Downside: Two-token confusion. Native `neu` for gas, ERC-20 `neu` for everything else.

**Option B: Rename native token, no protocol changes**

Call the native token `neu` instead of ETH. At the protocol level, nothing changes -- it's still the base asset used for gas. The "customization" is purely branding.

- Zero modifications to Reth
- Gas token = native token = `neu`
- Token economics (decay, etc.) need to be implemented at a different layer
- This is what most L1 chains do (e.g., Polygon uses MATIC, Avalanche uses AVAX)

**Option C: True custom gas token (ERC-20 fee token)**

Allow gas to be paid in an ERC-20 token, bypassing the native token requirement. This is what Celo does and what EIP-3074/ERC-4337 enable indirectly.

- Requires significant modifications to Reth's transaction validation pipeline
- Must modify `revm`'s `validate_tx_against_state()` logic
- Must modify block reward/fee distribution logic
- Must modify EIP-1559 base fee calculation (currently in native token units)
- Risk of diverging from upstream Reth, creating a maintenance burden
- **This is the hardest option and the one with the most ongoing maintenance cost.**

### 4.3 Recommendation

**Use Option B** for the initial chain launch. The native token IS `neu`. This requires zero code changes in Reth -- just a custom chain spec with the desired token name/symbol in metadata. The chain ID and genesis allocation are configurable.

For the agent UX concern (agents needing gas), implement **fee delegation/gas sponsorship** at the application layer:
- ERC-4337 (Account Abstraction) paymasters can sponsor gas for agents
- Meta-transactions via a relay contract
- A "faucet" service that auto-funds new agent wallets

This eliminates the "agents need ETH for gas" problem without modifying the execution layer.

### 4.4 Token Economics on the Custom Chain

For decay, membrane, and reputation-weighted staking:
- **Decay**: Implement as a smart contract (your existing `NeunodeToken` Solidity contracts already have this). Run decay calculations in block `apply_pre_execution_changes` via a system call.
- **Membrane**: Enforced via smart contract logic (bonding/unbonding with reputation requirements).
- **Staking**: Native staking contract with slashing conditions.

All of this works without modifying Reth. The EVM execution layer executes the smart contracts; the consensus layer (Malachite) handles block production and finality.

---

## 5. GO/NO-GO Assessment

### 5.1 Verdict: CONDITIONAL GO

The integration is technically feasible. Both Reth and Malachite are Rust-based, well-architected, and designed for extensibility. However, the condition is accepting Malachite's pre-alpha risk.

### 5.2 Effort Estimate

| Phase | Duration | Description |
|---|---|---|
| **Phase 0: Prototype** | 6-8 weeks | Malachite channel app that talks to Reth via Engine API. Single-node, no real networking. Proves the bridge works. |
| **Phase 1: Testnet** | 4-6 months | Multi-node testnet with reputation-weighted validator set. Custom chain spec. Smart contract deployment for token economics. |
| **Phase 2: Hardening** | 3-4 months | Security audit, stress testing, slashing implementation, monitoring/tooling. |
| **Phase 3: Mainnet** | 2-3 months | Genesis validator bootstrapping, public testnet, mainnet launch. |
| **Total** | 12-18 months | With 3-5 engineers |

### 5.3 Key Risks

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Malachite API breaking changes | **HIGH** | **HIGH** | Pin to a specific commit. Budget for rework. |
| Engine API bridge is harder than expected | **MEDIUM** | **MEDIUM** | The OP-Reth codebase serves as a reference for Reth + alternative CL integration. |
| Custom gas token requires Reth forks | **LOW** (if Option B) | **LOW** | Use native token as `neu`. Avoid protocol-level changes. |
| Malachite is never production-ready | **MEDIUM** | **CRITICAL** | Fallback to CometBFT (Go). Requires CGo bridge or separate process. Or use a different consensus library entirely. |
| Insufficient TPS for agent microtransactions | **LOW** | **MEDIUM** | Tendermint BFT achieves ~50k TPS in benchmarks. More than sufficient. Block time target: 1-2s. |

### 5.4 Alternatives Considered

#### Alternative A: Substrate (Polkadot SDK)
- Pros: Purpose-built for custom L1 chains. Native support for custom tokens, consensus, governance. Rust-based. Production-proven.
- Cons: Not EVM-compatible by default (requires Frontier). Smaller DeFi ecosystem. Different execution model.
- **Recommendation**: Evaluate as a parallel option. If EVM compatibility is non-negotiable, Substrate is harder. If you can use Substrate's native runtime, it's arguably a better fit.

#### Alternative B: CometBFT (Go) + Reth
- Pros: CometBFT is production-proven. Used by Cosmos ecosystem (hundreds of chains). ABCI++ provides rich application interface.
- Cons: Written in Go (not Rust). Requires inter-process communication between Go CL and Rust EL. Larger operational surface.
- **Recommendation**: Strong fallback if Malachite is not ready in time. The ABCI++ interface is well-documented and the bridge to Engine API is well-understood.

#### Alternative C: Rollkit (Sovereign Rollup) + Reth
- Pros: Deploy as a sovereign rollup on Celestia for data availability. Fast time-to-market.
- Cons: Depends on Celestia for DA. Less control over consensus. Not a true L1.
- **Recommendation**: Consider for a faster MVP, but does not meet the stated goal of building a true L1.

#### Alternative D: Fork Geth + CometBFT
- Pros: Most battle-tested combination. More examples of custom chains (Cosmos EVM chains).
- Cons: Go-based (inconsistent with Neunode's Rust stack). Geth is GPL-licensed (more restrictive).
- **Recommendation**: Lower-risk but moves away from Rust.

### 5.5 Critical Decision Points

Before committing to Reth + Malachite, resolve these questions:

1. **Is Malachite's pre-alpha status acceptable?** If not, switch to CometBFT (Go) or Substrate.
2. **Is EVM compatibility required?** If agents primarily interact via the SDK (not directly with EVM), Substrate may be a better fit.
3. **Can the native token be `neu` (Option B)?** If true custom gas token (Option C) is required, the effort doubles.
4. **What is the minimum viable validator set size?** Tendermint requires 2/3+1 honest validators. For reputation-weighted validation, you need at least 4 validators (tolerates 1 faulty).

---

## 6. Technical Architecture (Proposed)

```
                          Neunode L1 Architecture
                          =======================

+------------------+       Engine API        +-----------------------+
|                  |  <--- HTTP/JWT ----->   |                       |
|   Reth EL        |                         |   Malachite CL        |
|   (EVM exec)     |                         |   (Tendermint BFT)    |
|                  |                         |                       |
| - Custom chain   |                         | - Reputation-weighted |
|   spec (neu)     |                         |   validator set       |
| - Neunode        |                         | - 1-2s block times    |
|   contracts      |                         | - Custom proposer     |
| - Standard       |                         |   selection           |
|   JSON-RPC       |                         | - Vote extensions for |
|                  |                         |   reputation updates  |
+------------------+                         +-----------------------+
        |                                              |
        v                                              v
+------------------+                         +-----------------------+
|   MDBX Database  |                         |   libp2p Networking   |
|   (Reth Storage) |                         |   (P2P Gossip)        |
+------------------+                         +-----------------------+
                                                      |
                                                      v
                                              +-----------------------+
                                              |  Neunode Bridge       |
                                              |  (Rust Application)   |
                                              |                       |
                                              | - AppMsg handler      |
                                              | - Engine API client   |
                                              | - Validator set mgr   |
                                              | - Reputation oracle   |
                                              +-----------------------+
```

### 6.1 Neunode Bridge Layer

The bridge is the key integration component. It implements Malachite's `AppMsg` handler and translates consensus events into Engine API calls:

```rust
// Pseudocode for the bridge
async fn handle_consensus_msg(msg: AppMsg, channels: &mut Channels) {
    match msg {
        AppMsg::ConsensusReady { reply } => {
            let start_height = store.last_decided_height().increment();
            let validator_set = get_reputation_weighted_validators(start_height);
            reply.send((start_height, validator_set));
        }

        AppMsg::GetValue { height, round, reply } => {
            // Trigger block building in Reth
            let payload_id = engine_api.forkchoice_updated(head, payload_attrs).await;
            let payload = engine_api.get_payload(payload_id).await;
            reply.send(payload.into());
        }

        AppMsg::Decided { certificate, reply } => {
            // Submit decided block to Reth
            engine_api.new_payload(certificate.value).await;
            engine_api.forkchoice_updated(head, safe, finalized).await;
            store.commit(certificate);
            reply.send(ConsensusMsg::StartHeight(next_height, next_validator_set));
        }

        AppMsg::GetValidatorSet { height, reply } => {
            let set = get_reputation_weighted_validators(height);
            reply.send(set);
        }

        // ... other handlers
    }
}
```

### 6.2 Reputation-Weighted Validation

The validator set at each height is determined by on-chain reputation scores:

```rust
fn get_reputation_weighted_validators(height: Height) -> ValidatorSet {
    // Query on-chain reputation state from Reth
    let scores: Vec<(Address, ReputationScore)> = reth_provider
        .query_reputation(height)
        .top_n(MAX_VALIDATORS);

    ValidatorSet::new(
        scores.into_iter().map(|(addr, score)| {
            Validator::new(
                addr.public_key(),
                VotingPower::from(score.weighted_score()) // 5-factor: stake(30%) + attest(25%) + activity(20%) + verify(15%) + tenure(10%)
            )
        })
    )
}
```

This maps directly to Neunode's existing 5-factor reputation system (stake 30%, attest 25%, activity 20%, verify 15%, tenure 10%).

---

## 7. Compatibility with Existing Neunode Stack

### 7.1 Rust Core

Neunode's Rust stack (agnetd, neunode-core through neunode-identity) is edition 2021, Rust 1.93. Reth also targets Rust 1.93+. Malachite requires Rust 1.82+. **Fully compatible.**

However, there is a substantial dependency conflict risk. Reth depends on specific versions of `alloy`, `revm`, and other Ethereum ecosystem crates. Neunode also uses `alloy` (via `alloy sol!` macro for ABI + EIP-712 signing). These must be version-aligned.

### 7.2 Solidity Contracts

Neunode's existing Solidity contracts (NeunodeIdentity, NeunodeBounty, NeunodeEscrow, etc.) would deploy directly onto the Reth EL. The EIP-2535 Diamond proxy pattern, ERC-20 tokens with decay+staking, and governance contracts all work on any EVM-compatible chain.

**No modifications needed** for the Solidity contracts -- just deploy them on the custom chain with a new genesis allocation.

### 7.3 TypeScript SDK

The SDK uses CLI subprocess transport (`agnetd --output json-compact`) and optional Viem for on-chain ops. On the L1 chain:
- JSON-RPC endpoint provided by Reth (standard Ethereum API)
- Viem connects directly (no changes needed, just new chain config)
- CLI commands work against the Reth RPC

**Minimal changes needed** -- primarily adding a new chain configuration and potentially adjusting gas price defaults.

---

## 8. Summary

| Dimension | Assessment |
|---|---|
| **Reth EL** | Production-ready. Custom chain specs supported. Modular architecture. |
| **Malachite CL** | Pre-alpha. Credible team. Sound algorithm. High integration risk. |
| **Engine API Bridge** | Novel work. No existing reference. Moderate complexity. |
| **Custom Gas Token** | Use native token as `neu` (Option B). Zero Reth changes. |
| **Reputation-weighted Validation** | Natively supported by Malachite's Context trait. |
| **Smart Contract Compatibility** | Full. Existing contracts deploy without changes. |
| **SDK Compatibility** | Minimal changes (new chain config). |
| **Estimated Effort** | 12-18 months, 3-5 engineers. |
| **Biggest Risk** | Malachite pre-alpha status. Mitigate with CometBFT fallback. |

### Recommended Next Steps

1. **(Week 1-2)** Build a single-node prototype: Malachite channel app + Reth node connected via Engine API. Prove the bridge concept.
2. **(Week 3-4)** Implement reputation-weighted validator selection in the bridge. Test with 4 local validators.
3. **(Week 5-6)** Deploy Neunode Solidity contracts on the custom chain. Test token economics.
4. **(Week 7-8)** Evaluate if Malachite is stable enough to proceed. If not, pivot to CometBFT or Substrate.

---

## References

- Reth GitHub: https://github.com/paradigmxyz/reth
- Reth Book: https://paradigmxyz.github.io/reth/
- Malachite GitHub: https://github.com/informalsystems/malachite
- Malachite Architecture: https://github.com/informalsystems/malachite/blob/main/ARCHITECTURE.md
- Malachite Channel Tutorial: https://github.com/informalsystems/malachite/blob/main/docs/tutorials/channels.md
- Engine API Spec: https://github.com/ethereum/execution-apis/blob/main/src/engine/paris.md
- EIP-3675 (PoS consensus): https://eips.ethereum.org/EIPS/eip-3675
