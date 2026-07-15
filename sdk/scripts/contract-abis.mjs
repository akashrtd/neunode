import { spawnSync } from "node:child_process";
import {
	mkdtempSync,
	readFileSync,
	rmSync,
	writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const sdkRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const workspaceRoot = resolve(sdkRoot, "..");
const artifactRoot = resolve(workspaceRoot, "contracts/out");
const abiRoot = resolve(sdkRoot, "src/contracts/abi");
const biome = resolve(sdkRoot, "node_modules/.bin/biome");

const outputs = [
	[
		"bandwidth-token.ts",
		[],
		["bandwidthTokenAbi", "BandwidthToken", "computeTokenAbi", "ComputeToken"],
	],
	["bounty-review.ts", [["bountyReviewAbi", "BountyReview"]]],
	["compute-token.ts", [["computeTokenAbi", "ComputeToken"]]],
	["model-registry.ts", [["modelRegistryAbi", "ModelRegistry"]]],
	["neunode-bounty.ts", [["neunodeBountyAbi", "NeunodeBounty"]]],
	["neunode-escrow.ts", [["neunodeEscrowAbi", "NeunodeEscrow"]]],
	["neunode-governance.ts", [["neunodeGovernanceAbi", "NeunodeGovernance"]]],
	["neunode-identity.ts", [["neunodeIdentityAbi", "NeunodeIdentity"]]],
	["neunode-registry.ts", [["neunodeRegistryAbi", "NeunodeRegistry"]]],
	["neunode-reputation.ts", [["neunodeReputationAbi", "NeunodeReputation"]]],
	["neunode-slashing.ts", [["neunodeSlashingAbi", "NeunodeSlashing"]]],
	["neunode-token.ts", [["neunodeTokenAbi", "NeunodeToken"]]],
	["royalty-splitter.ts", [["royaltySplitterAbi", "RoyaltySplitter"]]],
	["resource-amm.ts", [["resourceAmmAbi", "ResourceAMM"]]],
	["staking-escrow.ts", [["stakingEscrowAbi", "StakingEscrow"]]],
	[
		"storage-token.ts",
		[],
		["storageTokenAbi", "StorageToken", "computeTokenAbi", "ComputeToken"],
	],
	[
		"training-token.ts",
		[],
		["trainingTokenAbi", "TrainingToken", "computeTokenAbi", "ComputeToken"],
	],
	[
		"diamond.ts",
		[
			["diamondAbi", "Diamond"],
			["diamondCutFacetAbi", "DiamondCutFacet"],
			["diamondLoupeFacetAbi", "DiamondLoupeFacet"],
			["libDiamondErrors", "LibDiamond"],
		],
	],
];

function loadAbi(contractName) {
	const artifactPath = resolve(
		artifactRoot,
		`${contractName}.sol`,
		`${contractName}.json`,
	);
	try {
		return JSON.parse(readFileSync(artifactPath, "utf8")).abi;
	} catch (error) {
		throw new Error(
			`Missing or invalid Forge artifact ${artifactPath}; run \`forge build\` first`,
			{ cause: error },
		);
	}
}

function render(entries, alias) {
	if (alias) {
		const [variable, contract, targetVariable, targetContract] = alias;
		if (JSON.stringify(loadAbi(contract)) !== JSON.stringify(loadAbi(targetContract))) {
			throw new Error(`${contract} ABI no longer matches ${targetContract}; generate it directly`);
		}
		return `// Generated from Foundry artifacts by scripts/contract-abis.mjs.\n// Do not edit manually.\n\nimport { ${targetVariable} } from "./compute-token.js";\n\nexport const ${variable} = ${targetVariable};\n`;
	}
	const declarations = entries.map(
		([variable, contract]) =>
			`export const ${variable} = ${JSON.stringify(loadAbi(contract), null, 2)} as const;`,
	);
	return `// Generated from Foundry artifacts by scripts/contract-abis.mjs.\n// Do not edit manually.\n\n${declarations.join("\n\n")}\n`;
}

const tempRoot = mkdtempSync(resolve(tmpdir(), "neunode-abis-"));
let drifted = false;
try {
	for (const [filename, entries, alias] of outputs) {
		const tempPath = resolve(tempRoot, filename);
		writeFileSync(tempPath, render(entries, alias));
		const formatted = spawnSync(biome, ["format", "--write", tempPath], {
			cwd: sdkRoot,
			encoding: "utf8",
		});
		if (formatted.status !== 0) {
			process.stderr.write(formatted.stderr || formatted.stdout);
			process.exit(formatted.status ?? 1);
		}
		const expected = readFileSync(tempPath, "utf8");
		const outputPath = resolve(abiRoot, filename);
		if (process.argv.includes("--write")) {
			writeFileSync(outputPath, expected);
		} else {
			let actual = "";
			try {
				actual = readFileSync(outputPath, "utf8");
			} catch {
				// Report absent outputs as drift below.
			}
			if (actual !== expected) {
				process.stderr.write(`ABI drift: ${basename(outputPath)}\n`);
				drifted = true;
			}
		}
	}
} finally {
	rmSync(tempRoot, { recursive: true, force: true });
}

if (drifted) {
	process.stderr.write("Run `npm run generate:abi` after rebuilding contracts.\n");
	process.exit(1);
}
