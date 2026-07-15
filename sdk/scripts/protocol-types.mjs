import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const sdkRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const workspaceRoot = resolve(sdkRoot, "..");
const outputPath = resolve(sdkRoot, "src/types/protocol.generated.ts");
const result = spawnSync(
	"cargo",
	["run", "--quiet", "-p", "neunode-core", "--example", "emit_sdk_protocol"],
	{ cwd: workspaceRoot, encoding: "utf8" },
);

if (result.status !== 0) {
	process.stderr.write(result.stderr);
	process.exit(result.status ?? 1);
}

if (process.argv.includes("--write")) {
	writeFileSync(outputPath, result.stdout);
	process.stdout.write(`Updated ${outputPath}\n`);
} else if (readFileSync(outputPath, "utf8") !== result.stdout) {
	process.stderr.write(
		"Protocol types drifted from neunode-core. Run `npm run generate:protocol`.\n",
	);
	process.exit(1);
}
