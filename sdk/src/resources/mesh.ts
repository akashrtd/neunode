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
  const cli = client.cli;
  if (!cli) throw new Error("CLI transport required for mesh operations");

  return {
    async status(): Promise<MeshStatusResult> {
      return cli.execute<MeshStatusResult>(["mesh", "status"]);
    },

    async peers(verbose?: boolean): Promise<MeshPeersResult> {
      const args = ["mesh", "peers"];
      if (verbose) args.push("--verbose");
      return cli.execute<MeshPeersResult>(args);
    },

    async connect(addr: string): Promise<MeshConnectResult> {
      return cli.execute<MeshConnectResult>(["mesh", "connect", "--addr", addr]);
    },

    async disconnect(peerId: string): Promise<MeshDisconnectResult> {
      return cli.execute<MeshDisconnectResult>(["mesh", "disconnect", "--peer-id", peerId]);
    },
  };
}
