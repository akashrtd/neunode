#!/usr/bin/env npx tsx

import * as fs from "node:fs";
import * as path from "node:path";

const CONTRACT_KEYS: Record<string, string> = {
  ComputeToken: "computeToken",
  TrainingToken: "trainingToken",
  BandwidthToken: "bandwidthToken",
  StorageToken: "storageToken",
  NeunodeIdentity: "neunodeIdentity",
  NeunodeRegistry: "neunodeRegistry",
  NeunodeBounty: "neunodeBounty",
  NeunodeEscrow: "neunodeEscrow",
  BountyReview: "bountyReview",
  ModelRegistry: "modelRegistry",
  RoyaltySplitter: "royaltySplitter",
  NeunodeGovernance: "neunodeGovernance",
  Diamond: "diamond",
  DiamondCutFacet: "diamondCutFacet",
  DiamondLoupeFacet: "diamondLoupeFacet",
};

function parseArgs(): { chainId: number; broadcastDir: string } {
  const args = process.argv.slice(2);
  let chainId = 0;
  let broadcastDir = "";

  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--chain-id") {
      chainId = parseInt(args[++i], 10);
    } else if (args[i] === "--broadcast-dir") {
      broadcastDir = args[++i];
    }
  }

  if (!chainId || !broadcastDir) {
    console.error(
      "Usage: extract-addresses.ts --chain-id <id> --broadcast-dir <path>"
    );
    process.exit(1);
  }

  return { chainId, broadcastDir };
}

function loadBroadcast(broadcastDir: string): Map<string, string> {
  const latestFile = path.join(broadcastDir, "run-latest.json");

  if (!fs.existsSync(latestFile)) {
    console.error(`Broadcast file not found: ${latestFile}`);
    process.exit(1);
  }

  const data = JSON.parse(fs.readFileSync(latestFile, "utf-8"));
  const addresses = new Map<string, string>();

  for (const tx of data.transactions) {
    if (tx.transactionType !== "CREATE" || !tx.contractName) continue;
    const key = CONTRACT_KEYS[tx.contractName];
    if (key && tx.contractAddress) {
      addresses.set(key, tx.contractAddress as string);
    }
  }

  return addresses;
}

function updateAddressesFile(chainId: number, addresses: Map<string, string>): void {
  const addressesPath = path.resolve(
    __dirname,
    "../src/contracts/addresses.ts"
  );

  if (!fs.existsSync(addressesPath)) {
    console.error(`Addresses file not found: ${addressesPath}`);
    process.exit(1);
  }

  let content = fs.readFileSync(addressesPath, "utf-8");

  const allKeys = Object.values(CONTRACT_KEYS);
  const missing = allKeys.filter((k) => !addresses.has(k));
  if (missing.length > 0) {
    console.warn(
      `Warning: contracts not found in broadcast: ${missing.join(", ")}`
    );
  }

  const fields = allKeys
    .map((key) => {
      const addr = addresses.get(key) ?? "0x0000000000000000000000000000000000000000";
      return `    ${key}: '${addr}' as Address,`;
    })
    .join("\n");

  const chainEntry = `  ${chainId}: {\n${fields}\n  },`;

  const blockPattern = new RegExp(
    `\\s*${chainId}:\\s*\\{[^}]*\\},?\\s*\\n?`,
    "s"
  );

  const recordBlock = "export const chainAddresses: Record<number, ContractAddresses> = {";
  const recordIndex = content.indexOf(recordBlock);

  if (recordIndex === -1) {
    console.error("Could not find chainAddresses export in addresses.ts");
    process.exit(1);
  }

  const openBrace = content.indexOf("{", recordIndex);
  const closeBrace = content.indexOf("};", openBrace);

  if (openBrace === -1 || closeBrace === -1) {
    console.error("Could not parse chainAddresses structure");
    process.exit(1);
  }

  const inner = content.slice(openBrace + 1, closeBrace);

  const updatedInner = blockPattern.test(inner)
    ? inner.replace(blockPattern, `\n${chainEntry}\n`)
    : `${inner.trimEnd()}\n${chainEntry}\n`;

  content =
    content.slice(0, openBrace + 1) +
    updatedInner +
    content.slice(closeBrace);

  fs.writeFileSync(addressesPath, content, "utf-8");
  console.log(`Updated ${addressesPath} with chain ${chainId}`);
}

const { chainId, broadcastDir } = parseArgs();
const addresses = loadBroadcast(broadcastDir);
console.log(`Found ${addresses.size} contract addresses`);

for (const [key, addr] of addresses) {
  console.log(`  ${key}: ${addr}`);
}

updateAddressesFile(chainId, addresses);
