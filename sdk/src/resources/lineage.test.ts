import { describe, expect, it } from "vitest";
import { createNeunodeClient } from "../client/client.js";
import type { MockTransport } from "../transport/mock-transport.js";
import type { CID } from "../types/core.js";

describe("LineageResource", () => {
	it("encodes CIDs, parents, and request field names for HTTP", async () => {
		const cid = "sha256:aa/bb" as CID;
		const parent = "sha256:parent" as CID;
		const client = createNeunodeClient({
			mock: {
				handler: ({ path }) => {
					if (path === "/api/v1/lineage/register") return { cid };
					if (path.endsWith("/royalties")) return [];
					if (path === "/api/v1/lineage/hash") return { hash: "abc" };
					if (path === "/api/v1/lineage/verify") return { verified: true };
					return { cid };
				},
			},
		});
		const mock = client.http as MockTransport;

		await client.lineage.register({
			cid,
			parents: [parent],
			contributionType: "fine_tune",
			loraRank: 16,
			loraAlpha: 32,
		});
		await client.lineage.show(cid);
		await client.lineage.parents(cid);
		await client.lineage.children(cid);
		await client.lineage.ancestors(cid);
		await client.lineage.depth(cid);
		await client.lineage.royalties(cid, 10_000);
		await client.lineage.hash("/tmp/model.bin");
		await client.lineage.verify(cid);

		expect(mock.requests[0]?.body).toEqual({
			cid,
			parents: parent,
			contribution_type: "fine_tune",
			lora_rank: 16,
			lora_alpha: 32,
		});
		expect(mock.requests[1]?.path).toBe("/api/v1/lineage/sha256%3Aaa%2Fbb");
		expect(mock.requests[6]?.body).toEqual({ amount: 10_000 });
		expect(mock.requests[8]?.body).toEqual({ cid });
	});

	it("rejects lineage calls without HTTP", async () => {
		const client = createNeunodeClient({ cli: { binaryPath: "agnetd" } });
		await expect(client.lineage.show("sha256:a" as CID)).rejects.toThrow(
			"HTTP transport required for lineage operations",
		);
	});
});
