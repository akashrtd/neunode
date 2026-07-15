import { createNeunodeClient } from "@neunode/sdk";

export const httpClient = createNeunodeClient({
	http: { baseUrl: process.env.NEUNODE_URL ?? "http://127.0.0.1:41000" },
});

export const cliClient = createNeunodeClient({
	cli: { binaryPath: process.env.AGNETD_BIN ?? "agnetd" },
});

export function required(name: string): string {
	const value = process.env[name]?.trim();
	if (!value) throw new Error(`Set ${name} before starting this agent`);
	return value;
}

export function numberEnv(
	name: string,
	fallback: number,
	minimum: number,
	maximum: number,
) {
	const value = Number(process.env[name] ?? fallback);
	if (!Number.isFinite(value) || value < minimum || value > maximum) {
		throw new Error(`${name} must be between ${minimum} and ${maximum}`);
	}
	return value;
}

export async function registerProvider(
	name: string,
	endpoint: string,
	model: string,
) {
	if (!cliClient.cli) throw new Error("CLI transport unavailable");
	return cliClient.cli.execute([
		"inference",
		"register-provider",
		"--name",
		name,
		"--endpoint",
		endpoint,
		"--models",
		model,
	]);
}
