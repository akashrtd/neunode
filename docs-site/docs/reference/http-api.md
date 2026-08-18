---
title: HTTP API reference
description: All 80 routes served by agnetd, generated from the router source.
---

# HTTP API reference

`agnetd serve` exposes these routes under `/api/v1`. The daemon binds `127.0.0.1:8080` by default.

!!! tip "Prefer the generated browser"

    `http://127.0.0.1:8080/swagger-ui` is generated from the server at runtime, and
    `/api-docs/openapi.json` gives you the raw schema. Both are more authoritative than this page.

## Conventions

Every response uses the same envelope as the CLI.

```json
{ "data": {  }, "success": true }
```

```json
{ "error": { "code": "...", "message": "..." }, "success": false }
```

Path segments in braces are parameters, for example `/api/v1/bounties/{id}`.

## Routes

The table below was extracted directly from
[`crates/agnetd/src/api_routes.rs`](https://github.com/akashrtd/neunode/blob/main/crates/agnetd/src/api_routes.rs).
It lists **80 route registrations** across 18 groups.

### `/health`

| Method | Path |
|---|---|
| GET | `/api/v1/health` |

### `/audit`

| Method | Path |
|---|---|
| GET | `/api/v1/audit` |
| GET | `/api/v1/audit/verify` |

### `/identity`

| Method | Path |
|---|---|
| GET | `/api/v1/identity` |
| POST | `/api/v1/identity/create` |
| GET | `/api/v1/identity/list` |
| GET | `/api/v1/identity/export` |
| POST | `/api/v1/identity/register-onchain` |

### `/feed`

| Method | Path |
|---|---|
| GET · POST | `/api/v1/feed` |
| GET | `/api/v1/feed/{event_id}` |

### `/bounties`

| Method | Path |
|---|---|
| GET · POST | `/api/v1/bounties` |
| GET | `/api/v1/bounties/{id}` |
| POST | `/api/v1/bounties/{id}/claim` |
| POST | `/api/v1/bounties/{id}/submit` |
| POST | `/api/v1/bounties/{id}/review` |
| POST | `/api/v1/bounties/{id}/pay` |
| POST | `/api/v1/bounties/{id}/cancel` |

### `/tokens`

| Method | Path |
|---|---|
| GET | `/api/v1/tokens/balance` |
| POST | `/api/v1/tokens/transfer` |
| POST | `/api/v1/tokens/stake` |
| POST | `/api/v1/tokens/unstake` |
| POST | `/api/v1/tokens/claim-unbonded` |
| GET | `/api/v1/tokens/stake-status` |
| GET | `/api/v1/tokens/decay-info` |

### `/inference`

| Method | Path |
|---|---|
| POST | `/api/v1/inference/request` |
| GET | `/api/v1/inference/models` |
| GET · POST | `/api/v1/inference/providers` |
| GET | `/api/v1/inference/route` |
| GET | `/api/v1/inference/pricing` |

### `/discovery`

| Method | Path |
|---|---|
| GET | `/api/v1/discovery/search` |
| GET | `/api/v1/discovery/complement` |
| GET | `/api/v1/discovery/gaps` |
| GET | `/api/v1/discovery/score` |
| GET | `/api/v1/discovery/weights` |

### `/mesh`

| Method | Path |
|---|---|
| GET | `/api/v1/mesh/status` |
| GET | `/api/v1/mesh/peers` |
| POST | `/api/v1/mesh/connect` |
| POST | `/api/v1/mesh/disconnect` |

### `/knowledge`

| Method | Path |
|---|---|
| GET | `/api/v1/knowledge/query` |
| POST | `/api/v1/knowledge/register-agent` |
| POST | `/api/v1/knowledge/register-model` |
| POST | `/api/v1/knowledge/register-bounty` |
| POST | `/api/v1/knowledge/join-job` |
| GET | `/api/v1/knowledge/classes` |
| GET | `/api/v1/knowledge/predicates` |

### `/reputation`

| Method | Path |
|---|---|
| GET | `/api/v1/reputation` |
| POST | `/api/v1/reputation/attest` |
| GET | `/api/v1/reputation/leaderboard` |
| GET | `/api/v1/reputation/factors` |

### `/models`

| Method | Path |
|---|---|
| GET · POST | `/api/v1/models` |
| GET · DELETE | `/api/v1/models/{model_id}` |

### `/lineage`

| Method | Path |
|---|---|
| POST | `/api/v1/lineage/register` |
| GET | `/api/v1/lineage/{cid}` |
| GET | `/api/v1/lineage/{cid}/parents` |
| GET | `/api/v1/lineage/{cid}/children` |
| GET | `/api/v1/lineage/{cid}/ancestors` |
| GET | `/api/v1/lineage/{cid}/depth` |
| POST | `/api/v1/lineage/{cid}/royalties` |
| POST | `/api/v1/lineage/hash` |
| POST | `/api/v1/lineage/verify` |

### `/train`

| Method | Path |
|---|---|
| POST | `/api/v1/train/start` |
| GET | `/api/v1/train/status` |
| POST | `/api/v1/train/stop` |
| GET | `/api/v1/train/jobs` |
| POST | `/api/v1/train/worker-register` |
| GET | `/api/v1/train/workers` |
| GET | `/api/v1/train/coordinator-status` |

### `/turboquant`

| Method | Path |
|---|---|
| POST | `/api/v1/turboquant/compress` |
| POST | `/api/v1/turboquant/codebook` |

### `/config`

| Method | Path |
|---|---|
| GET · PUT | `/api/v1/config` |
| GET | `/api/v1/config/path` |

### `/lifecycle`

| Method | Path |
|---|---|
| GET | `/api/v1/lifecycle/status` |
| POST | `/api/v1/lifecycle/activate` |
| POST | `/api/v1/lifecycle/hibernate` |
| POST | `/api/v1/lifecycle/reactivate` |
| GET | `/api/v1/lifecycle/list` |
| POST | `/api/v1/lifecycle/reap` |

### `/verification`

| Method | Path |
|---|---|
| POST | `/api/v1/verification/tee/intel-tdx` |
| POST | `/api/v1/verification/tee/amd-snp` |
| POST | `/api/v1/verification/tee/amd-vlek` |

## Other endpoints

| Path | Purpose |
|---|---|
| `/swagger-ui` | Interactive API browser |
| `/api-docs/openapi.json` | Generated OpenAPI schema |
| WebSocket feed stream | Live event delivery without polling |

## A note for contributors

These handlers live in `crates/agnetd/src/api_*.rs`, one module per group, and are registered in
`api_routes.rs`. The CLI in `cmd_*.rs` is a parallel surface over the same crate logic.

**Adding a CLI command does not add an HTTP route.** Nothing enforces parity between the two, and
the compiler will not warn you. The SDK and the MCP server only consume the HTTP surface, so a
CLI-only capability is invisible to both. Add both surfaces in the same change.
