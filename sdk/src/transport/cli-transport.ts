/**
 * CLI subprocess transport for @neunode/sdk.
 *
 * Spawns `agnetd --output json-compact` as a child process,
 * parses the JSON envelope, and returns typed responses.
 */

import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

// ---------------------------------------------------------------------------
// Envelope types (matches Rust output.rs wrap_success / write_error)
// ---------------------------------------------------------------------------

interface SuccessEnvelope<T> {
	readonly data: T;
	readonly success: true;
}

interface ErrorEnvelope {
	readonly error: string;
	readonly success: false;
}

type CliEnvelope<T> = SuccessEnvelope<T> | ErrorEnvelope;

// ---------------------------------------------------------------------------
// Transport config
// ---------------------------------------------------------------------------

export interface CliTransportConfig {
	/** Path to the agnetd binary. Default: "agnetd" (resolved via $PATH). */
	readonly binaryPath?: string;
	/** Timeout in milliseconds per command. Default: 30_000. */
	readonly timeout?: number;
	/** Global --identity flag (DID string). */
	readonly identity?: string;
	/** Global --network flag. */
	readonly network?: string;
	/** Global --config flag (path to config file). */
	readonly config?: string;
	/** Environment overrides for subprocesses (useful for isolated automation). */
	readonly env?: Readonly<NodeJS.ProcessEnv>;
}

// ---------------------------------------------------------------------------
// Transport error
// ---------------------------------------------------------------------------

export class CliTransportError extends Error {
	constructor(
		public readonly code: number,
		message: string,
		public readonly stderr: string,
	) {
		super(message);
		this.name = "CliTransportError";
	}
}

// ---------------------------------------------------------------------------
// Transport implementation
// ---------------------------------------------------------------------------

export class CliTransport {
	private readonly binaryPath: string;
	private readonly timeout: number;
	private readonly globalArgs: string[];
	private readonly env: NodeJS.ProcessEnv | undefined;

	constructor(config: CliTransportConfig = {}) {
		this.binaryPath = config.binaryPath ?? "agnetd";
		this.timeout = config.timeout ?? 30_000;
		this.env = config.env ? { ...config.env } : undefined;

		const args: string[] = [];
		if (config.identity) args.push("--identity", config.identity);
		if (config.network) args.push("--network", config.network);
		if (config.config) args.push("--config", config.config);
		this.globalArgs = args;
	}

	/**
	 * Execute an agnetd command and return a single parsed JSON envelope.
	 */
	async execute<T>(commandArgs: string[]): Promise<T> {
		const fullArgs = [
			"--output",
			"json-compact",
			...this.globalArgs,
			...commandArgs,
		];

		try {
			const { stdout, stderr } = await execFileAsync(
				this.binaryPath,
				fullArgs,
				{
					timeout: this.timeout,
					maxBuffer: 10 * 1024 * 1024, // 10MB
					encoding: "utf-8",
					env: this.env,
				},
			);

			return parseSingleEnvelope<T>(stdout, stderr);
		} catch (err) {
			if (err instanceof Error && "code" in err) {
				const nodeErr = err as NodeJS.ErrnoException;
				if (
					nodeErr.code === "ETIMEDOUT" ||
					("killed" in err && (err as { killed: boolean }).killed)
				) {
					throw new CliTransportError(
						11,
						`Command timed out after ${this.timeout}ms`,
						"",
					);
				}
				if (
					"stderr" in err &&
					typeof (err as { stderr: unknown }).stderr === "string"
				) {
					const stderr = (err as { stderr: string }).stderr;
					const errorEnvelope = tryParseError(stderr);
					if (errorEnvelope) {
						throw new CliTransportError(1, errorEnvelope.error, stderr);
					}
					throw new CliTransportError(1, err.message, stderr);
				}
			}
			throw new CliTransportError(1, String(err), "");
		}
	}

	/**
	 * Execute an agnetd command that produces multiple JSON envelopes
	 * (e.g., `token balance --token X` produces 3, `reputation factors` produces 4).
	 * Returns an array of parsed data payloads.
	 */
	async executeMulti<T>(commandArgs: string[]): Promise<T[]> {
		const fullArgs = [
			"--output",
			"json-compact",
			...this.globalArgs,
			...commandArgs,
		];

		try {
			const { stdout, stderr } = await execFileAsync(
				this.binaryPath,
				fullArgs,
				{
					timeout: this.timeout,
					maxBuffer: 10 * 1024 * 1024,
					encoding: "utf-8",
					env: this.env,
				},
			);

			return parseMultiEnvelope<T>(stdout, stderr);
		} catch (err) {
			if (
				err instanceof Error &&
				"stderr" in err &&
				typeof (err as { stderr: unknown }).stderr === "string"
			) {
				const stderr = (err as { stderr: string }).stderr;
				const errorEnvelope = tryParseError(stderr);
				if (errorEnvelope) {
					throw new CliTransportError(1, errorEnvelope.error, stderr);
				}
			}
			throw new CliTransportError(1, String(err), "");
		}
	}

	/**
	 * Execute a command and return raw stdout (for non-JSON output modes).
	 */
	async executeRaw(commandArgs: string[]): Promise<string> {
		const fullArgs = [...this.globalArgs, ...commandArgs];

		try {
			const { stdout } = await execFileAsync(this.binaryPath, fullArgs, {
				timeout: this.timeout,
				maxBuffer: 10 * 1024 * 1024,
				encoding: "utf-8",
				env: this.env,
			});
			return stdout;
		} catch (err) {
			if (
				err instanceof Error &&
				"stderr" in err &&
				typeof (err as { stderr: unknown }).stderr === "string"
			) {
				throw new CliTransportError(
					1,
					err.message,
					(err as { stderr: string }).stderr,
				);
			}
			throw new CliTransportError(1, String(err), "");
		}
	}
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

/**
 * Parse a single JSON envelope from stdout.
 * Handles the case where stderr contains an error envelope.
 */
function parseSingleEnvelope<T>(stdout: string, stderr: string): T {
	// Check stderr first for error envelopes
	const stdErrEnvelope = tryParseError(stderr);
	if (stdErrEnvelope) {
		throw new CliTransportError(1, stdErrEnvelope.error, stderr);
	}

	// Parse stdout — may contain 1 or more lines
	const lines = stdout
		.trim()
		.split("\n")
		.filter((l) => l.trim().length > 0);

	if (lines.length === 0) {
		throw new CliTransportError(1, "Empty response from agnetd", stderr);
	}

	// Merge all data fields from multiple lines
	let merged = {} as Partial<T>;
	for (const line of lines) {
		const parsed = tryParseEnvelope<Partial<T>>(line);
		if (!parsed) continue;

		if (!parsed.success) {
			throw new CliTransportError(1, parsed.error, stderr);
		}
		// Merge data — later values override earlier ones
		if (typeof parsed.data === "object" && parsed.data !== null) {
			merged = { ...merged, ...parsed.data };
		} else {
			// Scalar data — just return it
			return parsed.data as T;
		}
	}

	return merged as T;
}

/**
 * Parse multiple JSON envelopes from stdout.
 * Each line is a separate envelope; returns array of data payloads.
 */
function parseMultiEnvelope<T>(stdout: string, stderr: string): T[] {
	const stdErrEnvelope = tryParseError(stderr);
	if (stdErrEnvelope) {
		throw new CliTransportError(1, stdErrEnvelope.error, stderr);
	}

	const lines = stdout
		.trim()
		.split("\n")
		.filter((l) => l.trim().length > 0);
	const results: T[] = [];

	for (const line of lines) {
		const parsed = tryParseEnvelope<T>(line);
		if (!parsed) continue;

		if (!parsed.success) {
			throw new CliTransportError(1, parsed.error, stderr);
		}
		results.push(parsed.data);
	}

	return results;
}

/**
 * Try to parse a single line as a JSON envelope.
 */
function tryParseEnvelope<T>(line: string): CliEnvelope<T> | null {
	try {
		return JSON.parse(line) as CliEnvelope<T>;
	} catch {
		return null;
	}
}

/**
 * Try to parse an error envelope from stderr.
 */
function tryParseError(stderr: string): ErrorEnvelope | null {
	if (!stderr.trim()) return null;

	// stderr may contain multiple lines — try each
	const lines = stderr.trim().split("\n");
	for (const line of lines) {
		const parsed = tryParseEnvelope<never>(line);
		if (parsed && !parsed.success) {
			return parsed;
		}
	}
	return null;
}
