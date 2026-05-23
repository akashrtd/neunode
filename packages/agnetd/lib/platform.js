"use strict";

const os = require("os");

const knownPackages = {
  "linux x64": "@neunode/agnetd-linux-x64",
  "linux arm64": "@neunode/agnetd-linux-arm64",
  "darwin arm64": "@neunode/agnetd-darwin-arm64",
};

function pkgAndSubpathForCurrentPlatform() {
  const platformKey = `${process.platform} ${os.arch()}`;
  if (platformKey in knownPackages) {
    return { pkg: knownPackages[platformKey], subpath: "agnetd" };
  }
  throw new Error(
    `Unsupported platform: ${platformKey}. ` +
      `Supported: linux (x64, arm64), darwin (arm64).`
  );
}

function generateBinPath() {
  const overridePath = process.env.AGNETD_BINARY_PATH;
  if (overridePath) {
    const fs = require("fs");
    if (!fs.existsSync(overridePath)) {
      console.warn(
        `[agnetd] Ignoring bad configuration: AGNETD_BINARY_PATH=${overridePath}`
      );
    } else {
      return overridePath;
    }
  }

  const { pkg, subpath } = pkgAndSubpathForCurrentPlatform();
  try {
    return require.resolve(`${pkg}/${subpath}`);
  } catch (e) {
    throw new Error(
      `Could not resolve platform package "${pkg}". ` +
        `Make sure you did not install with --no-optional. ` +
        `Original error: ${e.message}`
    );
  }
}

module.exports = { pkgAndSubpathForCurrentPlatform, generateBinPath };
