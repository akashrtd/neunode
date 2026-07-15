/** MCP server construction and transport lifecycles. */

import http, {
  type IncomingMessage,
  type Server as HttpServer,
  type ServerResponse,
} from "node:http";
import { randomUUID } from "node:crypto";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { SSEServerTransport } from "@modelcontextprotocol/sdk/server/sse.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import type { AgnetdClient } from "./client.js";
import { registerAllTools } from "./tools/index.js";
import { registerResources } from "./resources.js";
import { registerPrompts } from "./prompts.js";

const SERVER_VERSION = "0.1.0";
const MAX_HTTP_SESSIONS = 100;

type HttpSession = {
  readonly server: McpServer;
  readonly transport: SSEServerTransport | StreamableHTTPServerTransport;
};

export function createServer(client: AgnetdClient): McpServer {
  const server = new McpServer({ name: "neunode", version: SERVER_VERSION });
  registerAllTools(server, client);
  registerResources(server, client);
  registerPrompts(server);
  return server;
}

export async function startStdio(client: AgnetdClient): Promise<void> {
  await createServer(client).connect(new StdioServerTransport());
}

/**
 * Create modern Streamable HTTP and legacy SSE endpoints. Every session gets
 * an isolated protocol server because the MCP SDK permits one transport per
 * server.
 */
export function createHttpServer(client: AgnetdClient): HttpServer {
  const sessions = new Map<string, HttpSession>();

  const httpServer = http.createServer(async (req, res) => {
    try {
      if (!isLocalRequest(req)) {
        respond(res, 403, "Forbidden");
        return;
      }

      const url = new URL(req.url ?? "/", "http://localhost");
      if (url.pathname === "/mcp") {
        const rawSessionId = req.headers["mcp-session-id"];
        const sessionId = Array.isArray(rawSessionId) ? undefined : rawSessionId;
        const existing = sessionId ? sessions.get(sessionId) : undefined;
        if (existing) {
          if (!(existing.transport instanceof StreamableHTTPServerTransport)) {
            respond(res, 400, "Session belongs to a different MCP transport");
            return;
          }
          await existing.transport.handleRequest(req, res);
          return;
        }
        if (sessionId) {
          respond(res, 404, "Unknown or expired MCP session");
          return;
        }
        if (req.method !== "POST") {
          respond(res, 400, "A new MCP session must begin with POST initialize");
          return;
        }
        if (sessions.size >= MAX_HTTP_SESSIONS) {
          respond(res, 503, "Too many active MCP sessions");
          return;
        }

        let initializedSessionId: string | undefined;
        let transport: StreamableHTTPServerTransport;
        const server = createServer(client);
        transport = new StreamableHTTPServerTransport({
          sessionIdGenerator: randomUUID,
          onsessioninitialized: (newSessionId) => {
            initializedSessionId = newSessionId;
            sessions.set(newSessionId, { server, transport });
          },
        });
        transport.onclose = () => {
          if (initializedSessionId) sessions.delete(initializedSessionId);
        };
        await server.connect(transport);
        await transport.handleRequest(req, res);
        if (!initializedSessionId) await server.close();
        return;
      }

      if (url.pathname === "/sse" && req.method === "GET") {
        if (sessions.size >= MAX_HTTP_SESSIONS) {
          respond(res, 503, "Too many active MCP sessions");
          return;
        }

        const transport = new SSEServerTransport("/messages", res);
        const server = createServer(client);
        const sessionId = transport.sessionId;
        transport.onclose = () => sessions.delete(sessionId);
        sessions.set(sessionId, { server, transport });
        try {
          await server.connect(transport);
        } catch (error) {
          sessions.delete(sessionId);
          if (!res.headersSent) {
            respond(res, 500, "Failed to establish MCP session");
          }
          console.error("Failed to establish MCP SSE session:", error);
        }
        return;
      }

      if (url.pathname === "/messages" && req.method === "POST") {
        const sessionId = url.searchParams.get("sessionId");
        if (!sessionId) {
          respond(res, 400, "Missing sessionId");
          return;
        }
        const session = sessions.get(sessionId);
        if (!session) {
          respond(res, 404, "Unknown or expired MCP session");
          return;
        }
        if (!(session.transport instanceof SSEServerTransport)) {
          respond(res, 400, "Session belongs to a different MCP transport");
          return;
        }
        await session.transport.handlePostMessage(req, res);
        return;
      }

      respond(res, 404, "Not found");
    } catch (error) {
      console.error("MCP HTTP request failed:", error);
      if (!res.headersSent) {
        respond(res, 500, "Internal server error");
      } else if (!res.writableEnded) {
        res.end();
      }
    }
  });

  httpServer.on("close", () => {
    for (const { transport } of sessions.values()) {
      void transport.close();
    }
    sessions.clear();
  });
  return httpServer;
}

export async function startHttp(
  client: AgnetdClient,
  port: number,
): Promise<HttpServer> {
  const server = createHttpServer(client);
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, "127.0.0.1", () => {
      server.off("error", reject);
      resolve();
    });
  });
  console.error(`Neunode MCP server (HTTP) listening on http://127.0.0.1:${port}`);
  console.error(`  Streamable HTTP: http://127.0.0.1:${port}/mcp`);
  console.error(`  Legacy SSE:     http://127.0.0.1:${port}/sse`);
  return server;
}

function respond(res: ServerResponse, status: number, body: string): void {
  res.writeHead(status, { "Content-Type": "text/plain; charset=utf-8" });
  res.end(body);
}

/** Reject browser DNS-rebinding requests while allowing local MCP clients. */
function isLocalRequest(req: IncomingMessage): boolean {
  const host = req.headers.host;
  if (!host) return false;
  try {
    const hostname = new URL(`http://${host}`).hostname;
    if (hostname !== "127.0.0.1" && hostname !== "localhost" && hostname !== "[::1]") {
      return false;
    }
  } catch {
    return false;
  }

  const origin = req.headers.origin;
  if (!origin) return true;
  try {
    const hostname = new URL(origin).hostname;
    return hostname === "127.0.0.1" || hostname === "localhost" || hostname === "[::1]";
  } catch {
    return false;
  }
}
