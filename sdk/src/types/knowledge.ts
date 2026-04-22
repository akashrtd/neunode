// @neunode/sdk — Knowledge graph types mirroring neunode-knowledge crate
// (triple, ontology, query, mutations, dictionary)

import type { Did, CID, BountyId, JobId } from "./core.js";

// ─── Ontology Classes ─────────────────────────────────────────────────────────────

/** Entity classes in the knowledge graph ontology. */
export const KgClass = {
	Agent: "Agent",
	Model: "Model",
	Bounty: "Bounty",
	TrainingJob: "TrainingJob",
	Capability: "Capability",
	Knowledge: "Knowledge",
} as const;

export type KgClass = (typeof KgClass)[keyof typeof KgClass];

// ─── Ontology Predicates ──────────────────────────────────────────────────────────

/** Relationship predicates in the knowledge graph ontology. */
export const KgPredicate = {
	HasCapability: "hasCapability",
	OwnsModel: "ownsModel",
	CreatedBounty: "createdBounty",
	ClaimedBounty: "claimedBounty",
	ParticipatesIn: "participatesIn",
	TrainedModel: "trainedModel",
	RequiresCapability: "requiresCapability",
	DependsOn: "dependsOn",
	Knows: "knows",
	ContributedTo: "contributedTo",
	Type: "type",
} as const;

export type KgPredicate = (typeof KgPredicate)[keyof typeof KgPredicate];

// ─── Query Pattern ────────────────────────────────────────────────────────────────

/** A query pattern for the knowledge graph. Bound components filter results;
 *  unbound components act as wildcards. At least one component must be set. */
export interface KgQueryPattern {
	readonly subject?: string;
	readonly predicate?: string;
	readonly object?: string;
	readonly graph?: string;
}

// ─── Query Result ─────────────────────────────────────────────────────────────────

/** A resolved knowledge graph quad with human-readable string components. */
export interface KgQueryResult {
	readonly subject: string;
	readonly predicate: string;
	readonly object: string;
	readonly graph: string;
}

// ─── Registration Results ─────────────────────────────────────────────────────────

/** Result of registering an agent in the knowledge graph. */
export interface KgRegisterAgentResult {
	readonly did: string;
	readonly capabilities: readonly string[];
	readonly triples_inserted: number;
}

/** Result of registering a model in the knowledge graph. */
export interface KgRegisterModelResult {
	readonly owner: string;
	readonly cid: string;
	readonly parent?: string;
	readonly triples_inserted: number;
}

/** Result of registering a bounty in the knowledge graph. */
export interface KgRegisterBountyResult {
	readonly id: string;
	readonly required_capabilities: readonly string[];
	readonly triples_inserted: number;
}

/** Result of joining a training job in the knowledge graph. */
export interface KgJoinJobResult {
	readonly agent: string;
	readonly job: string;
	readonly triples_inserted: number;
}

// ─── Schema Listing ───────────────────────────────────────────────────────────────

/** An ontology class or predicate with its full URI. */
export interface KgSchemaEntry {
	readonly name: string;
	readonly uri: string;
}
