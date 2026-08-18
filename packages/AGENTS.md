# PACKAGES — npm Distribution

## OVERVIEW

Everything Neunode publishes to npm except `@neunode/sdk` (which lives in `/sdk`).

```
packages/
├── agnetd/               # @neunode/agnetd — thin Node wrapper that resolves a platform binary
├── agnetd-linux-x64/     # @neunode/agnetd-linux-x64      (os: linux, cpu: x64)
├── agnetd-linux-arm64/   # @neunode/agnetd-linux-arm64    (os: linux, cpu: arm64)
├── agnetd-darwin-arm64/  # @neunode/agnetd-darwin-arm64   (os: darwin, cpu: arm64)
└── mcp-server/           # @neunode/mcp-server — MCP server over the agnetd HTTP API
```

## AGNETD BINARY DISTRIBUTION

`@neunode/agnetd` is an esbuild-style platform-package wrapper:

- `bin/agnetd` — launcher
- `lib/platform.js` — `generateBinPath()`, maps `process.platform`/`process.arch` to a platform package
- `install.js` — `postinstall` hook that verifies the binary exists and `chmod 0755`s it
- The three platform packages are listed as `optionalDependencies`, so npm installs only the
  matching one. Installing with `--no-optional` breaks it, and `install.js` says so.

**The binaries are never committed.** `.gitignore` excludes `packages/agnetd-*/agnetd*`;
`release.yml` cross-compiles them, drops them into the platform packages, rewrites every
`package.json` version (and the `optionalDependencies` pins) from the `v*` tag, then publishes.

Adding a target means: a matrix entry in `release.yml`, a new `packages/agnetd-<platform>/`
package with correct `os`/`cpu`, an `optionalDependencies` entry in `packages/agnetd/package.json`,
and a branch in `lib/platform.js`.

## MCP SERVER

`@neunode/mcp-server` exposes Neunode to MCP clients (Claude Code, Cursor, Windsurf). It is a pure
HTTP client of the daemon — it holds no protocol logic and never opens the RocksDB database:

```
AI agent → MCP → @neunode/mcp-server → HTTP → agnetd serve → P2P network
```

```
src/
├── index.ts       # Entry point, --transport flag
├── server.ts      # McpServer wiring
├── client.ts      # AgnetdClient — HTTP client for /api/v1
├── tools/         # 7 modules: identity, feed, inference, bounty, token, model, mesh
├── resources.ts   # MCP resources
├── prompts.ts     # MCP prompts
└── types.ts
```

Tools are registered in `src/tools/index.ts` via `registerAllTools(server, client)`. To add one:
create/extend a module exporting `registerXTools(server, client)`, register it in `index.ts`, and
make sure the endpoint it calls exists in `crates/agnetd/src/api_routes.rs`. Tool names are prefixed
`neunode_*`. Zod (`^3.25`) defines the schemas.

Transports: `stdio` (default) and Streamable HTTP on `127.0.0.1:3100` (`/mcp`, with a legacy `/sse`
endpoint that gives each connection an isolated session).

| Env var | Default | Meaning |
|---|---|---|
| `AGNETD_URL` | `http://127.0.0.1:41000` | Daemon URL |
| `MCP_TRANSPORT` | `stdio` | `stdio` or `http` |
| `MCP_PORT` | `3100` | Port in HTTP mode |

**Port mismatch to watch for:** `agnetd serve` defaults to **8080**, but this package (and
`examples/`) default to **41000**. Start the daemon with `--port 41000` or set `AGNETD_URL`.

## COMMANDS

```bash
cd mcp-server && npm ci
cd mcp-server && npm run typecheck   # CI enforced
cd mcp-server && npm test            # vitest; CI enforced
cd mcp-server && npm run build       # tsup; CI enforced
cd mcp-server && npm run start:stdio # or start:http
```

The `mcp-server` job in `.github/workflows/ci.yml` runs typecheck → test → build on Node 22 with
`npm ci` against `packages/mcp-server/package-lock.json`. The wrapper packages have no CI job; they
are exercised by `release.yml`.

## NOTES

- `mcp-server` depends on `@modelcontextprotocol/sdk` and `zod` only — it does **not** depend on
  `@neunode/sdk`. Keep it that way unless you intend to couple their release cycles.
- Versions across all five packages are set from the git tag at release time; the committed `0.1.0`
  values are placeholders. Don't bump them by hand.
- `packages/agnetd/node_modules/` may exist locally from a wrapper test — it is not tracked.
