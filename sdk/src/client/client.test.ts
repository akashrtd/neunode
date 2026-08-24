import type { Chain, PublicClient } from "viem";
import { describe, expect, it } from "vitest";
import { createNeunodeClient } from "./client.js";

const publicClient = {
	readContract: () => Promise.resolve(),
} as unknown as PublicClient;
const chain = { id: 31337, name: "Anvil" } as unknown as Chain;

describe("createNeunodeClient", () => {
	it("creates an HTTP client", () => {
		const client = createNeunodeClient({
			http: { baseUrl: "http://127.0.0.1:41000/" },
		});
		expect(client.transportMode).toBe("http");
		expect(client.http.getBaseUrl()).toBe("http://127.0.0.1:41000");
	});

	it("supports the in-memory HTTP-compatible mock", () => {
		const client = createNeunodeClient({ mock: {} });
		expect(client.transportMode).toBe("mock");
		expect(client.http).toBeDefined();
	});

	it("supports HTTP plus optional on-chain access", () => {
		const client = createNeunodeClient({
			http: { baseUrl: "http://127.0.0.1:41000" },
			viem: { publicClient, chain },
		});
		expect(client.transportMode).toBe("dual");
		expect(client.viem).toBeDefined();
	});

	it("rejects configurations without an HTTP-compatible transport", () => {
		expect(() => createNeunodeClient()).toThrow("HTTP-compatible transport");
		expect(() =>
			createNeunodeClient({ viem: { publicClient, chain } }),
		).toThrow("HTTP-compatible transport");
	});

	it("exposes every built-in resource", () => {
		const client = createNeunodeClient({ mock: {} });
		for (const key of [
			"identity",
			"config",
			"feed",
			"mesh",
			"model",
			"train",
			"bounty",
			"token",
			"reputation",
			"inference",
			"knowledge",
			"discovery",
			"turboquant",
			"lifecycle",
			"lineage",
			"verification",
		] as const) {
			expect(client[key]).toBeDefined();
		}
	});

	it("extends the client without overwriting built-ins", () => {
		const client = createNeunodeClient({ mock: {} });
		expect(client.extend(() => ({ custom: 42 })).custom).toBe(42);
		expect(() => client.extend(() => ({ feed: "collision" }))).toThrow(
			"collision",
		);
	});
});
