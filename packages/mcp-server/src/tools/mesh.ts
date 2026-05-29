/**
 * Mesh/network tools for the Neunode MCP server.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { AgnetdClient } from "../client.js";

export function registerMeshTools(
  server: McpServer,
  client: AgnetdClient,
): void {
  server.tool(
    "neunode_get_peers",
    "List currently connected peers in the Neunode mesh network",
    {},
    async () => {
      const result = await client.getPeers();
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_get_network_info",
    "Get status of the local mesh node (peer ID, listeners, topics, connected peers)",
    {},
    async () => {
      const result = await client.getNetworkInfo();
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );

  server.tool(
    "neunode_discover",
    "Connect to a peer on the Neunode mesh network by multiaddr",
    {
      addr: z
        .string()
        .min(1)
        .describe("Multiaddr of the peer to connect to (e.g. /ip4/1.2.3.4/tcp/41000/p2p/12D3Koo...)"),
    },
    async ({ addr }) => {
      const result = await client.discover(addr);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    },
  );
}
