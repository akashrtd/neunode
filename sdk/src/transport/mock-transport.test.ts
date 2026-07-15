import { describe, expect, it } from "vitest";
import { createNeunodeClient } from "../client/client.js";
import { MockTransport, MockTransportError } from "./mock-transport.js";

describe("MockTransport", () => {
	it("returns canned responses and records requests", async () => {
		const transport = new MockTransport({
			responses: { "post /api/v1/example": { id: "response-1" } },
		});

		await expect(
			transport.post<{ id: string }>("/api/v1/example", { prompt: "hello" }),
		).resolves.toEqual({ id: "response-1" });
		expect(transport.requests).toEqual([
			{
				method: "POST",
				path: "/api/v1/example",
				body: { prompt: "hello" },
			},
		]);
	});

	it("uses a dynamic handler when no canned response exists", async () => {
		const transport = new MockTransport({
			handler: (request) => ({ route: `${request.method} ${request.path}` }),
		});

		await expect(transport.get("/dynamic")).resolves.toEqual({
			route: "GET /dynamic",
		});
	});

	it("fails clearly for an unconfigured route in strict mode", async () => {
		const transport = new MockTransport();
		const error = await transport
			.get("/missing")
			.catch((reason: unknown) => reason);

		expect(error).toBeInstanceOf(MockTransportError);
		expect(error).toMatchObject({
			message: "No mock response configured for GET /missing",
			request: { method: "GET", path: "/missing" },
		});
	});

	it("allows responses to be replaced and state to be reset", async () => {
		const transport = new MockTransport();
		transport.setResponse("GET", "/status", { ready: true });
		await expect(transport.get("/status")).resolves.toEqual({ ready: true });

		transport.reset();
		expect(transport.requests).toHaveLength(0);
		await expect(transport.get("/status")).rejects.toBeInstanceOf(
			MockTransportError,
		);
	});

	it("drives SDK resources without agnetd", async () => {
		const client = createNeunodeClient({
			mock: {
				responses: {
					"GET /api/v1/identity/list": {
						data: [{ DID: "did:neunode:test", Status: "active" }],
					},
				},
			},
		});

		expect(client.transportMode).toBe("mock");
		await expect(client.identity.list()).resolves.toEqual({
			data: [{ DID: "did:neunode:test", Status: "active" }],
		});
	});

	it("rejects ambiguous HTTP and mock configuration", () => {
		expect(() =>
			createNeunodeClient({
				http: { baseUrl: "http://127.0.0.1:41000" },
				mock: {},
			}),
		).toThrow("cannot use http and mock transports together");
	});
});
