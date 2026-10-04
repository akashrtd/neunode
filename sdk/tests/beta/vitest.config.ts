import { defineConfig } from "vitest/config";

// These are beta acceptance criteria. Failures identify missing product behavior;
// they intentionally run separately from tests of the current implementation.
export default defineConfig({
	test: {
		environment: "node",
		include: ["tests/beta/**/*.acceptance.test.ts"],
		testTimeout: 15_000,
		hookTimeout: 30_000,
		pool: "forks",
		poolOptions: { forks: { singleFork: true } },
		fileParallelism: false,
	},
});
