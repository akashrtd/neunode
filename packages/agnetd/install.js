#!/usr/bin/env node
"use strict";

const { generateBinPath } = require("./lib/platform");

try {
  const binPath = generateBinPath();
  const fs = require("fs");
  if (!fs.existsSync(binPath)) {
    console.error(
      `[agnetd] Binary not found at ${binPath}. ` +
        `Try reinstalling without --no-optional.`
    );
    process.exit(1);
  }
  try {
    fs.chmodSync(binPath, 0o755);
  } catch {
    // May fail on Windows or read-only filesystems
  }
} catch (e) {
  console.error(`[agnetd] Install check failed: ${e.message}`);
  process.exit(1);
}
