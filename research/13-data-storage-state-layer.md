# Neunode — Data Storage & State Management Layer

## The Problem

```
A decentralized agent network needs ONE storage engine that handles:
  • Millions of append-only feed events (SSB sigchain pattern)
  • 6-permutation triple indexes for SPARQL-like graph queries
  • Token balances with sub-millisecond read latency
  • Merkle-tree state sync for peer catchup
  • Point-in-time snapshots for disaster recovery
```

---

## Storage Engine Selection

| Requirement | Solution | Precedent |
|---|---|---|
| Embedded KV store | RocksDB v0.24 (rust-rocksdb) | TiKV, Sui, Aptos, Cloudflare |
| Column Families | 20 CFs for access pattern isolation | Sui/Aptos blockchain state |
| In-process cache | moka v0.12 (concurrent, TTL, LRU) | Production Rust services |
| Distributed routing | libp2p KadDHT | IPFS, libp2p ecosystem |
| Large blob storage | IPFS/rust-ipfs (CID-addressed) | Model checkpoints, datasets |
| State sync | Sparse Merkle Tree | zksync, Aptos state sync |

---

## Column Family Schema

```
┌──────────────────────────────────────────────────────────┐
│                   ROCKSDB INSTANCE                        │
│                                                           │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌──────────┐ │
│  │ identity  │ │  config   │ │feed_events│ │feed_index│ │
│  │ DID docs  │ │ settings  │ │ sigchain  │ │secondary │ │
│  │ agent card│ │ net params│ │append-only│ │ indexes  │ │
│  └───────────┘ └───────────┘ └───────────┘ └──────────┘ │
│                                                           │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌──────────┐ │
│  │feed_state │ │  tokens   │ │reputation │ │  models  │ │
│  │ subs/offs │ │ balances  │ │  scores   │ │ lineage  │ │
│  │ filters   │ │ stakes    │ │attest hist│ │ serving  │ │
│  └───────────┘ └───────────┘ └───────────┘ └──────────┘ │
│                                                           │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌──────────┐ │
│  │ training  │ │ bounties  │ │ p2p_state │ │snapshots │ │
│  │checkpoints│ │  claims   │ │ DHT routes│ │point-in-t│ │
│  │ contribs  │ │submissions│ │  conns    │ │   time   │ │
│  └───────────┘ └───────────┘ └───────────┘ └──────────┘ │
│                                                           │
│  ┌───────────┐ ┌──────────────────────────────────────┐  │
│  │ kg_id2str │ │  KG PERMUTATION INDEXES (6)          │  │
│  │ string    │ │ spog  posg  ospg  gspo  gpos  gosp  │  │
│  │ dictionary│ │ SipHash24 keys, 64-byte quad entries │  │
│  └───────────┘ └──────────────────────────────────────┘  │
│                                                           │
│  ┌────────────────────────────────────────────────────┐   │
│  │ merkle_nodes — Sparse Merkle Tree for state sync   │   │
│  └────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────┘
```

---

## Key Design Per Column Family

### Feed Events — Core Append-Only Log

```
Key: [agent_did_hash(16) | sequence(u64 BE)]

  ┌───────────────────────────────────────────────┐
  │ agent_did_hash (16B) │ sequence (8B, big-end) │
  │ SipHash24(DID)[..16] │ monotonically increasing│
  │ ▲── prefix for range scan ──┘                  │
  └───────────────────────────────────────────────┘

  One agent's entire feed is contiguous on disk → sequential I/O

Value: { kind:u16, timestamp:u64, payload:bytes, prev_hash:[u8;32], signature:bytes }
  kind: 1000=bounty, 2000=training, 3000=attestation, 4000=model
  prev_hash: SHA-256 of previous event (SSB sigchain)
  signature: Ed25519 detached sig over (kind || ts || payload || prev_hash)
```

### Feed Index — Secondary Queries

```
Key: [index_type(1) | index_key(var) | agent_did_hash(16) | sequence(u64 BE)]
  0x01 by_kind → kind(u16 BE)  │  0x02 by_time → timestamp(u64 BE) reversed
  0x03 by_tag  → tag_hash(16)  │  0x04 by_ref  → event_hash(32)
Value: empty (key-only index → resolves back to feed_events)
```

### Knowledge Graph — String Dictionary + 6 Indexes

```
kg_id2str  Key: SipHash24-128(string) = 16 bytes fixed
           Val: { original_string, type_flag(URI|literal|blank|var) }
           Strings ≤23B inline, longer → LZ4. Insert-only, never delete.

6 PERMUTATION INDEXES (Oxigraph pattern — each sorts quads differently):
  Index   Sort Order     Best Query Pattern
  ─────   ───────────    ──────────────────────
  spog    S-P-O-G        ?s p o g  (subject+predicate)
  posg    P-O-S-G        p ?o ?s g (predicate first)
  ospg    O-S-P-G        ?s ?p o g (object known)
  gspo    G-S-P-O        g ?s ?p ?o (graph-scoped)
  gpos    G-P-O-S        g p ?o ?s (graph+predicate)
  gosp    G-O-S-P        g ?s o ?p (graph+object)

  Key: [hash(S)|hash(P)|hash(O)|hash(G)] — 64 bytes fixed, value = empty

  Query example — "Who attested agent X?":
    ?S <attested> <did:neunode:X> <graph:reputation>
    → Use OSPG index, prefix scan [hash(X)|hash(attested)]
    → Resolve S hashes → strings via kg_id2str
```

### Hot-Path CFs (L1 Cached)

```
tokens       Key: [agent_did_hash(16)|token_type(1)]
             Val: { balance:u128, staked:u128, allowance:u128, last_decay_epoch:u64 }
             token_type: 0x01=compute 0x02=training 0x03=bandwidth 0x04=storage

reputation   Key: [agent_did_hash(16)]
             Val: { score:f64, stake/attest/activity/verify/tenure weights, last_updated:u64 }
```

### Domain & Infrastructure CFs

```
models       Key: model_cid_hash(16)  → { cid, parent_cids, contributor_did, contrib_type, sig, metadata }
training     Key: job_id_hash(16)     → { job_type, status, assignees, checkpoints, shapley_contribs }
bounties     Key: bounty_id_hash(16)  → { state_machine, claims, submissions, escrow, deadlines }
p2p_state    Key: prefix(1)+peer_id   → { multiaddrs, conn_state, last_seen, score }
feed_state   Key: sub_hash(16)+id(u64)→ { filter_config, read_offset, last_seq }
merkle_nodes Key: tree_id(1)+path(32) → { hash:[u8;32], value_hash, is_leaf }
snapshots    Key: epoch(u64 BE)       → { root_hash, timestamp, block_height, cf_digests }
config       Key: setting_name        → { value }
```

---

## 3-Tier Cache

```
  Query → L1 MOKA (<1μs, TTL 60s, 10K entries) ─HIT→ Return
           │MISS
           → L2 RocksDB Block Cache (~1μs, 256MB) ─HIT→ Return + promote
              │MISS
              → L3 OS Page Cache (~10μs) ─HIT→ Return + promote
                 │MISS
                 → Disk SST → Return + promote to L1

  Hot (tokens, rep): L1 >95% │ Warm (index, bounty): L2 >80% │ Cold: L3
```

---

## Data Flow

```
WRITE PATH:
  Agent Action → Validate+Sign ─→ WriteBatch (atomic):
    1. Append feed_events  2. Update feed_index (4 keys)
    3. Invalidate L1       4. Batch-update merkle_nodes
    5. Snapshot if epoch boundary
    → WAL → MemTable → SST files (L0-L6)

READ PATH:
  Query → L1(moka) → L2(RocksDB block) → L3(OS page) → Disk(SST)
          Each miss level promotes to L1 on hit
```

---

## Merkle State Sync

```
  New Peer                              Network
  ─────────                             ───────
       │  GET /snapshot/latest          │
       │───────────────────────────────→│
       │  { epoch:50000, root:0xABCD }  │
       │←───────────────────────────────│
       │  GET /snapshot/50000/data      │
       │───────────────────────────────→│
       │  [compressed snapshot stream]  │
       │←───────────────────────────────│
       │  Apply + verify root_hash      │
       │  GET /deltas?from=50000        │
       │───────────────────────────────→│
       │  [delta stream 50001..current] │
       │←───────────────────────────────│
       │  Apply deltas, verify root     │
       │  CAUGHT UP ✓                  │

  Sparse Merkle Tree:
       root_hash
       ╱         ╲
   left_hash   right_hash
    ╱    ╲       ╱     ╲
  leaf  leaf   leaf   leaf
  (CID) (bal)  (rep)  (state)

  Each leaf = hash(key-value pair). Non-membership: siblings → empty subtree.
```

---

## Compaction Strategy

```
ROCKSDB LEVEL COMPACTION (L0→L6, 10× size ratio per level)

feed_events:  Level-based │ Keep last 10K/agent │ Archive old to IPFS
kg_*:         Periodic tombstone cleanup │ id2str: never delete
tokens/rep:   Aggressive compaction │ Keep only latest balance
merkle_nodes: Snapshot-triggered │ Prune old tree versions after N snapshots
snapshots:    Keep last 100 locally │ Older → archive to IPFS

TUNING: write_buffer_size=64MB, max_buffers=3, target_file_size=64MB,
        max_bytes_for_level_base=256MB, compaction=Level
```

---

## Storage Size Estimates

```
┌──────────────────────┬────────────┬─────────────┬──────────────┐
│  CF / Data           │  1K Agents │ 10K Agents  │ 100K Agents  │
├──────────────────────┼────────────┼─────────────┼──────────────┤
│  feed_events (1K/ag) │    2 GB    │   20 GB     │   200 GB     │
│  feed_index (4×)     │    8 GB    │   80 GB     │   800 GB     │
│  kg_* (6 idx,10K/ag) │    3 GB    │   30 GB     │   300 GB     │
│  identity            │   50 MB    │  500 MB     │     5 GB     │
│  tokens/reputation   │   10 MB    │  100 MB     │     1 GB     │
│  models/training     │  100 MB    │    1 GB     │    10 GB     │
│  bounties            │   50 MB    │  500 MB     │     5 GB     │
│  merkle/snapshots    │  200 MB    │    2 GB     │    20 GB     │
│  p2p_state           │    5 MB    │   50 MB     │   500 MB     │
├──────────────────────┼────────────┼─────────────┼──────────────┤
│  TOTAL raw           │   ~14 GB   │  ~135 GB    │  ~1.3 TB     │
│  TOTAL compressed    │   ~4 GB    │   ~40 GB    │  ~400 GB     │
└──────────────────────┴────────────┴─────────────┴──────────────┘

Nodes store subscribed feeds + local state only. Archive nodes = everything.
Light nodes <1 GB. LZ4/Zstd compression ~3.5× on structured data.
```

---

## Design Decisions

```
WHY NOT SQLite?     → No CF support, single-writer lock, 10× worse write throughput
WHY NOT sled/redb?  → sled abandoned, redb unproven at scale. RocksDB: 10yr track record
WHY 6 KG INDEXES?   → Cover ALL triple patterns. Oxigraph: 2-50× faster vs 3 indexes.
                     Storage cost is modest — entries are 64-byte hashes only.
WHY MOKA?           → Purpose-built cache with TTL+LRU+async. 4× vs dashmap for caches.
WHY SMT not Trie?   → Fixed-depth, O(log n) proofs, simpler code. Aptos+zksync chose SMT.
```

---

## Key Crates

| Crate | Version | Purpose |
|---|---|---|
| `rust-rocksdb` | v0.24 | Embedded KV, CF support, snapshots, compaction |
| `moka` | v0.12 | Concurrent in-process cache (TTL, LRU, weigher) |
| `cid` | v0.11 | Content-addressed IPFS identifiers |
| `multihash` | v0.19 | Multihash digest (SHA-256, Blake3) |
| `rust-ipfs` | — | Large blob storage (checkpoints, datasets) |
| `serde` | v1.0 | Serialization (JSON + bincode for binary values) |
| `bincode` | v3.0 | Compact binary encoding for hot-path CFs |
| `twox-hash` | — | SipHash24 for KG string dictionary |

---

## References

- **RocksDB** — https://rocksdb.org — Meta's embedded KV store
- **TiKV** — https://tikv.org — Distributed RocksDB (TiDB foundation)
- **Sui/Aptos** — RocksDB CF pattern for blockchain state (Narwhal/Bullshark)
- **Oxigraph** — https://github.com/oxigraph/oxigraph — RDF store, 6 permutation indexes
- **moka** — https://github.com/moka-rs/moka — Rust concurrent cache
- **zksync** — Merkle checkpoints + delta replay state sync
- **SSB** — Secure Scuttlebutt sigchain — append-only, sequence + prev_hash
