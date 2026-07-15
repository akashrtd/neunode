import { HttpTransport } from "./http-transport.js";

export type MockMethod = "GET" | "POST" | "PUT" | "DELETE";

export interface MockRequest {
	readonly method: MockMethod;
	readonly path: string;
	readonly body: unknown;
}

export type MockResponseHandler = (
	request: MockRequest,
) => unknown | Promise<unknown>;

export interface MockTransportConfig {
	/** Canned values keyed by `METHOD /path`, for example `GET /api/v1/feed`. */
	readonly responses?: Readonly<Record<string, unknown>>;
	/** Optional fallback for dynamic responses. */
	readonly handler?: MockResponseHandler;
	/** Throw when no canned response or handler exists. Defaults to true. */
	readonly strict?: boolean;
}

export class MockTransportError extends Error {
	constructor(
		message: string,
		public readonly request: MockRequest,
	) {
		super(message);
		this.name = "MockTransportError";
	}
}

/** In-memory HTTP-compatible transport for development and tests. */
export class MockTransport extends HttpTransport {
	private readonly responses = new Map<string, unknown>();
	private readonly handler: MockResponseHandler | undefined;
	private readonly strict: boolean;
	private readonly requestLog: MockRequest[] = [];

	constructor(config: MockTransportConfig = {}) {
		super({ baseUrl: "mock://neunode" });
		for (const [key, value] of Object.entries(config.responses ?? {})) {
			this.responses.set(normalizeKey(key), value);
		}
		this.handler = config.handler;
		this.strict = config.strict ?? true;
	}

	get requests(): readonly MockRequest[] {
		return this.requestLog.map((request) => ({ ...request }));
	}

	setResponse(method: MockMethod, path: string, response: unknown): void {
		this.responses.set(requestKey(method, path), response);
	}

	reset(): void {
		this.responses.clear();
		this.requestLog.length = 0;
	}

	override async get<T>(path: string): Promise<T> {
		return this.dispatch<T>("GET", path);
	}

	override async post<T>(path: string, body?: unknown): Promise<T> {
		return this.dispatch<T>("POST", path, body);
	}

	override async put<T>(path: string, body?: unknown): Promise<T> {
		return this.dispatch<T>("PUT", path, body);
	}

	override async delete<T>(path: string): Promise<T> {
		return this.dispatch<T>("DELETE", path);
	}

	private async dispatch<T>(
		method: MockMethod,
		path: string,
		body?: unknown,
	): Promise<T> {
		const request: MockRequest = { method, path, body };
		this.requestLog.push(request);
		const key = requestKey(method, path);

		if (this.responses.has(key)) {
			return this.responses.get(key) as T;
		}
		if (this.handler) {
			return (await this.handler(request)) as T;
		}
		if (this.strict) {
			throw new MockTransportError(
				`No mock response configured for ${key}`,
				request,
			);
		}
		return undefined as T;
	}
}

function requestKey(method: MockMethod, path: string): string {
	return `${method} ${path}`;
}

function normalizeKey(key: string): string {
	const separator = key.indexOf(" ");
	if (separator < 1) return key;
	return `${key.slice(0, separator).toUpperCase()} ${key.slice(separator + 1)}`;
}
