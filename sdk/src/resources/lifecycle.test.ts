import { describe, expect, it } from "vitest";
import { createNeunodeClient } from "../client/client.js";
import type { MockTransport } from "../transport/mock-transport.js";

describe("LifecycleResource", () => {
	it("routes every lifecycle operation through HTTP", async () => {
		const client = createNeunodeClient({
			mock: {
				responses: {
					"GET /api/v1/lifecycle/status": { message: "not activated" },
					"POST /api/v1/lifecycle/activate": { message: "activated" },
					"POST /api/v1/lifecycle/hibernate": { message: "hibernating" },
					"POST /api/v1/lifecycle/reactivate": { message: "reactivated" },
					"GET /api/v1/lifecycle/list": [],
					"POST /api/v1/lifecycle/reap": { transitions: [], count: 0 },
				},
			},
		});
		const mock = client.http as MockTransport;

		expect(await client.lifecycle.status()).toEqual({
			message: "not activated",
		});
		expect(await client.lifecycle.activate()).toEqual({ message: "activated" });
		expect(await client.lifecycle.hibernate()).toEqual({
			message: "hibernating",
		});
		expect(await client.lifecycle.reactivate()).toEqual({
			message: "reactivated",
		});
		expect(await client.lifecycle.list()).toEqual([]);
		expect(await client.lifecycle.reap()).toEqual({
			transitions: [],
			count: 0,
		});
		expect(
			mock.requests.map(({ method, path }) => `${method} ${path}`),
		).toEqual([
			"GET /api/v1/lifecycle/status",
			"POST /api/v1/lifecycle/activate",
			"POST /api/v1/lifecycle/hibernate",
			"POST /api/v1/lifecycle/reactivate",
			"GET /api/v1/lifecycle/list",
			"POST /api/v1/lifecycle/reap",
		]);
	});
});
