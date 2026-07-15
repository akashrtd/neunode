import type { AddressInfo } from "node:net";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { SSEClientTransport } from "@modelcontextprotocol/sdk/client/sse.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { afterEach, describe, expect, it } from "vitest";
import type { AgnetdClient } from "./client.js";
import { createHttpServer } from "./server.js";

describe("HTTP+SSE MCP transport", () => {
  const openServers: Array<ReturnType<typeof createHttpServer>> = [];

  afterEach(async () => {
    await Promise.all(
      openServers.splice(0).map(
        (server) =>
          new Promise<void>((resolve, reject) => {
            server.close((error) => (error ? reject(error) : resolve()));
          }),
      ),
    );
  });

  async function listen(
    path = "/mcp",
    agnetdClient: AgnetdClient = {} as unknown as AgnetdClient,
  ): Promise<URL> {
    const server = createHttpServer(agnetdClient);
    openServers.push(server);
    await new Promise<void>((resolve, reject) => {
      server.once("error", reject);
      server.listen(0, "127.0.0.1", resolve);
    });
    const address = server.address() as AddressInfo;
    return new URL(`http://127.0.0.1:${address.port}${path}`);
  }

  it("completes initialization and tool discovery over Streamable HTTP", async () => {
    const agnetdClient = {
      getModel: async (modelId: string) => ({ model_id: modelId, status: "registered" }),
    } as unknown as AgnetdClient;
    const endpoint = await listen("/mcp", agnetdClient);
    const client = new Client({ name: "integration-test", version: "1.0.0" });
    await client.connect(new StreamableHTTPClientTransport(endpoint));

    const response = await client.listTools();
    const names = response.tools.map((tool) => tool.name);
    expect(names).toContain("neunode_request_inference");
    expect(names).toContain("neunode_get_registered_model");
    expect(names).not.toContain("neunode_get_inference_result");
    expect(names).not.toContain("neunode_set_pricing");

    const toolResult = await client.callTool({
      name: "neunode_get_registered_model",
      arguments: { model_id: "model-7" },
    });
    expect(JSON.stringify(toolResult.content)).toContain("model-7");

    await client.close();
  });

  it("keeps the legacy SSE session POST path protocol-complete", async () => {
    const endpoint = await listen("/sse");
    const client = new Client({ name: "legacy-integration-test", version: "1.0.0" });
    await client.connect(new SSEClientTransport(endpoint));
    expect((await client.listTools()).tools.length).toBeGreaterThan(0);
    await client.close();
  });

  it("rejects message posts without a valid session", async () => {
    const endpoint = await listen("/sse");
    const response = await fetch(new URL("/messages", endpoint), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ jsonrpc: "2.0", method: "ping", id: 1 }),
    });
    expect(response.status).toBe(400);

    const unknown = await fetch(new URL("/messages?sessionId=missing", endpoint), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ jsonrpc: "2.0", method: "ping", id: 1 }),
    });
    expect(unknown.status).toBe(404);
  });
});
