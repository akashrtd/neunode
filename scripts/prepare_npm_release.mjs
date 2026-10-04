import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!version || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?$/.test(version)) {
	throw new Error("Release tag must contain a valid package version");
}
const packages = ["sdk", "packages/mcp-server", "packages/agnetd", "packages/agnetd-linux-x64", "packages/agnetd-linux-arm64", "packages/agnetd-darwin-arm64"];
for (const directory of packages) {
	const path = `${directory}/package.json`;
	const manifest = JSON.parse(readFileSync(path, "utf8"));
	manifest.version = version;
	for (const field of ["dependencies", "optionalDependencies"]) {
		for (const name of Object.keys(manifest[field] ?? {})) {
			if (name.startsWith("@neunode/")) manifest[field][name] = version;
		}
	}
	writeFileSync(path, `${JSON.stringify(manifest, null, 2)}\n`);
}
