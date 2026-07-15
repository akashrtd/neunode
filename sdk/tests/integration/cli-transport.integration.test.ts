/**
 * Integration tests for CliTransport against the real `agnetd` binary.
 *
 * These tests spawn actual `agnetd` processes and verify JSON envelope
 * responses end-to-end. The entire suite is skipped when the binary
 * is not available (e.g., CI without a Rust build step).
 */

import { beforeAll, describe, it, expect } from "vitest";
import { CliTransport, CliTransportError } from "../../src/transport/cli-transport.js";
import { BINARY_PATH } from "./helpers/agnetd.js";

// ---------------------------------------------------------------------------
// Shared transport instance (uses discovered binary path)
// ---------------------------------------------------------------------------

function makeTransport(): CliTransport {
  return new CliTransport({
    binaryPath: BINARY_PATH ?? "agnetd",
    timeout: 15_000,
  });
}

// ===========================================================================
// A. Binary & Envelope Format
// ===========================================================================

describe("Integration: Binary & Envelope Format", () => {
  const transport = makeTransport();

  it("should find agnetd binary", () => {
    expect(BINARY_PATH).not.toBeNull();
  });

  it("should return JSON envelope with success=true for config list", async () => {
    const result = await transport.execute<Record<string, unknown>>([
      "config", "list",
    ]);
    expect(result).toBeDefined();
    expect(typeof result).toBe("object");
  });

  it("should parse data field from envelope with expected config keys", async () => {
    const result = await transport.execute<Record<string, unknown>>([
      "config", "list",
    ]);
    expect(result).toHaveProperty("agent.name");
    expect(result).toHaveProperty("storage.db_path");
  });
});

// ===========================================================================
// B. Identity Commands
// ===========================================================================

describe("Integration: Identity Commands", () => {
  const transport = makeTransport();
  let createdDid: string;

  it("should create a new identity", async () => {
    const ts = Date.now();
    const result = await transport.execute<Record<string, string>>([
      "identity", "create", "--name", `integration-test-${ts}`,
    ]);
    expect(result.DID).toBeDefined();
    expect(result.DID).toMatch(/^did:neunode:/);
    createdDid = result.DID;
  });

  it("should list identities", async () => {
    // identity list returns a single envelope with data as an array.
    // parseSingleEnvelope spreads arrays into indexed objects, so we use
    // executeRaw + manual parsing to preserve the array structure.
    const raw = await transport.executeRaw([
      "--output", "json-compact", "identity", "list",
    ]);
    const envelope = JSON.parse(raw.trim()) as { data: Record<string,string>[]; success: boolean };
    expect(envelope.success).toBe(true);
    expect(Array.isArray(envelope.data)).toBe(true);
    expect(envelope.data.length).toBeGreaterThanOrEqual(1);
    const entry = envelope.data[0];
    expect(entry).toHaveProperty("DID");
    expect(entry).toHaveProperty("Status");
  });

  it("should show identity details (multi-envelope merge)", async () => {
    // `identity show` returns 2 envelopes; execute<T> merges them
    const result = await transport.execute<Record<string, unknown>>([
      "identity", "show", "--did", createdDid,
    ]);
    expect(result).toHaveProperty("did");
    expect(result).toHaveProperty("document");
    expect(result).toHaveProperty("verification_methods");
  });
});

// ===========================================================================
// C. Config Commands
// ===========================================================================

describe("Integration: Config Commands", () => {
  const transport = makeTransport();

  it("should list all config values", async () => {
    const result = await transport.execute<Record<string, unknown>>([
      "config", "list",
    ]);
    expect(result).toHaveProperty("agent.name");
    expect(result).toHaveProperty("storage.db_path");
  });

  it("should get a single config value", async () => {
    const result = await transport.execute<Record<string, unknown>>([
      "config", "get", "agent.name",
    ]);
    expect(result).toHaveProperty("agent.name");
  });
});

// ===========================================================================
// D. Bounty Commands
// ===========================================================================

describe("Integration: Bounty Commands", () => {
  const transport = makeTransport();

  beforeAll(async () => {
    await transport.execute(["token", "seed"]);
    await transport.execute(["token", "unstake", "--amount", "100"]);
  });

  it("should create a bounty", async () => {
    const result = await transport.execute<Record<string, unknown>>([
      "bounty", "create",
      "--title", "Integration test bounty",
      "--description", "Created by integration test",
      "--reward", "100",
      "--work-deadline", "259200",
    ]);
    expect(result).toHaveProperty("id");
    expect((result as Record<string, unknown>).id).toMatch(/^bnty_/);
    expect(result).toHaveProperty("state", "Open");
    expect(result).toHaveProperty("reward", 100);
    expect(result).toHaveProperty("title", "Integration test bounty");
  });

  it("should list bounties", async () => {
    // bounty list returns a single envelope with data as an array.
    // parseSingleEnvelope spreads arrays into indexed objects, so we use
    // executeRaw + manual parsing to preserve the array structure.
    const raw = await transport.executeRaw([
      "--output", "json-compact", "bounty", "list",
    ]);
    const envelope = JSON.parse(raw.trim()) as { data: Record<string,unknown>[]; success: boolean };
    expect(envelope.success).toBe(true);
    expect(Array.isArray(envelope.data)).toBe(true);
    expect(envelope.data.length).toBeGreaterThanOrEqual(1);
    const entry = envelope.data[0];
    expect(entry).toHaveProperty("ID");
    expect(entry).toHaveProperty("State");
    expect(entry).toHaveProperty("Creator");
    expect(entry).toHaveProperty("Reward");
  });
});

// ===========================================================================
// E. Token Commands (multi-envelope)
// ===========================================================================

describe("Integration: Token Commands (multi-envelope)", () => {
  const transport = makeTransport();

  it("should return token balance as multiple envelopes", async () => {
    const results = await transport.executeMulti<Record<string, string>>([
      "token", "balance", "--token", "nCompute",
    ]);
    expect(results).toHaveLength(3);
    // First envelope: token name
    expect(results[0]).toHaveProperty("token", "nCompute");
    // Second envelope: balance
    expect(results[1]).toHaveProperty("balance");
    // Third envelope: staked
    expect(results[2]).toHaveProperty("staked");
  });
});

// ===========================================================================
// F. Reputation Commands
// ===========================================================================

describe("Integration: Reputation Commands", () => {
  const transport = makeTransport();
  let agentDid: string;

  it("should create identity for reputation test", async () => {
    const ts = Date.now();
    const result = await transport.execute<Record<string, string>>([
      "identity", "create", "--name", `rep-test-${ts}`,
    ]);
    agentDid = result.DID;
  });

  it("should show reputation for an agent", async () => {
    const result = await transport.execute<Record<string, unknown>>([
      "reputation", "show", "--agent", agentDid,
    ]);
    expect(result).toHaveProperty("score");
    expect(result).toHaveProperty("grade");
    expect(result).toHaveProperty("factors");
    const factors = result.factors as Record<string, unknown>;
    expect(factors).toHaveProperty("stake");
    expect(factors).toHaveProperty("attest");
    expect(factors).toHaveProperty("activity");
    expect(factors).toHaveProperty("verify");
    expect(factors).toHaveProperty("tenure");
  });
});

// ===========================================================================
// G. Feed Commands
// ===========================================================================

describe("Integration: Feed Commands", () => {
  const transport = makeTransport();

  it("should post to feed", async () => {
    const result = await transport.execute<Record<string, string>>([
      "feed", "post",
      "--kind", "1000",
      "--content", '{"title":"integration test"}',
    ]);
    expect(result).toHaveProperty("Author");
    expect(result).toHaveProperty("Event ID");
    expect(result).toHaveProperty("Kind");
    expect(result).toHaveProperty("Topic");
    expect(result).toHaveProperty("Sequence");
  });
});

// ===========================================================================
// H. Error Handling
// ===========================================================================

describe("Integration: Error Handling", () => {
  const transport = makeTransport();

  it("should throw CliTransportError for invalid identity", async () => {
    try {
      await transport.execute(["identity", "show", "--did", "nonexistent"]);
      expect.unreachable("Expected CliTransportError to be thrown");
    } catch (err) {
      expect(err).toBeInstanceOf(CliTransportError);
      const transportErr = err as CliTransportError;
      expect(transportErr.stderr).toContain("not found");
    }
  });

  it("should throw CliTransportError for invalid subcommand", async () => {
    await expect(
      transport.execute(["model", "register", "--name", "test", "--cid", "abc"]),
    ).rejects.toThrow(CliTransportError);
  });
});
