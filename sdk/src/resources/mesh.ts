import type { NeunodeClient } from "../client/client.js";

export interface MeshStatusResult {
	Status: string;
	"Peer ID": string;
	Listeners: string;
	"Connected Peers": string;
	"Subscribed Topics": string;
}

export interface MeshPeersResult {
	data: Array<{ "Peer ID": string }>;
}

export interface MeshConnectResult {
	action: string;
	address: string;
	status: string;
}

export interface MeshDisconnectResult {
	action: string;
	peer_id: string;
	status: string;
}

/** P2P mesh network operations. */
export interface MeshResource {
	/** Get the current mesh connection status. */
	status(): Promise<MeshStatusResult>;
	/** List connected peers. */
	peers(verbose?: boolean): Promise<MeshPeersResult>;
	/** Connect to a peer by multiaddr. */
	connect(addr: string): Promise<MeshConnectResult>;
	/** Disconnect from a peer by its Peer ID. */
	disconnect(peerId: string): Promise<MeshDisconnectResult>;
}

export function createMeshResource(client: NeunodeClient): MeshResource {
	return {
		async status(): Promise<MeshStatusResult> {
			return client.http.get<MeshStatusResult>("/api/v1/mesh/status");
		},

		async peers(verbose?: boolean): Promise<MeshPeersResult> {
			const qs = new URLSearchParams();
			if (verbose) qs.set("verbose", "true");
			const query = qs.toString();
			return client.http.get<MeshPeersResult>(
				query ? `/api/v1/mesh/peers?${query}` : "/api/v1/mesh/peers",
			);
		},

		async connect(addr: string): Promise<MeshConnectResult> {
			return client.http.post<MeshConnectResult>("/api/v1/mesh/connect", {
				addr,
			});
		},

		async disconnect(peerId: string): Promise<MeshDisconnectResult> {
			return client.http.post<MeshDisconnectResult>("/api/v1/mesh/disconnect", {
				peerId,
			});
		},
	};
}
