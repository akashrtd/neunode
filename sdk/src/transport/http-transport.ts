/**
 * HTTP transport for @neunode/sdk.
 *
 * Talks to a running `agnetd serve` instance via REST API at `/api/v1/*`.
 * Uses the same JSON envelope format as the CLI transport.
 */

// ---------------------------------------------------------------------------
// Envelope types (matches Rust api::types)
// ---------------------------------------------------------------------------

interface SuccessEnvelope<T> {
	readonly data: T;
	readonly success: true;
}

interface ErrorEnvelope {
	readonly error: {
		readonly code: string;
		readonly message: string;
	};
	readonly success: false;
}

type ApiEnvelope<T> = SuccessEnvelope<T> | ErrorEnvelope;

// ---------------------------------------------------------------------------
// Transport config
// ---------------------------------------------------------------------------

export interface HttpTransportConfig {
	/** Base URL of the agnetd server (e.g. "http://127.0.0.1:41000"). */
	readonly baseUrl: string;
	/** API key for authentication (optional). */
	readonly apiKey?: string;
	/** Timeout in milliseconds per request. Default: 30_000. */
	readonly timeout?: number;
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

export class HttpTransportError extends Error {
	constructor(
		message: string,
		public readonly code: string,
	) {
		super(message);
		this.name = "HttpTransportError";
	}
}

// ---------------------------------------------------------------------------
// HTTP Transport
// ---------------------------------------------------------------------------

export class HttpTransport {
	private readonly baseUrl: string;
	private readonly apiKey: string | undefined;
	private readonly timeout: number;

	constructor(config: HttpTransportConfig) {
		this.baseUrl = config.baseUrl.replace(/\/$/, "");
		this.apiKey = config.apiKey;
		this.timeout = config.timeout ?? 30_000;
	}

	/** The base URL this transport connects to (e.g. "http://127.0.0.1:41000"). */
	getBaseUrl(): string {
		return this.baseUrl;
	}

	/** Execute a GET request and return typed data. */
	async get<T>(path: string): Promise<T> {
		return this.request<T>("GET", path);
	}

	/** Execute a POST request with a JSON body. */
	async post<T>(path: string, body?: unknown): Promise<T> {
		return this.request<T>("POST", path, body);
	}

	/** Execute a PUT request with a JSON body. */
	async put<T>(path: string, body?: unknown): Promise<T> {
		return this.request<T>("PUT", path, body);
	}

	/** Execute a DELETE request. */
	async delete<T>(path: string): Promise<T> {
		return this.request<T>("DELETE", path);
	}

	private async request<T>(
		method: string,
		path: string,
		body?: unknown,
	): Promise<T> {
		const url = `${this.baseUrl}${path}`;
		const headers: Record<string, string> = {
			"Content-Type": "application/json",
		};
		if (this.apiKey) {
			headers.Authorization = `Bearer ${this.apiKey}`;
		}

		const controller = new AbortController();
		const timeoutId = setTimeout(() => controller.abort(), this.timeout);

		try {
			const init: RequestInit = {
				method,
				headers,
				signal: controller.signal,
			};
			if (body !== undefined) {
				(init as Record<string, unknown>).body = JSON.stringify(body);
			}
			const response = await fetch(url, init);

			if (!response.ok) {
				const text = await response.text().catch(() => "");
				let envelope: ApiEnvelope<unknown>;
				try {
					envelope = JSON.parse(text) as ApiEnvelope<unknown>;
				} catch {
					throw new HttpTransportError(
						`HTTP ${response.status}: ${text}`,
						`HTTP_${response.status}`,
					);
				}
				if (!envelope.success) {
					throw new HttpTransportError(
						envelope.error.message,
						envelope.error.code,
					);
				}
				throw new HttpTransportError(
					`HTTP ${response.status}`,
					`HTTP_${response.status}`,
				);
			}

			const text = await response.text();
			const envelope = JSON.parse(text) as ApiEnvelope<T>;
			if (!envelope.success) {
				throw new HttpTransportError(
					envelope.error.message,
					envelope.error.code,
				);
			}
			return envelope.data;
		} catch (err) {
			if (err instanceof HttpTransportError) throw err;
			if (err instanceof DOMException && err.name === "AbortError") {
				throw new HttpTransportError(
					`request timed out after ${this.timeout}ms`,
					"TIMEOUT",
				);
			}
			throw new HttpTransportError(
				err instanceof Error ? err.message : String(err),
				"NETWORK_ERROR",
			);
		} finally {
			clearTimeout(timeoutId);
		}
	}
}
