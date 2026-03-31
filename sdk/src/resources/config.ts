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
  const cli = client.cli;
  if (!cli) throw new Error("CLI transport required for config operations");

  return {
    async set(params: ConfigSetParams): Promise<void> {
      await cli.execute(["config", "set", params.key, params.value]);
    },

    async get(key: string): Promise<string> {
      const result = await cli.execute<Record<string, string>>(["config", "get", key]);
      return result[key] ?? "";
    },

    async list(): Promise<Record<string, string>> {
      return cli.execute<Record<string, string>>(["config", "list"]);
    },

    async path(): Promise<string> {
      const result = await cli.execute<Record<string, string>>(["config", "path"]);
      return result["Config path"] ?? "";
    },
  };
}
