import * as fs from "node:fs";
import * as path from "node:path";

const BINARY_NAME = "agnetd";

function findBinary(): string | null {
  // Try relative path from sdk/tests/integration/helpers/ to workspace root target/
  const candidates = [
    path.resolve(__dirname, "../../../../target/release/agnetd"),
    path.resolve(__dirname, "../../../../target/debug/agnetd"),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) return candidate;
  }
  // Fallback: let execFile resolve via PATH
  return null;
}

const BINARY_PATH: string | null = findBinary();

const hasBinary: boolean = BINARY_PATH !== null;

export { BINARY_PATH, hasBinary };
