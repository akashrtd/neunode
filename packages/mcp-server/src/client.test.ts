import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { AgnetdClient, AgnetdClientError } from "./client";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";

describe("AgnetdClient", () => {
  let client: AgnetdClient;
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    client = new AgnetdClient("http://127.0.0.1:41000", 5000);
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ---------------------------------------------------------------------------
  // Construction
  // ---------------------------------------------------------------------------

  describe("construction", () => {
    it("uses the provided base URL", () => {
      const c = new AgnetdClient("http://example.com:9999/");
      expect(c).toBeInstanceOf(AgnetdClient);
    });

    it("uses default timeout of 30s", () => {
      const c = new AgnetdClient("http://localhost:41000");
      expect(c).toBeInstanceOf(AgnetdClient);
    });

    it("accepts custom timeout", () => {
      const c = new AgnetdClient("http://localhost:41000", 1000);
      expect(c).toBeInstanceOf(AgnetdClient);
    });
  });

  // ---------------------------------------------------------------------------
  // Request building
  // ---------------------------------------------------------------------------

  describe("GET requests", () => {
    it("sends GET to the correct URL with JSON content-type", async () => {
      fetchMock.mockResolvedValue(
        new Response(JSON.stringify({ data: { did: "test" }, success: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

      await client.whoami();

      expect(fetchMock).toHaveBeenCalledTimes(1);
      const callArgs = fetchMock.mock.calls[0]!;
      const url = callArgs[0] as string;
      const init = callArgs[1] as RequestInit;
      expect(url).toBe("http://127.0.0.1:41000/api/v1/identity");
      expect(init.method).toBe("GET");
      expect(init.headers).toEqual({
        "Content-Type": "application/json",
      });
    });

    it("builds query strings for list endpoints", async () => {
      fetchMock.mockResolvedValue(
        new Response(JSON.stringify({ data: [], success: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

      await client.listBounties({ state: "Open", limit: 10 });

      const callArgs = fetchMock.mock.calls[0]!;
      const url = callArgs[0] as string;
      expect(url).toContain("/api/v1/bounties?");
      expect(url).toContain("state=Open");
      expect(url).toContain("limit=10");
    });
  });

  describe("POST requests", () => {
    it("sends POST with JSON body", async () => {
      fetchMock.mockResolvedValue(
        new Response(
          JSON.stringify({
            data: { identity: { did: "did:key:abc" }, card_cid: "QmX" },
            success: true,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      );

      await client.createIdentity({ name: "test-agent" });

      const callArgs = fetchMock.mock.calls[0]!;
      const url = callArgs[0] as string;
      const init = callArgs[1] as RequestInit;
      expect(url).toBe("http://127.0.0.1:41000/api/v1/identity/create");
      expect(init.method).toBe("POST");
      expect(JSON.parse(init.body as string)).toEqual({
        name: "test-agent",
        method: "key",
      });
    });
  });

  // ---------------------------------------------------------------------------
  // JSON envelope parsing
  // ---------------------------------------------------------------------------

  describe("envelope parsing", () => {
    it("unwraps success envelope and returns data", async () => {
      const data = { did: "did:key:abc", method: "key", name: "agent", ethereum: "0x", peer_id: "12D3Koo" };
      fetchMock.mockResolvedValue(
        new Response(JSON.stringify({ data, success: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

      const result = await client.whoami();
      expect(result).toEqual(data);
    });

    it("throws AgnetdClientError for error envelope", async () => {
      const errorBody = JSON.stringify({
        error: { code: "NOT_FOUND", message: "identity not found" },
        success: false,
      });
      fetchMock.mockResolvedValue(
        new Response(errorBody, {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

      const thrown = await client.whoami().catch((e: unknown) => e);
      expect(thrown).toBeInstanceOf(AgnetdClientError);
      expect((thrown as AgnetdClientError).message).toBe("identity not found");
      expect((thrown as AgnetdClientError).code).toBe("NOT_FOUND");
    });

    it("throws AgnetdClientError for HTTP error with error envelope", async () => {
      fetchMock.mockResolvedValue(
        new Response(
          JSON.stringify({
            error: { code: "VALIDATION_ERROR", message: "invalid input" },
            success: false,
          }),
          { status: 400, headers: { "Content-Type": "application/json" } },
        ),
      );

      await expect(client.createIdentity({ name: "" })).rejects.toThrow("invalid input");
    });

    it("throws AgnetdClientError for HTTP error with non-JSON body", async () => {
      fetchMock.mockResolvedValue(
        new Response("Internal Server Error", {
          status: 500,
        }),
      );

      await expect(client.whoami()).rejects.toThrow("HTTP 500");
    });
  });

  // ---------------------------------------------------------------------------
  // Connection errors
  // ---------------------------------------------------------------------------

  describe("error handling", () => {
    it("throws CONNECTION_REFUSED on fetch failure", async () => {
      const err = new TypeError("fetch failed");
      fetchMock.mockRejectedValue(err);

      const thrown = await client.whoami().catch((e: unknown) => e);
      expect(thrown).toBeInstanceOf(AgnetdClientError);
      expect((thrown as AgnetdClientError).code).toBe("CONNECTION_REFUSED");
      expect((thrown as AgnetdClientError).message).toContain("cannot connect");
    });

    it("throws TIMEOUT on abort", async () => {
      const err = new DOMException("The operation was aborted", "AbortError");
      fetchMock.mockRejectedValue(err);

      const thrown = await client.whoami().catch((e: unknown) => e);
      expect(thrown).toBeInstanceOf(AgnetdClientError);
      expect((thrown as AgnetdClientError).code).toBe("TIMEOUT");
    });

    it("throws NETWORK_ERROR for unknown errors", async () => {
      fetchMock.mockRejectedValue(new Error("something unexpected"));

      const thrown = await client.whoami().catch((e: unknown) => e);
      expect(thrown).toBeInstanceOf(AgnetdClientError);
      expect((thrown as AgnetdClientError).code).toBe("NETWORK_ERROR");
    });
  });

  // ---------------------------------------------------------------------------
  // Method-specific tests
  // ---------------------------------------------------------------------------

  describe("getIdentity", () => {
    it("finds identity from list and calls whoami", async () => {
      const listData = [
        { did: "did:key:other", status: "active" },
        { did: "did:key:target", status: "active" },
      ];
      const whoamiData = { did: "did:key:target", method: "key", name: "target", ethereum: "0x1", peer_id: "peer1" };

      fetchMock
        .mockResolvedValueOnce(
          new Response(JSON.stringify({ data: listData, success: true }), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        )
        .mockResolvedValueOnce(
          new Response(JSON.stringify({ data: whoamiData, success: true }), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );

      const result = await client.getIdentity("did:key:target");
      expect(result).toEqual(whoamiData);
    });

    it("throws NOT_FOUND when DID is not in the list", async () => {
      const listData = [{ did: "did:key:other", status: "active" }];
      fetchMock.mockResolvedValue(
        new Response(JSON.stringify({ data: listData, success: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

      await expect(client.getIdentity("did:key:missing")).rejects.toThrow("not found");
    });
  });

  describe("AgnetdClientError", () => {
    it("has correct name and properties", () => {
      const err = new AgnetdClientError("test message", "TEST_CODE");
      expect(err.name).toBe("AgnetdClientError");
      expect(err.message).toBe("test message");
      expect(err.code).toBe("TEST_CODE");
      expect(err).toBeInstanceOf(Error);
    });
  });
});
