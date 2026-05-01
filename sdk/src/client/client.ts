import {
	type BountyResource,
	createBountyResource,
} from "../resources/bounty.js";
import {
	type ConfigResource,
	createConfigResource,
} from "../resources/config.js";
import {
	createDiscoveryResource,
	type DiscoveryResource,
} from "../resources/discovery.js";
import { createFeedResource, type FeedResource } from "../resources/feed.js";
import {
	createIdentityResource,
	type IdentityResource,
} from "../resources/identity.js";
import {
	createInferenceResource,
	type InferenceResource,
} from "../resources/inference.js";
import {
	createKnowledgeResource,
	type KnowledgeResource,
} from "../resources/knowledge.js";
import { createMeshResource, type MeshResource } from "../resources/mesh.js";
import { createModelResource, type ModelResource } from "../resources/model.js";
import {
	createReputationResource,
	type ReputationResource,
} from "../resources/reputation.js";
import { createTokenResource, type TokenResource } from "../resources/token.js";
import { createTrainResource, type TrainResource } from "../resources/train.js";
import {
	createTurboquantResource,
	type TurboquantResource,
} from "../resources/turboquant.js";
import type { CliTransportConfig } from "../transport/cli-transport.js";
import { CliTransport } from "../transport/cli-transport.js";
import type { ViemTransportConfig } from "../transport/viem-transport.js";
import { ViemTransport } from "../transport/viem-transport.js";

/** Configuration for creating a Neunode client. At least one transport must be provided. */
export interface NeunodeClientConfig {
	/** CLI subprocess transport config. Spawns `agnetd` as a child process. */
	readonly cli?: CliTransportConfig;
	/** Viem (Ethereum) transport config. Direct on-chain reads/writes via RPC. */
	readonly viem?: ViemTransportConfig;
}

/** Which transport(s) the client was configured with. */
export type TransportMode = "cli" | "viem" | "dual";

/** Root client for interacting with the Neunode network. */
export interface NeunodeClient {
	/** Which transport(s) are active. */
	readonly transportMode: TransportMode;
	/** CLI subprocess transport, if configured. */
	readonly cli: CliTransport | undefined;
	/** Viem on-chain transport, if configured. */
	readonly viem: ViemTransport | undefined;
	/** DID creation, listing, and export. */
	readonly identity: IdentityResource;
	/** Read and write local agent configuration. */
	readonly config: ConfigResource;
	/** Post events, list feeds, subscribe to topics. */
	readonly feed: FeedResource;
	/** P2P mesh status, peers, connect, disconnect. */
	readonly mesh: MeshResource;
	/** Push, list, inspect, and remove models. */
	readonly model: ModelResource;
	/** Start, monitor, and stop training jobs. */
	readonly train: TrainResource;
	/** Create, claim, submit, review, and cancel bounties. */
	readonly bounty: BountyResource;
	/** Balances, staking, transfers, and decay info. */
	readonly token: TokenResource;
	/** Reputation scores, attestations, leaderboard, and factor breakdowns. */
	readonly reputation: ReputationResource;
	/** Inference requests, model listing, provider discovery, routing, and pricing. */
	readonly inference: InferenceResource;
	/** Knowledge graph: query triples, register entities, inspect ontology. */
	readonly knowledge: KnowledgeResource;
	/** Agent discovery: search, complement analysis, capability gaps, scoring. */
	readonly discovery: DiscoveryResource;
	/** TurboQuant compression: adaptive strategy selection, codebook generation. */
	readonly turboquant: TurboquantResource;
	/** Attach custom properties to the client. Returns the merged object. */
	extend<T>(extender: (client: NeunodeClient) => T): NeunodeClient & T;
}

class NeunodeClientImpl implements NeunodeClient {
	readonly cli: CliTransport | undefined;
	readonly viem: ViemTransport | undefined;
	readonly identity: IdentityResource;
	readonly config: ConfigResource;
	readonly feed: FeedResource;
	readonly mesh: MeshResource;
	readonly model: ModelResource;
	readonly train: TrainResource;
	readonly bounty: BountyResource;
	readonly token: TokenResource;
	readonly reputation: ReputationResource;
	readonly inference: InferenceResource;
	readonly knowledge: KnowledgeResource;
	readonly discovery: DiscoveryResource;
	readonly turboquant: TurboquantResource;

	constructor(
		cliConfig?: CliTransportConfig,
		viemConfig?: ViemTransportConfig,
	) {
		if (!cliConfig && !viemConfig) {
			throw new Error(
				"NeunodeClient requires at least one transport (cli or viem). " +
				"Pass { cli: { ... } } or { viem: { ... } } to createNeunodeClient().",
			);
		}
		this.cli = cliConfig ? new CliTransport(cliConfig) : undefined;
		this.viem = viemConfig ? new ViemTransport(viemConfig) : undefined;
		this.identity = createIdentityResource(this);
		this.config = createConfigResource(this);
		this.feed = createFeedResource(this);
		this.mesh = createMeshResource(this);
		this.model = createModelResource(this);
		this.train = createTrainResource(this);
		this.bounty = createBountyResource(this);
		this.token = createTokenResource(this);
		this.reputation = createReputationResource(this);
		this.inference = createInferenceResource(this);
		this.knowledge = createKnowledgeResource(this);
		this.discovery = createDiscoveryResource(this);
		this.turboquant = createTurboquantResource(this);
	}

	get transportMode(): TransportMode {
		if (this.cli && this.viem) return "dual";
		if (this.viem) return "viem";
		return "cli";
	}

	extend<T>(extender: (client: NeunodeClient) => T): NeunodeClient & T {
		const extension = extender(this);
		const builtInKeys = new Set([
			"cli", "viem", "identity", "config", "feed", "mesh", "model",
			"train", "bounty", "token", "reputation", "inference",
			"knowledge", "discovery", "turboquant", "transportMode", "extend",
		]);
		const collisions = Object.keys(extension as Record<string, unknown>).filter((k) => builtInKeys.has(k));
		if (collisions.length > 0) {
			throw new Error(
				`extend() collision: keys [${collisions.join(", ")}] ` +
				"conflict with built-in client properties.",
			);
		}
		return Object.assign(this, extension);
	}
}

/**
 * Create a new Neunode client with one or both transports.
 *
 * @example
 * ```ts
 * import { createNeunodeClient } from "@neunode/sdk";
 *
 * // CLI-only (spawns agnetd subprocess)
 * const client = createNeunodeClient({
 *   cli: { binaryPath: "/usr/local/bin/agnetd" },
 * });
 *
 * // Dual transport (CLI + on-chain)
 * const client = createNeunodeClient({
 *   cli: { timeout: 60_000 },
 *   viem: { publicClient, chain, walletClient },
 * });
 *
 * // Then use any resource
 * const did = await client.identity.create({ name: "my-agent" });
 * ```
 *
 * @param config - Transport configuration. At least one of `cli` or `viem` is required.
 * @returns A typed Neunode client instance.
 */
export function createNeunodeClient(
	config: NeunodeClientConfig = {},
): NeunodeClient {
	return new NeunodeClientImpl(config.cli, config.viem);
}
