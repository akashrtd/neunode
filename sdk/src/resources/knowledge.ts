import type { NeunodeClient } from "../client/client.js";

export interface KnowledgeQueryParams {
	subject?: string;
	predicate?: string;
	object?: string;
	graph?: string;
	limit?: number;
}

export interface KnowledgeRegisterAgentParams {
	did: string;
	capabilities: string;
}

export interface KnowledgeRegisterModelParams {
	did: string;
	cid: string;
	parent?: string;
}

export interface KnowledgeRegisterBountyParams {
	id: string;
	capabilities: string;
}

export interface KnowledgeJoinJobParams {
	did: string;
	jobId: string;
}

export interface KnowledgeQueryResult {
	subject: string;
	predicate: string;
	object: string;
	graph: string;
}

export interface KnowledgeQueryListResult {
	data: KnowledgeQueryResult[];
}

export interface KnowledgeRegisterAgentResult {
	did: string;
	capabilities: string[];
	triples_inserted: number;
}

export interface KnowledgeRegisterModelResult {
	owner: string;
	cid: string;
	parent?: string;
	triples_inserted: number;
}

export interface KnowledgeRegisterBountyResult {
	id: string;
	required_capabilities: string[];
	triples_inserted: number;
}

export interface KnowledgeJoinJobResult {
	agent: string;
	job: string;
	triples_inserted: number;
}

export interface KnowledgeSchemaEntry {
	name: string;
	uri: string;
}

export interface KnowledgeListClassesResult {
	data: KnowledgeSchemaEntry[];
}

export interface KnowledgeListPredicatesResult {
	data: KnowledgeSchemaEntry[];
}

/** Knowledge graph management: query triples, register entities, inspect ontology. */
export interface KnowledgeResource {
	/** Query the knowledge graph with optional subject/predicate/object/graph filters. */
	query(params?: KnowledgeQueryParams): Promise<KnowledgeQueryListResult>;
	/** Register an agent with capabilities in the knowledge graph. */
	registerAgent(params: KnowledgeRegisterAgentParams): Promise<KnowledgeRegisterAgentResult>;
	/** Register a model (with optional lineage parent) in the knowledge graph. */
	registerModel(params: KnowledgeRegisterModelParams): Promise<KnowledgeRegisterModelResult>;
	/** Register a bounty with required capabilities in the knowledge graph. */
	registerBounty(params: KnowledgeRegisterBountyParams): Promise<KnowledgeRegisterBountyResult>;
	/** Join an agent to a training job in the knowledge graph. */
	joinJob(params: KnowledgeJoinJobParams): Promise<KnowledgeJoinJobResult>;
	/** List all ontology classes. */
	listClasses(): Promise<KnowledgeListClassesResult>;
	/** List all ontology predicates. */
	listPredicates(): Promise<KnowledgeListPredicatesResult>;
}

export function createKnowledgeResource(client: NeunodeClient): KnowledgeResource {
	const cli = client.cli;
	if (!cli) throw new Error("CLI transport required for knowledge operations");

	return {
		async query(params?: KnowledgeQueryParams): Promise<KnowledgeQueryListResult> {
			const args = ["knowledge", "query"];
			if (params?.subject) args.push("--subject", params.subject);
			if (params?.predicate) args.push("--predicate", params.predicate);
			if (params?.object) args.push("--object", params.object);
			if (params?.graph) args.push("--graph", params.graph);
			if (params?.limit) args.push("--limit", String(params.limit));
			return cli.execute<KnowledgeQueryListResult>(args);
		},

		async registerAgent(
			params: KnowledgeRegisterAgentParams,
		): Promise<KnowledgeRegisterAgentResult> {
			const args = [
				"knowledge",
				"register-agent",
				"--did",
				params.did,
				"--capabilities",
				params.capabilities,
			];
			return cli.execute<KnowledgeRegisterAgentResult>(args);
		},

		async registerModel(
			params: KnowledgeRegisterModelParams,
		): Promise<KnowledgeRegisterModelResult> {
			const args = [
				"knowledge",
				"register-model",
				"--did",
				params.did,
				"--cid",
				params.cid,
			];
			if (params.parent) args.push("--parent", params.parent);
			return cli.execute<KnowledgeRegisterModelResult>(args);
		},

		async registerBounty(
			params: KnowledgeRegisterBountyParams,
		): Promise<KnowledgeRegisterBountyResult> {
			const args = [
				"knowledge",
				"register-bounty",
				"--id",
				params.id,
				"--capabilities",
				params.capabilities,
			];
			return cli.execute<KnowledgeRegisterBountyResult>(args);
		},

		async joinJob(
			params: KnowledgeJoinJobParams,
		): Promise<KnowledgeJoinJobResult> {
			const args = [
				"knowledge",
				"join-job",
				"--did",
				params.did,
				"--job-id",
				params.jobId,
			];
			return cli.execute<KnowledgeJoinJobResult>(args);
		},

		async listClasses(): Promise<KnowledgeListClassesResult> {
			return cli.execute<KnowledgeListClassesResult>(["knowledge", "list-classes"]);
		},

		async listPredicates(): Promise<KnowledgeListPredicatesResult> {
			return cli.execute<KnowledgeListPredicatesResult>([
				"knowledge",
				"list-predicates",
			]);
		},
	};
}
