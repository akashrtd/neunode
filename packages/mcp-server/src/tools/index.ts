/**
 * Tool registration — wires all tool modules to the MCP server.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { AgnetdClient } from "../client.js";
import { registerIdentityTools } from "./identity.js";
import { registerFeedTools } from "./feed.js";
import { registerInferenceTools } from "./inference.js";
import { registerBountyTools } from "./bounty.js";
import { registerTokenTools } from "./token.js";
import { registerModelTools } from "./model.js";
import { registerMeshTools } from "./mesh.js";
import { registerKnowledgeTools } from "./knowledge.js";
import { registerReputationTools } from "./reputation.js";
import { registerTrainingTools } from "./training.js";

export function registerAllTools(
  server: McpServer,
  client: AgnetdClient,
): void {
  registerIdentityTools(server, client);
  registerFeedTools(server, client);
  registerInferenceTools(server, client);
  registerBountyTools(server, client);
  registerTokenTools(server, client);
  registerModelTools(server, client);
  registerMeshTools(server, client);
  registerTrainingTools(server, client);
  registerReputationTools(server, client);
  registerKnowledgeTools(server, client);
}
