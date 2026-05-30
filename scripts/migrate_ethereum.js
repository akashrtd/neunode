// scripts/migrate_ethereum.js
const { ethers } = require("hardhat");
const { promises: fs } = require("fs");
const path = require("path");

// ============================================================
// Configuration: adjust addresses and contract names as needed
// ============================================================
const CONTRACTS = [
  {
    name: "NeunodeToken",
    mainnetAddress: "0x...", // Replace with actual mainnet address
    deployerAddress: "0x...", // Address that will deploy via CREATE2 on new L1
    salt: ethers.utils.hexZeroPad(ethers.utils.hexlify(1), 32), // Example salt
  },
  {
    name: "ReputationRegistry",
    mainnetAddress: "0x...",
    deployerAddress: "0x...",
    salt: ethers.utils.hexZeroPad(ethers.utils.hexlify(2), 32),
  },
  {
    name: "ValidatorSetManager",
    mainnetAddress: "0x...",
    deployerAddress: "0x...",
    salt: ethers.utils.hexZeroPad(ethers.utils.hexlify(3), 32),
  },
  {
    name: "Governance",
    mainnetAddress: "0x...",
    deployerAddress: "0x...",
    salt: ethers.utils.hexZeroPad(ethers.utils.hexlify(4), 32),
  },
  {
    name: "Membrane",
    mainnetAddress: "0x...",
    deployerAddress: "0x...",
    salt: ethers.utils.hexZeroPad(ethers.utils.hexlify(5), 32),
  },
  // Add more contracts as needed
];

// Paths for output data (will be consumed by deployment scripts on L1)
const OUTPUT_DIR = path.join(__dirname, "..", "migrations", "l1-deploy");

async function main() {
  // Connect to Ethereum mainnet (use env var MAINNET_RPC_URL)
  if (!process.env.MAINNET_RPC_URL) {
    throw new Error("MAINNET_RPC_URL environment variable not set");
  }
  const provider = new ethers.providers.JsonRpcProvider(process.env.MAINNET_RPC_URL);

  // Ensure output directory exists
  await fs.mkdir(OUTPUT_DIR, { recursive: true });

  const deploymentData = [];

  for (const contract of CONTRACTS) {
    console.log(`Processing ${contract.name} at ${contract.mainnetAddress}...`);

    // Fetch bytecode from mainnet
    const bytecode = await provider.getCode(contract.mainnetAddress);
    if (bytecode === "0x") {
      console.warn(`  Warning: No bytecode found at ${contract.mainnetAddress}`);
      continue;
    }

    // Fetch storage at critical slots (adjust based on contract structure)
    // For simplicity, we fetch the full storage (can be optimized)
    const storageKeys = [];
    // Example: fetch first 10 storage slots (or use a predefined list)
    for (let i = 0; i < 10; i++) {
      storageKeys.push(ethers.utils.hexZeroPad(ethers.utils.hexlify(i), 32));
    }
    const storageValues = {};
    for (const key of storageKeys) {
      const value = await provider.getStorageAt(contract.mainnetAddress, key);
      if (value !== "0x0000000000000000000000000000000000000000000000000000000000000000") {
        storageValues[key] = value;
      }
    }

    // Compute CREATE2 address on new L1
    const computedAddress = ethers.utils.getCreate2Address(
      contract.deployerAddress,
      contract.salt,
      ethers.utils.keccak256(bytecode)
    );

    console.log(`  Computed L1 address: ${computedAddress}`);

    deploymentData.push({
      name: contract.name,
      mainnetAddress: contract.mainnetAddress,
      l1Address: computedAddress,
      bytecode,
      storage: storageValues,
      deployerAddress: contract.deployerAddress,
      salt: contract.salt,
    });
  }

  // Write deployment artifact
  const outputPath = path.join(OUTPUT_DIR, "migration.json");
  await fs.writeFile(outputPath, JSON.stringify(deploymentData, null, 2));
  console.log(`\nMigration data written to ${outputPath}`);

  // Optionally, also write separate files for each contract
  for (const data of deploymentData) {
    const contractPath = path.join(OUTPUT_DIR, `${data.name}.json`);
    await fs.writeFile(contractPath, JSON.stringify(data, null, 2));
    console.log(`Wrote ${contractPath}`);
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });