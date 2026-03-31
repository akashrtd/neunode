import { startAnvil, stopAnvil, waitForAnvil, ANVIL_RPC_URL, type AnvilInstance } from "./helpers/anvil.js";

let anvilInstance: AnvilInstance | null = null;

export async function setup() {
  anvilInstance = startAnvil();
  await waitForAnvil(anvilInstance.rpcUrl);
  process.env.ANVIL_PID = String(anvilInstance.process.pid);
}

export async function teardown() {
  if (anvilInstance) {
    stopAnvil(anvilInstance.process);
    anvilInstance = null;
  }
}
