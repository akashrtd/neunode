import { describe, expect, it, vi } from "vitest";
import type { NeunodeClient } from "../client/client.js";
import type { HttpTransport } from "../transport/http-transport.js";
import { createInferenceResource } from "./inference.js";

describe("inference WebSocket streaming", () => {
	it("submits a request and delivers the streamed result", () => {
		const send = vi.fn();
		const close = vi.fn();
		let socket: FakeWebSocket | undefined;
		class FakeWebSocket {
			onopen?: () => void;
			onmessage?: (event: MessageEvent) => void;
			constructor(readonly url: string) {
				socket = this;
			}
			send = send;
			close = close;
		}
		vi.stubGlobal("WebSocket", FakeWebSocket);
		const http = {
			getBaseUrl: () => "http://127.0.0.1:41000",
		} as HttpTransport;
		const client = { http } as NeunodeClient;
		const callback = vi.fn();
		const params = { model: "tiny", prompt: "hello", maxTokens: 8 };
		const cancel = createInferenceResource(client).stream(params, callback);

		expect(socket?.url).toBe("ws://127.0.0.1:41000/ws/inference");
		socket?.onopen?.();
		expect(send).toHaveBeenCalledWith(
			JSON.stringify({
				model: "tiny",
				prompt: "hello",
				max_tokens: 8,
			}),
		);
		socket?.onmessage?.({
			data: JSON.stringify({ status: "submitted" }),
		} as MessageEvent);
		expect(callback).toHaveBeenCalledWith({ status: "submitted" });
		cancel();
		expect(close).toHaveBeenCalled();
		vi.unstubAllGlobals();
	});
});
