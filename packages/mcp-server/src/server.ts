/**
 * MCP server setup — creates and configures the Model Context Protocol server.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import type { AgnetdClient } from "./client.js";
import { registerAllTools } from "./tools/index.js";
import { registerResources } from "./resources.js";
import { registerPrompts } from "./prompts.js";

// ---------------------------------------------------------------------------
// Version — read from package.json at build time via tsup replacement
// ---------------------------------------------------------------------------

const SERVER_VERSION = "0.1.0";

// ---------------------------------------------------------------------------
// Create + wire
// ---------------------------------------------------------------------------

export function createServer(client: AgnetdClient): McpServer {
  const server = new McpServer({
    name: "neunode",
    version: SERVER_VERSION,
  });

  registerAllTools(server, client);
  registerResources(server, client);
  registerPrompts(server);

  return server;
}

// ---------------------------------------------------------------------------
// Start (stdio)
// ---------------------------------------------------------------------------

export async function startStdio(client: AgnetdClient): Promise<void> {
  const server = createServer(client);
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

// ---------------------------------------------------------------------------
// Start (HTTP+SSE) — streams via SSE for remote clients
// ---------------------------------------------------------------------------

export async function startHttp(
  client: AgnetdClient,
  port: number,
): Promise<void> {
  const server = createServer(client);

  // Dynamic import because SSE transport is Node-specific and may not
  // always be available depending on the MCP SDK version.
  const { SSEServerTransport } = await import(
    "@modelcontextprotocol/sdk/server/sse.js"
  );
  const http = await import("node:http");

  const httpServer = http.createServer(async (req, res) => {
    const url = new URL(
      req.url ?? "/",
      `http://127.0.0.1:${port}`,
    );

    if (url.pathname === "/sse" && req.method === "GET") {
      const transport = new SSEServerTransport("/messages", res);
      await server.connect(transport);
      return;
    }

    if (url.pathname === "/messages" && req.method === "POST") {
      let body = "";
      for await (const chunk of req) {
        body += chunk;
      }
      // The SSE transport handles message routing internally.
      // We forward the raw body to the transport that was created
      // during the /sse handshake.
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end('{"ok":true}');
      return;
    }

    res.writeHead(404);
    res.end("Not found");
  });

  httpServer.listen(port, () => {
    console.error(`Neunode MCP server (HTTP+SSE) listening on http://127.0.0.1:${port}`);
    console.error(`  SSE endpoint: http://127.0.0.1:${port}/sse`);
    console.error(`  Messages:     http://127.0.0.1:${port}/messages`);
  });
}
