import { existsSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const distDir = join(import.meta.dirname, "../dist");

describe("Build output verification", () => {
	it("should have dist/ directory", () => {
		expect(existsSync(distDir)).toBe(true);
		const stat = statSync(distDir);
		expect(stat.isDirectory()).toBe(true);
	});

	it("should have dist/index.js (ESM)", () => {
		const file = join(distDir, "index.js");
		expect(existsSync(file)).toBe(true);
		const stat = statSync(file);
		expect(stat.size).toBeGreaterThan(0);
	});

	it("should have dist/index.cjs (CJS)", () => {
		const file = join(distDir, "index.cjs");
		expect(existsSync(file)).toBe(true);
	});

	it("should have dist/index.d.ts (ESM types)", () => {
		const file = join(distDir, "index.d.ts");
		expect(existsSync(file)).toBe(true);
	});

	it("should have dist/index.d.cts (CJS types)", () => {
		const file = join(distDir, "index.d.cts");
		expect(existsSync(file)).toBe(true);
	});
});

describe("Package exports", () => {
	it("should export createNeunodeClient", async () => {
		const mod = await import(join(distDir, "index.js"));
		expect(mod.createNeunodeClient).toBeDefined();
		expect(typeof mod.createNeunodeClient).toBe("function");
	});

	it("should export CliTransport", async () => {
		const mod = await import(join(distDir, "index.js"));
		expect(mod.CliTransport).toBeDefined();
		expect(typeof mod.CliTransport).toBe("function");
	});

	it("should export CliTransportError", async () => {
		const mod = await import(join(distDir, "index.js"));
		expect(mod.CliTransportError).toBeDefined();
		expect(typeof mod.CliTransportError).toBe("function");
	});

	it("should export ViemTransport", async () => {
		const mod = await import(join(distDir, "index.js"));
		expect(mod.ViemTransport).toBeDefined();
		expect(typeof mod.ViemTransport).toBe("function");
	});

	it("should export type const objects", async () => {
		const mod = await import(join(distDir, "index.js"));
		expect(mod.Kind).toBeDefined();
		expect(mod.TokenType).toBeDefined();
		expect(mod.BountyState).toBeDefined();
		expect(mod.AgentLifecycle).toBeDefined();
		expect(mod.ActivityLevel).toBeDefined();
		expect(mod.KindCategory).toBeDefined();
		expect(mod.ExitCode).toBeDefined();
		expect(mod.OutputFormat).toBeDefined();
	});
});
