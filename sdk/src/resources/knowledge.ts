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
	registerAgent(
		params: KnowledgeRegisterAgentParams,
	): Promise<KnowledgeRegisterAgentResult>;
	/** Register a model (with optional lineage parent) in the knowledge graph. */
	registerModel(
		params: KnowledgeRegisterModelParams,
	): Promise<KnowledgeRegisterModelResult>;
	/** Register a bounty with required capabilities in the knowledge graph. */
	registerBounty(
		params: KnowledgeRegisterBountyParams,
	): Promise<KnowledgeRegisterBountyResult>;
	/** Join an agent to a training job in the knowledge graph. */
	joinJob(params: KnowledgeJoinJobParams): Promise<KnowledgeJoinJobResult>;
	/** List all ontology classes. */
	listClasses(): Promise<KnowledgeListClassesResult>;
	/** List all ontology predicates. */
	listPredicates(): Promise<KnowledgeListPredicatesResult>;
}

export function createKnowledgeResource(
	client: NeunodeClient,
): KnowledgeResource {
	return {
		async query(
			params?: KnowledgeQueryParams,
		): Promise<KnowledgeQueryListResult> {
			const qs = new URLSearchParams();
			if (params?.subject) qs.set("subject", params.subject);
			if (params?.predicate) qs.set("predicate", params.predicate);
			if (params?.object) qs.set("object", params.object);
			if (params?.graph) qs.set("graph", params.graph);
			if (params?.limit) qs.set("limit", String(params.limit));
			const query = qs.toString();
			return client.http.get<KnowledgeQueryListResult>(
				query ? `/api/v1/knowledge/query?${query}` : "/api/v1/knowledge/query",
			);
		},

		async registerAgent(
			params: KnowledgeRegisterAgentParams,
		): Promise<KnowledgeRegisterAgentResult> {
			return client.http.post<KnowledgeRegisterAgentResult>(
				"/api/v1/knowledge/register-agent",
				params,
			);
		},

		async registerModel(
			params: KnowledgeRegisterModelParams,
		): Promise<KnowledgeRegisterModelResult> {
			return client.http.post<KnowledgeRegisterModelResult>(
				"/api/v1/knowledge/register-model",
				params,
			);
		},

		async registerBounty(
			params: KnowledgeRegisterBountyParams,
		): Promise<KnowledgeRegisterBountyResult> {
			return client.http.post<KnowledgeRegisterBountyResult>(
				"/api/v1/knowledge/register-bounty",
				params,
			);
		},

		async joinJob(
			params: KnowledgeJoinJobParams,
		): Promise<KnowledgeJoinJobResult> {
			return client.http.post<KnowledgeJoinJobResult>(
				"/api/v1/knowledge/join-job",
				params,
			);
		},

		async listClasses(): Promise<KnowledgeListClassesResult> {
			return client.http.get<KnowledgeListClassesResult>(
				"/api/v1/knowledge/classes",
			);
		},

		async listPredicates(): Promise<KnowledgeListPredicatesResult> {
			return client.http.get<KnowledgeListPredicatesResult>(
				"/api/v1/knowledge/predicates",
			);
		},
	};
}
