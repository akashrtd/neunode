import { defineConfig } from "tsup";

export default defineConfig({
  entry: { index: "src/index.ts", contracts: "src/contracts/index.ts" },
  format: ["esm", "cjs"],
  dts: true,
  splitting: false,
  sourcemap: true,
  clean: true,
  treeshake: true,
  external: ["viem"],
});
