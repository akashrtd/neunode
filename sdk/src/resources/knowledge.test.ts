import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createKnowledgeResource } from "./knowledge.js";

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
		lifecycle: {} as never,
		lineage: {} as never,
		extend: vi.fn(),
	};
}

describe("createKnowledgeResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if both transports are missing", async () => {
		const resource = createKnowledgeResource({
			...mockClient,
			cli: undefined,
			http: undefined,
		});
		await expect(resource.listClasses()).rejects.toThrow(
			"HTTP or CLI transport required",
		);
	});

	describe("query", () => {
		it("should use HTTP transport with query params", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createKnowledgeResource(dualClient);
			await resource.query({ subject: "did:neunode:abc", limit: 10 });
			expect(http.get).toHaveBeenCalled();
			const callUrl = http.get.mock.calls[0]?.[0] as string;
			expect(callUrl).toContain("/api/v1/knowledge/query?");
			expect(callUrl).toContain("subject=did%3Aneunode%3Aabc");
			expect(callUrl).toContain("limit=10");
		});

		it("should call execute with knowledge query (no params) via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createKnowledgeResource(mockClient);
			await resource.query();
			expect(execute).toHaveBeenCalledWith(["knowledge", "query"]);
		});

		it("should pass all filter params when provided via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createKnowledgeResource(mockClient);
			await resource.query({
				subject: "did:neunode:abc",
				predicate: "knows",
				object: "did:neunode:def",
				graph: "social",
				limit: 10,
			});
			expect(execute).toHaveBeenCalledWith([
				"knowledge",
				"query",
				"--subject",
				"did:neunode:abc",
				"--predicate",
				"knows",
				"--object",
				"did:neunode:def",
				"--graph",
				"social",
				"--limit",
				"10",
			]);
		});
	});

	describe("registerAgent", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({
				did: "did:neunode:test",
				triples_inserted: 3,
			});
			const resource = createKnowledgeResource(dualClient);
			await resource.registerAgent({
				did: "did:neunode:test",
				capabilities: "NLP,Vision",
			});
			expect(http.post).toHaveBeenCalledWith(
				"/api/v1/knowledge/register-agent",
				{
					did: "did:neunode:test",
					capabilities: "NLP,Vision",
				},
			);
		});

		it("should call execute with knowledge register-agent args via CLI", async () => {
			execute.mockResolvedValue({
				did: "did:neunode:test",
				capabilities: ["NLP", "Vision"],
				triples_inserted: 3,
			});
			const resource = createKnowledgeResource(mockClient);
			await resource.registerAgent({
				did: "did:neunode:test",
				capabilities: "NLP,Vision",
			});
			expect(execute).toHaveBeenCalledWith([
				"knowledge",
				"register-agent",
				"--did",
				"did:neunode:test",
				"--capabilities",
				"NLP,Vision",
			]);
		});
	});

	describe("registerModel", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({
				owner: "did:neunode:dev",
				cid: "ipfs://QmModel",
			});
			const resource = createKnowledgeResource(dualClient);
			await resource.registerModel({
				did: "did:neunode:dev",
				cid: "ipfs://QmModel",
			});
			expect(http.post).toHaveBeenCalledWith(
				"/api/v1/knowledge/register-model",
				{
					did: "did:neunode:dev",
					cid: "ipfs://QmModel",
				},
			);
		});

		it("should call execute with knowledge register-model args via CLI", async () => {
			execute.mockResolvedValue({
				owner: "did:neunode:dev",
				cid: "ipfs://QmModel",
				triples_inserted: 2,
			});
			const resource = createKnowledgeResource(mockClient);
			await resource.registerModel({
				did: "did:neunode:dev",
				cid: "ipfs://QmModel",
			});
			expect(execute).toHaveBeenCalledWith([
				"knowledge",
				"register-model",
				"--did",
				"did:neunode:dev",
				"--cid",
				"ipfs://QmModel",
			]);
		});

		it("should pass --parent when provided via CLI", async () => {
			execute.mockResolvedValue({
				owner: "did:neunode:dev",
				cid: "ipfs://QmChild",
				parent: "ipfs://QmParent",
				triples_inserted: 3,
			});
			const resource = createKnowledgeResource(mockClient);
			await resource.registerModel({
				did: "did:neunode:dev",
				cid: "ipfs://QmChild",
				parent: "ipfs://QmParent",
			});
			expect(execute).toHaveBeenCalledWith([
				"knowledge",
				"register-model",
				"--did",
				"did:neunode:dev",
				"--cid",
				"ipfs://QmChild",
				"--parent",
				"ipfs://QmParent",
			]);
		});
	});

	describe("registerBounty", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({ id: "bounty:42", triples_inserted: 3 });
			const resource = createKnowledgeResource(dualClient);
			await resource.registerBounty({
				id: "bounty:42",
				capabilities: "NLP,RLHF",
			});
			expect(http.post).toHaveBeenCalledWith(
				"/api/v1/knowledge/register-bounty",
				{
					id: "bounty:42",
					capabilities: "NLP,RLHF",
				},
			);
		});

		it("should call execute with knowledge register-bounty args via CLI", async () => {
			execute.mockResolvedValue({
				id: "bounty:42",
				required_capabilities: ["NLP", "RLHF"],
				triples_inserted: 3,
			});
			const resource = createKnowledgeResource(mockClient);
			await resource.registerBounty({
				id: "bounty:42",
				capabilities: "NLP,RLHF",
			});
			expect(execute).toHaveBeenCalledWith([
				"knowledge",
				"register-bounty",
				"--id",
				"bounty:42",
				"--capabilities",
				"NLP,RLHF",
			]);
		});
	});

	describe("joinJob", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				post: ReturnType<typeof vi.fn>;
			};
			http.post.mockResolvedValue({
				agent: "did:neunode:worker",
				job: "job:101",
				triples_inserted: 1,
			});
			const resource = createKnowledgeResource(dualClient);
			await resource.joinJob({ did: "did:neunode:worker", jobId: "job:101" });
			expect(http.post).toHaveBeenCalledWith("/api/v1/knowledge/join-job", {
				did: "did:neunode:worker",
				jobId: "job:101",
			});
		});

		it("should call execute with knowledge join-job args via CLI", async () => {
			execute.mockResolvedValue({
				agent: "did:neunode:worker",
				job: "job:101",
				triples_inserted: 1,
			});
			const resource = createKnowledgeResource(mockClient);
			await resource.joinJob({
				did: "did:neunode:worker",
				jobId: "job:101",
			});
			expect(execute).toHaveBeenCalledWith([
				"knowledge",
				"join-job",
				"--did",
				"did:neunode:worker",
				"--job-id",
				"job:101",
			]);
		});
	});

	describe("listClasses", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createKnowledgeResource(dualClient);
			await resource.listClasses();
			expect(http.get).toHaveBeenCalledWith("/api/v1/knowledge/classes");
		});

		it("should call execute with knowledge list-classes via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createKnowledgeResource(mockClient);
			await resource.listClasses();
			expect(execute).toHaveBeenCalledWith(["knowledge", "list-classes"]);
		});
	});

	describe("listPredicates", () => {
		it("should use HTTP transport", async () => {
			const dualClient = makeMockClient({ withHttp: true, withCli: true });
			const http = dualClient.http as unknown as {
				get: ReturnType<typeof vi.fn>;
			};
			http.get.mockResolvedValue({ data: [] });
			const resource = createKnowledgeResource(dualClient);
			await resource.listPredicates();
			expect(http.get).toHaveBeenCalledWith("/api/v1/knowledge/predicates");
		});

		it("should call execute with knowledge list-predicates via CLI", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createKnowledgeResource(mockClient);
			await resource.listPredicates();
			expect(execute).toHaveBeenCalledWith(["knowledge", "list-predicates"]);
		});
	});
});
