import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createBountyResource } from "./bounty.js";

function makeMockClient(
	opts: { withHttp?: boolean; withCli?: boolean } = {},
): NeunodeClient {
	const execute = vi.fn();
	const transport = {
		execute,
		executeMulti: vi.fn(),
		executeRaw: vi.fn(),
	} as unknown as CliTransport;

	const httpGet = vi.fn();
	const httpPost = vi.fn();
	const httpTransport = {
		get: httpGet,
		post: httpPost,
		put: vi.fn(),
		delete: vi.fn(),
	} as unknown as HttpTransport;

	return {
		cli: opts.withHttp && !opts.withCli ? undefined : transport,
		http: opts.withHttp ? httpTransport : undefined,
		viem: undefined,
		transportMode: "cli",
		identity: {} as never,
		config: {} as never,
		feed: {} as never,
		mesh: {} as never,
		model: {} as never,
		train: {} as never,
		bounty: {} as never,
		token: {} as never,
		reputation: {} as never,
		inference: {} as never,
		knowledge: {} as never,
		discovery: {} as never,
		turboquant: {} as never,
		extend: vi.fn(),
	};
}

describe("createBountyResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both transports are missing", async () => {
		const resource = createBountyResource({
			...mockClient,
			cli: undefined,
			http: undefined,
		});
		await expect(resource.list()).rejects.toThrow(
			"HTTP or CLI transport required",
		);
	});

	describe("create", () => {
		it("should use HTTP transport when available", async () => {
			const expected = { id: "bnty_http", state: "Open" };
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue(expected);
			const resource = createBountyResource(dualClient);
			const result = await resource.create({
				title: "Train classifier",
				description: ">95% accuracy",
				reward: 1000,
				token: "nCompute",
			});
			expect(http.post).toHaveBeenCalledWith("/api/v1/bounties", {
				title: "Train classifier",
				description: ">95% accuracy",
				reward: 1000,
				token: "nCompute",
			});
			expect(result).toEqual(expected);
		});

		it("should call execute with required params only via CLI", async () => {
			execute.mockResolvedValue({ id: "bnty_123", state: "Open" });
			const resource = createBountyResource(mockClient);
			await resource.create({
				title: "Train classifier",
				description: ">95% accuracy",
				reward: 1000,
				token: "nCompute",
			});
			expect(execute).toHaveBeenCalledWith([
				"bounty",
				"create",
				"--title",
				"Train classifier",
				"--description",
				">95% accuracy",
				"--reward",
				"1000",
				"--token",
				"nCompute",
			]);
		});

		it("should pass --claim-deadline and --work-deadline when provided via CLI", async () => {
			execute.mockResolvedValue({ id: "bnty_123" });
			const resource = createBountyResource(mockClient);
			await resource.create({
				title: "test",
				description: "desc",
				reward: 500,
				token: "nTrain",
				claimDeadline: 86400,
				workDeadline: 259200,
			});
			expect(execute).toHaveBeenCalledWith([
				"bounty",
				"create",
				"--title",
				"test",
				"--description",
				"desc",
				"--reward",
				"500",
				"--token",
				"nTrain",
				"--claim-deadline",
				"86400",
				"--work-deadline",
				"259200",
			]);
		});
	});

	describe("claim", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({ bounty_id: "bnty_123", state: "Claimed" });
			const resource = createBountyResource(dualClient);
			await resource.claim({ id: "bnty_123", stake: 50 });
			expect(http.post).toHaveBeenCalledWith(
				"/api/v1/bounties/bnty_123/claim",
				{ stake: 50 },
			);
		});

		it("should call execute with bounty claim --id --stake via CLI", async () => {
			execute.mockResolvedValue({
				bounty_id: "bnty_123",
				claimant: "did:neunode:abc",
				state: "Claimed",
			});
			const resource = createBountyResource(mockClient);
			await resource.claim({ id: "bnty_123", stake: 50 });
			expect(execute).toHaveBeenCalledWith([
				"bounty",
				"claim",
				"--id",
				"bnty_123",
				"--stake",
				"50",
			]);
		});
	});

	describe("submit", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({
				bounty_id: "bnty_123",
				state: "Submitted",
			});
			const resource = createBountyResource(dualClient);
			await resource.submit({ id: "bnty_123", artifact: "ipfs://QmX7b" });
			expect(http.post).toHaveBeenCalledWith(
				"/api/v1/bounties/bnty_123/submit",
				{ id: "bnty_123", artifact: "ipfs://QmX7b" },
			);
		});

		it("should call execute with bounty submit --id --artifact via CLI", async () => {
			execute.mockResolvedValue({ bounty_id: "bnty_123", state: "Submitted" });
			const resource = createBountyResource(mockClient);
			await resource.submit({ id: "bnty_123", artifact: "ipfs://QmX7b" });
			expect(execute).toHaveBeenCalledWith([
				"bounty",
				"submit",
				"--id",
				"bnty_123",
				"--artifact",
				"ipfs://QmX7b",
			]);
		});

		it("should pass --evidence when provided via CLI", async () => {
			execute.mockResolvedValue({ bounty_id: "bnty_123" });
			const resource = createBountyResource(mockClient);
			await resource.submit({
				id: "bnty_123",
				artifact: "ipfs://QmX7b",
				evidence: '{"acc":0.96}',
			});
			expect(execute).toHaveBeenCalledWith([
				"bounty",
				"submit",
				"--id",
				"bnty_123",
				"--artifact",
				"ipfs://QmX7b",
				"--evidence",
				'{"acc":0.96}',
			]);
		});
	});

	describe("review", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({ bounty_id: "bnty_123", score: 9 });
			const resource = createBountyResource(dualClient);
			await resource.review({
				id: "bnty_123",
				score: 9,
				feedback: "Great work",
			});
			expect(http.post).toHaveBeenCalledWith(
				"/api/v1/bounties/bnty_123/review",
				{ id: "bnty_123", score: 9, feedback: "Great work" },
			);
		});

		it("should call execute with bounty review --id --score --feedback via CLI", async () => {
			execute.mockResolvedValue({
				bounty_id: "bnty_123",
				score: 9,
				state: "UnderReview",
			});
			const resource = createBountyResource(mockClient);
			await resource.review({
				id: "bnty_123",
				score: 9,
				feedback: "Great work",
			});
			expect(execute).toHaveBeenCalledWith([
				"bounty",
				"review",
				"--id",
				"bnty_123",
				"--score",
				"9",
				"--feedback",
				"Great work",
			]);
		});
	});

	describe("list", () => {
		it("should use HTTP transport with query params", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue([]);
			const resource = createBountyResource(dualClient);
			await resource.list({ state: "Open", limit: 10 });
			expect(http.get).toHaveBeenCalled();
			const callUrl = http.get.mock.calls[0]?.[0] as string;
			expect(callUrl).toContain("/api/v1/bounties?");
			expect(callUrl).toContain("state=Open");
			expect(callUrl).toContain("limit=10");
		});

		it("should call execute with bounty list (no params) via CLI", async () => {
			execute.mockResolvedValue([]);
			const resource = createBountyResource(mockClient);
			const result = await resource.list();
			expect(execute).toHaveBeenCalledWith(["bounty", "list"]);
			expect(result).toEqual([]);
		});

		it("should pass optional filter params via CLI", async () => {
			const mockItems = [
				{
					ID: "bnty_1",
					State: "Open",
					Creator: "did:neunode:abc",
					Claimant: "",
					Reward: "1000",
					Deadline: "",
					Created: "",
					Escrow: "",
				},
			];
			execute.mockResolvedValue(mockItems);
			const resource = createBountyResource(mockClient);
			const result = await resource.list({
				state: "Open",
				creator: "did:neunode:abc",
				limit: 10,
			});
			expect(execute).toHaveBeenCalledWith([
				"bounty",
				"list",
				"--state",
				"Open",
				"--creator",
				"did:neunode:abc",
				"--limit",
				"10",
			]);
			expect(result).toEqual(mockItems);
		});
	});

	describe("show", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ ID: "bnty_123", State: "Open" });
			const resource = createBountyResource(dualClient);
			await resource.show("bnty_123");
			expect(http.get).toHaveBeenCalledWith("/api/v1/bounties/bnty_123");
		});

		it("should call execute with bounty show --id via CLI", async () => {
			execute.mockResolvedValue({
				ID: "bnty_123",
				State: "Open",
				Creator: "did:neunode:abc",
			});
			const resource = createBountyResource(mockClient);
			await resource.show("bnty_123");
			expect(execute).toHaveBeenCalledWith([
				"bounty",
				"show",
				"--id",
				"bnty_123",
			]);
		});
	});

	describe("cancel", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({
				bounty_id: "bnty_123",
				state: "Cancelled",
			});
			const resource = createBountyResource(dualClient);
			await resource.cancel("bnty_123", "no longer needed");
			expect(http.post).toHaveBeenCalledWith(
				"/api/v1/bounties/bnty_123/cancel",
				{ reason: "no longer needed" },
			);
		});

		it("should call execute with bounty cancel --id (no reason) via CLI", async () => {
			execute.mockResolvedValue({ bounty_id: "bnty_123", state: "Cancelled" });
			const resource = createBountyResource(mockClient);
			await resource.cancel("bnty_123");
			expect(execute).toHaveBeenCalledWith([
				"bounty",
				"cancel",
				"--id",
				"bnty_123",
			]);
		});

		it("should pass --reason when provided via CLI", async () => {
			execute.mockResolvedValue({ bounty_id: "bnty_123", state: "Cancelled" });
			const resource = createBountyResource(mockClient);
			await resource.cancel("bnty_123", "no longer needed");
			expect(execute).toHaveBeenCalledWith([
				"bounty",
				"cancel",
				"--id",
				"bnty_123",
				"--reason",
				"no longer needed",
			]);
		});
	});
});
