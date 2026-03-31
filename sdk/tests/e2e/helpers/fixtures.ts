import type { PublicClient, WalletClient, TestClient, HDAccount, Hex } from "viem";
import { beforeAll, beforeEach } from "vitest";
import { createClients, ANVIL_RPC_URL, waitForAnvil } from "./anvil.js";
import { deployAll, type DeployedContracts } from "./deploy.js";

export interface E2EFixture {
  publicClient: PublicClient;
  walletClient: WalletClient;
  testClient: TestClient;
  account: HDAccount;
  addresses: DeployedContracts;
  snapshotId: Hex;
}

let _fixture: E2EFixture | null = null;
let _initPromise: Promise<E2EFixture> | null = null;

async function initFixture(): Promise<E2EFixture> {
  if (_fixture) return _fixture;
  if (_initPromise) return _initPromise;

  _initPromise = (async () => {
    await waitForAnvil(ANVIL_RPC_URL);
    const clients = createClients(ANVIL_RPC_URL);
    const addresses = await deployAll(clients.walletClient, clients.publicClient);
    const snapshotId = await clients.testClient.snapshot();

    _fixture = {
      publicClient: clients.publicClient,
      walletClient: clients.walletClient,
      testClient: clients.testClient,
      account: clients.account,
      addresses,
      snapshotId,
    };

    return _fixture;
  })();

  return _initPromise;
}

export function useE2E() {
  let f: E2EFixture;

  beforeAll(async () => {
    f = await initFixture();
  });

  beforeEach(async () => {
    if (!f) throw new Error("E2E fixture not initialized — beforeAll did not complete");
    await f.testClient.revert({ id: f.snapshotId });
    f.snapshotId = await f.testClient.snapshot();
  });

  return {
    getFixture: (): E2EFixture => {
      if (!f) throw new Error("Fixture not ready — beforeAll has not run yet");
      return f;
    },
  };
}
