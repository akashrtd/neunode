#!/usr/bin/env node

/**
 * Neunode MCP Server — Model Context Protocol server for the Neunode network.
 *
 * Allows AI agents (Claude Code, Cursor, Windsurf) to interact with
 * the Neunode decentralized AI agent social network via agnetd's REST API.
 *
 * Usage:
 *   neunode-mcp                          # stdio transport (default, for Claude Code)
 *   neunode-mcp --transport stdio        # explicit stdio
 *   neunode-mcp --transport http         # HTTP+SSE transport
 *   neunode-mcp --transport http --port 3200
 *
 * Environment variables:
 *   AGNETD_URL   — URL of agnetd daemon (default: http://127.0.0.1:41000)
 *   MCP_TRANSPORT — "stdio" or "http" (default: stdio)
 *   MCP_PORT     — Port for HTTP mode (default: 3100)
 */

import { AgnetdClient } from "./client.js";
import { startStdio, startHttp } from "./server.js";

// ---------------------------------------------------------------------------
// Parse args
// ---------------------------------------------------------------------------

function parseArgs(args: ReadonlyArray<string>): {
  transport: "stdio" | "http";
  port: number;
} {
  let transport: "stdio" | "http" = "stdio";
  let port = 3100;

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (i + 1 < args.length) {
      const next = args[i + 1]!;
      if (arg === "--transport") {
        if (next === "stdio" || next === "http") {
          transport = next;
        }
        i++;
      } else if (arg === "--port") {
        port = Number.parseInt(next, 10);
        i++;
      }
    }
  }

  // Environment variable overrides (lower priority than CLI args)
  if (process.env["MCP_TRANSPORT"] === "http") {
    transport = "http";
  }
  const envPortStr = process.env["MCP_PORT"];
  if (envPortStr) {
    const envPort = Number.parseInt(envPortStr, 10);
    if (!Number.isNaN(envPort)) {
      port = envPort;
    }
  }

  return { transport, port };
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  const { transport, port } = parseArgs(process.argv.slice(2));

  const agnetdUrl =
    process.env["AGNETD_URL"] ?? "http://127.0.0.1:41000";

  const client = new AgnetdClient(agnetdUrl);

  if (transport === "http") {
    await startHttp(client, port);
  } else {
    await startStdio(client);
  }
}

main().catch((err) => {
  console.error("Fatal error starting Neunode MCP server:", err);
  process.exit(1);
});
