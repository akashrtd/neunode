// @neunode/sdk — P2P types mirroring neunode-p2p (peer_score, node)

import type { PeerId } from "./core.js";

export interface PeerScoreParams {
	readonly p1_weight: number;
	readonly p2_weight: number;
	readonly p3_weight: number;
	readonly p4_weight: number;
	readonly p5_weight: number;
	readonly p6_weight: number;
	readonly p7_weight: number;
	readonly p1_cap: number;
	readonly p2_cap: number;
	readonly p3_cap: number;
	readonly p4_cap: number;
	readonly p5_cap: number;
	readonly p6_cap: number;
	readonly p7_cap: number;
	readonly graylist_threshold: number;
	readonly publish_threshold: number;
	readonly gossip_threshold: number;
	readonly decay_interval_secs: number;
	readonly decay_to_zero: number;
	readonly retain_score_secs: number;
}

export type NodeEvent =
	| {
			readonly type: "GossipsubMessage";
			readonly source?: PeerId;
			readonly topic: string;
			readonly data: Uint8Array;
	  }
	| { readonly type: "PeerConnected"; readonly peer_id: PeerId }
	| { readonly type: "PeerDisconnected"; readonly peer_id: PeerId }
	| {
			readonly type: "IdentifyReceived";
			readonly peer_id: PeerId;
			readonly agent_version: string;
	  }
	| {
			readonly type: "PingResult";
			readonly peer_id: PeerId;
			readonly rtt_ms: number;
	  };
