import { spawn, type ChildProcess } from "node:child_process";
import {
  createPublicClient,
  createWalletClient,
  createTestClient,
  http,
} from "viem";
import { foundry } from "viem/chains";
import { privateKeyToAccount } from "viem/accounts";
import type {
  PublicClient,
  WalletClient,
  TestClient,
  Transport,
} from "viem";

export const ANVIL_HOST = "127.0.0.1";
export const ANVIL_PORT = 8545;
export const ANVIL_RPC_URL = `http://${ANVIL_HOST}:${ANVIL_PORT}`;

/** Anvil account 0 — default deployer */
export const DEFAULT_PRIVATE_KEY =
  "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" as const;

/** Well-known Anvil addresses */
export const ANVIL_ACCOUNTS = {
  0: "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
  1: "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
  2: "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
  3: "0x90F79bf6EB2c4f870365E785982E1f101E93b906",
  4: "0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65",
} as const;

export interface AnvilInstance {
  process: ChildProcess;
  rpcUrl: string;
}

/**
 * Start an Anvil local node.
 * Returns the child process and the RPC URL.
 */
export function startAnvil(): AnvilInstance {
  const proc = spawn(
    "anvil",
    [
      "--host",
      ANVIL_HOST,
      "--port",
      String(ANVIL_PORT),
      "--accounts",
      "10",
      "--balance",
      "10000",
    ],
    { stdio: ["ignore", "pipe", "pipe"] },
  );

  // Suppress noisy output but capture errors
  proc.stdout?.on("data", () => {});
  proc.stderr?.on("data", () => {});

  return { process: proc, rpcUrl: ANVIL_RPC_URL };
}

/**
 * Stop a running Anvil instance.
 */
export function stopAnvil(proc: ChildProcess): void {
  if (!proc.killed) {
    proc.kill("SIGTERM");
  }
}

export interface ViemClients {
  publicClient: PublicClient;
  walletClient: WalletClient<Transport, typeof foundry, ReturnType<typeof privateKeyToAccount>>;
  testClient: TestClient;
  account: ReturnType<typeof privateKeyToAccount>;
}

/**
 * Create viem clients connected to the given RPC URL.
 * Uses Anvil account 0 as the default signer.
 */
export function createClients(rpcUrl: string): ViemClients {
  const account = privateKeyToAccount(DEFAULT_PRIVATE_KEY);

  const publicClient = createPublicClient({
    chain: foundry,
    transport: http(rpcUrl),
  });

  const walletClient = createWalletClient({
    account,
    chain: foundry,
    transport: http(rpcUrl),
  });

  const testClient = createTestClient({
    mode: "anvil",
    chain: foundry,
    transport: http(rpcUrl),
  });

  return { publicClient, walletClient, testClient, account };
}

/**
 * Wait for Anvil to be ready by polling the RPC endpoint.
 */
export async function waitForAnvil(
  rpcUrl: string,
  maxAttempts = 20,
  delayMs = 250,
): Promise<void> {
  for (let i = 0; i < maxAttempts; i++) {
    try {
      const response = await fetch(rpcUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          jsonrpc: "2.0",
          id: 1,
          method: "eth_blockNumber",
          params: [],
        }),
      });
      if (response.ok) return;
    } catch {
      // not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }
  throw new Error(`Anvil not ready after ${maxAttempts} attempts`);
}
