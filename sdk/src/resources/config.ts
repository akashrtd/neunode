import type { NeunodeClient } from "../client/client.js";

export interface ConfigSetParams {
	key: string;
	value: string;
}

/** Local agent configuration. */
export interface ConfigResource {
	/** Set a config key-value pair. */
	set(params: ConfigSetParams): Promise<void>;
	/** Get a single config value by key. */
	get(key: string): Promise<string>;
	/** List all config key-value pairs. */
	list(): Promise<Record<string, string>>;
	/** Get the filesystem path to the config file. */
	path(): Promise<string>;
}

export function createConfigResource(client: NeunodeClient): ConfigResource {
	return {
		async set(params: ConfigSetParams): Promise<void> {
			if (client.http) {
				await client.http.put("/api/v1/config", params);
				return;
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for config operations");
			await cli.execute(["config", "set", params.key, params.value]);
		},

		async get(key: string): Promise<string> {
			if (client.http) {
				const result = await client.http.get<Record<string, string>>(
					`/api/v1/config?key=${encodeURIComponent(key)}`,
				);
				return result[key] ?? "";
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for config operations");
			const result = await cli.execute<Record<string, string>>([
				"config",
				"get",
				key,
			]);
			return result[key] ?? "";
		},

		async list(): Promise<Record<string, string>> {
			if (client.http) {
				return client.http.get<Record<string, string>>("/api/v1/config");
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for config operations");
			return cli.execute<Record<string, string>>(["config", "list"]);
		},

		async path(): Promise<string> {
			if (client.http) {
				const result = await client.http.get<Record<string, string>>(
					"/api/v1/config/path",
				);
				return result["Config path"] ?? "";
			}
			const cli = client.cli;
			if (!cli) throw new Error("HTTP or CLI transport required for config operations");
			const result = await cli.execute<Record<string, string>>([
				"config",
				"path",
			]);
			return result["Config path"] ?? "";
		},
	};
}
