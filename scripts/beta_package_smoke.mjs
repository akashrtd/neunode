// Test packed distributions from a fresh installation against a real isolated daemon.
// Usage: node scripts/beta_package_smoke.mjs <agnetd> <sdk.tgz> <mcp.tgz>
import assert from "node:assert/strict";
import { execFile, spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
import { pathToFileURL } from "node:url";
import { promisify } from "node:util";
import { createRequire } from "node:module";

const execute = promisify(execFile);
const [binaryArg, sdkArg, mcpArg] = process.argv.slice(2);
if (!binaryArg || !sdkArg || !mcpArg) throw new Error("Expected agnetd, SDK tarball and MCP tarball paths");
const [binary, sdkTarball, mcpTarball] = [binaryArg, sdkArg, mcpArg].map((path) => resolve(path));
const directory = await mkdtemp(join(tmpdir(), "neunode-package-smoke-"));
const home = join(directory, "home");
const env = { ...process.env, HOME: home, NEUNODE_API_KEY: "package-fixture-authority-token-32-characters" };
delete env.NEUNODE_KEYSTORE_KEY;
let daemon;
let mcp;
let daemonLog = "";
try {
  await writeFile(join(directory, "package.json"), JSON.stringify({ private: true, type: "module" }));
  await execute("npm", ["install", "--omit=optional", "--ignore-scripts", "--no-audit", "--no-fund", "--package-lock=false", sdkTarball, mcpTarball], { cwd: directory });
  const require = createRequire(join(directory, "package.json"));
  assert.throws(() => require.resolve("viem"), { code: "MODULE_NOT_FOUND" });
  const esm = await import(pathToFileURL(require.resolve("@neunode/sdk").replace(/\.cjs$/, ".js")).href);
  const cjs = require("@neunode/sdk");
  assert.equal(typeof cjs.createNeunodeClient, "function");
  await execute(binary, ["config", "set", "network.listen_addr", "/ip4/127.0.0.1/tcp/0"], { env });
  await execute(binary, ["identity", "create", "--name", "package-fixture"], { env });
  const listener = createServer();
  listener.listen(0, "127.0.0.1");
  await once(listener, "listening");
  const port = listener.address().port;
  await new Promise((done) => listener.close(done));
  const url = `http://127.0.0.1:${port}`;
  daemon = spawn(binary, ["serve", "--port", String(port)], { env, stdio: ["ignore", "pipe", "pipe"] });
  daemon.stderr.on("data", (chunk) => { daemonLog += chunk; });
  daemon.stdout.on("data", (chunk) => { daemonLog += chunk; });
  const deadline = Date.now() + 15_000;
  for (;;) {
    if (daemon.exitCode !== null) throw new Error(`Daemon exited: ${daemonLog}`);
    try { if ((await fetch(`${url}/api/v1/health`)).ok) break; } catch { /* Wait for listener. */ }
    if (Date.now() >= deadline) throw new Error(`Daemon startup timed out: ${daemonLog}`);
    await new Promise((done) => setTimeout(done, 50));
  }
  const client = esm.createNeunodeClient({ http: { baseUrl: url, apiKey: env.NEUNODE_API_KEY } });
  const event = await client.feed.post({ kind: 9001, content: "packed SDK evidence", tags: ["source=packed"] });
  assert.equal((await client.feed.show(event.event_id)).content, "packed SDK evidence");
  assert.equal((await client.security.breakers()).length, 3);
  const { Client } = await import(pathToFileURL(require.resolve("@modelcontextprotocol/sdk/client/index.js")).href);
  const { StdioClientTransport } = await import(pathToFileURL(require.resolve("@modelcontextprotocol/sdk/client/stdio.js")).href);
  mcp = new Client({ name: "packed-beta-smoke", version: "1.0.0" });
  await mcp.connect(new StdioClientTransport({ command: "npx", args: ["--no-install", "neunode-mcp", "--transport", "stdio"], cwd: directory, env: { ...env, NEUNODE_URL: url } }));
  const tools = await mcp.listTools();
  assert(tools.tools.some((tool) => tool.name === "neunode_post_feed"));
  const posted = await mcp.callTool({ name: "neunode_post_feed", arguments: { kind: 9002, content: "packed MCP evidence" } });
  assert(!posted.isError, JSON.stringify(posted));
  const feed = await client.feed.list();
  assert(feed.some((item) => item.content === "packed MCP evidence" && item.signature.startsWith("ed25519:")));
  console.log(JSON.stringify({ passed: ["fresh tarball installation", "ESM and CJS without optional viem", "authenticated SDK signed feed", "SDK safety controls", "npx MCP initialize and tool discovery", "MCP authenticated signed feed"], tool_count: tools.tools.length }));
} finally {
  await mcp?.close();
  if (daemon && daemon.exitCode === null) {
    daemon.kill("SIGINT");
    const timeout = setTimeout(() => daemon.kill("SIGKILL"), 5_000);
    await once(daemon, "exit");
    clearTimeout(timeout);
  }
  await rm(directory, { recursive: true, force: true });
}
