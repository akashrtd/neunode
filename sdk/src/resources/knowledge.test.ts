import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { CliTransport } from "../transport/cli-transport.js";
import { createKnowledgeResource } from "./knowledge.js";

function makeMockClient(): NeunodeClient {
	const execute = vi.fn();
	const transport = {
		execute,
		executeMulti: vi.fn(),
		executeRaw: vi.fn(),
	} as unknown as CliTransport;
	return {
		cli: transport,
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

describe("createKnowledgeResource", () => {
	let mockClient: NeunodeClient;
	let execute: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockClient = makeMockClient();
		execute = mockClient.cli?.execute as ReturnType<typeof vi.fn>;
	});

	it("should throw if cli transport is missing", () => {
		expect(() =>
			createKnowledgeResource({ ...mockClient, cli: undefined }),
		).toThrow("CLI transport required");
	});

	describe("query", () => {
		it("should call execute with knowledge query (no params)", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createKnowledgeResource(mockClient);
			await resource.query();
			expect(execute).toHaveBeenCalledWith(["knowledge", "query"]);
		});

		it("should pass all filter params when provided", async () => {
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
		it("should call execute with knowledge register-agent args", async () => {
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
		it("should call execute with knowledge register-model args", async () => {
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

		it("should pass --parent when provided", async () => {
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
		it("should call execute with knowledge register-bounty args", async () => {
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
		it("should call execute with knowledge join-job args", async () => {
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
		it("should call execute with knowledge list-classes", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createKnowledgeResource(mockClient);
			await resource.listClasses();
			expect(execute).toHaveBeenCalledWith(["knowledge", "list-classes"]);
		});
	});

	describe("listPredicates", () => {
		it("should call execute with knowledge list-predicates", async () => {
			execute.mockResolvedValue({ data: [] });
			const resource = createKnowledgeResource(mockClient);
			await resource.listPredicates();
			expect(execute).toHaveBeenCalledWith(["knowledge", "list-predicates"]);
		});
	});
});
