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
			await client.http.put("/api/v1/config", params);
			return;
		},

		async get(key: string): Promise<string> {
			const result = await client.http.get<Record<string, string>>(
				`/api/v1/config?key=${encodeURIComponent(key)}`,
			);
			return result[key] ?? "";
		},

		async list(): Promise<Record<string, string>> {
			return client.http.get<Record<string, string>>("/api/v1/config");
		},

		async path(): Promise<string> {
			const result = await client.http.get<{ path: string }>(
				"/api/v1/config/path",
			);
			return result.path;
		},
	};
}
