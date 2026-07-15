import { beforeEach, describe, expect, it, vi } from "vitest";
import { CliTransport, CliTransportError } from "./cli-transport.js";

const mockExecFile = vi.fn();
vi.mock("node:child_process", () => ({
	execFile: (...args: unknown[]) => mockExecFile(...args),
}));

vi.mock("node:util", () => ({
	promisify: (fn: unknown) => fn,
}));

describe("CliTransportError", () => {
	it("should set name, code, message, and stderr", () => {
		const err = new CliTransportError(11, "timed out", "stderr output");
		expect(err.name).toBe("CliTransportError");
		expect(err.code).toBe(11);
		expect(err.message).toBe("timed out");
		expect(err.stderr).toBe("stderr output");
	});

	it("should be an instance of Error", () => {
		const err = new CliTransportError(1, "fail", "");
		expect(err).toBeInstanceOf(Error);
		expect(err).toBeInstanceOf(CliTransportError);
	});
});

describe("CliTransport", () => {
	beforeEach(() => {
		mockExecFile.mockReset();
	});

	describe("constructor", () => {
		it("should use default binaryPath 'agnetd'", () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout: '{"data":{"ok":true},"success":true}',
				stderr: "",
			});
			transport.execute(["test"]);
			expect(mockExecFile).toHaveBeenCalledWith(
				"agnetd",
				expect.any(Array),
				expect.any(Object),
			);
		});

		it("should use custom binaryPath", () => {
			const transport = new CliTransport({
				binaryPath: "/usr/local/bin/agnetd",
			});
			mockExecFile.mockResolvedValue({
				stdout: '{"data":{"ok":true},"success":true}',
				stderr: "",
			});
			transport.execute(["test"]);
			expect(mockExecFile).toHaveBeenCalledWith(
				"/usr/local/bin/agnetd",
				expect.any(Array),
				expect.any(Object),
			);
		});

		it("should use default timeout 30000ms", () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout: '{"data":{"ok":true},"success":true}',
				stderr: "",
			});
			transport.execute(["test"]);
			expect(mockExecFile).toHaveBeenCalledWith(
				expect.any(String),
				expect.any(Array),
				expect.objectContaining({ timeout: 30_000 }),
			);
		});

		it("should use custom timeout", () => {
			const transport = new CliTransport({ timeout: 5_000 });
			mockExecFile.mockResolvedValue({
				stdout: '{"data":{"ok":true},"success":true}',
				stderr: "",
			});
			transport.execute(["test"]);
			expect(mockExecFile).toHaveBeenCalledWith(
				expect.any(String),
				expect.any(Array),
				expect.objectContaining({ timeout: 5_000 }),
			);
		});

		it("should pass global --identity flag", () => {
			const transport = new CliTransport({ identity: "did:neunode:abc123" });
			mockExecFile.mockResolvedValue({
				stdout: '{"data":{},"success":true}',
				stderr: "",
			});
			transport.execute(["identity", "show"]);
			const call = mockExecFile.mock.calls[0];
			expect(call).toBeDefined();
			const args = (call as [string, string[], unknown])[1];
			expect(args).toContain("--identity");
			expect(args).toContain("did:neunode:abc123");
		});

		it("should pass global --network flag", () => {
			const transport = new CliTransport({ network: "testnet" });
			mockExecFile.mockResolvedValue({
				stdout: '{"data":{},"success":true}',
				stderr: "",
			});
			transport.execute(["mesh", "status"]);
			const call = mockExecFile.mock.calls[0];
			expect(call).toBeDefined();
			const args = (call as [string, string[], unknown])[1];
			expect(args).toContain("--network");
			expect(args).toContain("testnet");
		});

		it("should pass global --config flag", () => {
			const transport = new CliTransport({ config: "/tmp/agnetd.toml" });
			mockExecFile.mockResolvedValue({
				stdout: '{"data":{},"success":true}',
				stderr: "",
			});
			transport.execute(["config", "list"]);
			const call = mockExecFile.mock.calls[0];
			expect(call).toBeDefined();
			const args = (call as [string, string[], unknown])[1];
			expect(args).toContain("--config");
			expect(args).toContain("/tmp/agnetd.toml");
		});

		it("should pass global --db-path flag", () => {
			const transport = new CliTransport({ dbPath: "/tmp/neunode-agent-a" });
			mockExecFile.mockResolvedValue({
				stdout: '{"data":{},"success":true}',
				stderr: "",
			});
			transport.execute(["config", "list"]);
			const call = mockExecFile.mock.calls[0];
			expect(call).toBeDefined();
			const args = (call as [string, string[], unknown])[1];
			expect(args).toContain("--db-path");
			expect(args).toContain("/tmp/neunode-agent-a");
		});
	});

	describe("execute", () => {
		it("should always pass --output json-compact", () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout: '{"data":{"x":1},"success":true}',
				stderr: "",
			});
			transport.execute(["identity", "list"]);
			const call = mockExecFile.mock.calls[0];
			expect(call).toBeDefined();
			const args = (call as [string, string[], unknown])[1];
			expect(args[0]).toBe("--output");
			expect(args[1]).toBe("json-compact");
		});

		it("should parse a single success envelope", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout:
					'{"data":{"DID":"did:neunode:abc","Name":"test"},"success":true}',
				stderr: "",
			});
			const result = await transport.execute<{ DID: string; Name: string }>([
				"identity",
				"show",
			]);
			expect(result).toEqual({ DID: "did:neunode:abc", Name: "test" });
		});

		it("should merge multiple success envelopes into one object", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout:
					'{"data":{"a":1},"success":true}\n{"data":{"b":2},"success":true}',
				stderr: "",
			});
			const result = await transport.execute<Record<string, number>>(["test"]);
			expect(result).toEqual({ a: 1, b: 2 });
		});

		it("should return scalar data directly", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout: '{"data":42,"success":true}',
				stderr: "",
			});
			const result = await transport.execute<number>(["test"]);
			expect(result).toBe(42);
		});

		it("should throw CliTransportError on error envelope in stdout", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout: '{"error":"agent not found","success":false}',
				stderr: "",
			});
			await expect(transport.execute(["identity", "show"])).rejects.toThrow(
				CliTransportError,
			);
			await expect(transport.execute(["identity", "show"])).rejects.toThrow(
				"agent not found",
			);
		});

		it("should throw CliTransportError on error envelope in stderr", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout: "",
				stderr: '{"error":"config not found","success":false}',
			});
			await expect(transport.execute(["config", "get", "x"])).rejects.toThrow(
				"config not found",
			);
		});

		it("should throw CliTransportError on empty stdout", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({ stdout: "   ", stderr: "" });
			await expect(transport.execute(["test"])).rejects.toThrow(
				"Empty response from agnetd",
			);
		});

		it("should throw CliTransportError with code 11 on timeout", async () => {
			const transport = new CliTransport({ timeout: 100 });
			const err = new Error("timed out") as NodeJS.ErrnoException;
			err.code = "ETIMEDOUT";
			mockExecFile.mockRejectedValue(err);
			try {
				await transport.execute(["slow-command"]);
				expect.unreachable("should have thrown");
			} catch (e) {
				expect(e).toBeInstanceOf(CliTransportError);
				expect((e as CliTransportError).code).toBe(11);
			}
		});

		it("should throw CliTransportError with code 11 on killed process", async () => {
			const transport = new CliTransport({ timeout: 100 });
			const err = new Error("killed") as NodeJS.ErrnoException & {
				killed: boolean;
			};
			err.code = "ERR_CHILD_PROCESS_EXIT";
			err.killed = true;
			mockExecFile.mockRejectedValue(err);
			try {
				await transport.execute(["slow"]);
				expect.unreachable("should have thrown");
			} catch (e) {
				expect(e).toBeInstanceOf(CliTransportError);
				expect((e as CliTransportError).code).toBe(11);
			}
		});

		it("should handle child process error with stderr containing error envelope", async () => {
			const transport = new CliTransport();
			const err = new Error("exit code 1") as NodeJS.ErrnoException & {
				stderr: string;
			};
			err.code = "ERR_CHILD_PROCESS";
			err.stderr = '{"error":"invalid argument","success":false}';
			mockExecFile.mockRejectedValue(err);
			await expect(transport.execute(["bad"])).rejects.toThrow(
				"invalid argument",
			);
		});

		it("should handle child process error with non-JSON stderr", async () => {
			const transport = new CliTransport();
			const err = new Error("exit code 1") as NodeJS.ErrnoException & {
				stderr: string;
			};
			err.code = "ERR_CHILD_PROCESS";
			err.stderr = "some random stderr text";
			mockExecFile.mockRejectedValue(err);
			await expect(transport.execute(["bad"])).rejects.toThrow(
				CliTransportError,
			);
		});

		it("should handle generic errors", async () => {
			const transport = new CliTransport();
			mockExecFile.mockRejectedValue("string error");
			await expect(transport.execute(["bad"])).rejects.toThrow(
				CliTransportError,
			);
		});

		it("should pass command args after global flags", () => {
			const transport = new CliTransport({ identity: "did:neunode:abc" });
			mockExecFile.mockResolvedValue({
				stdout: '{"data":{},"success":true}',
				stderr: "",
			});
			transport.execute(["identity", "show", "--did", "did:neunode:xyz"]);
			const call = mockExecFile.mock.calls[0];
			expect(call).toBeDefined();
			const args = (call as [string, string[], unknown])[1];
			expect(args).toEqual([
				"--output",
				"json-compact",
				"--identity",
				"did:neunode:abc",
				"identity",
				"show",
				"--did",
				"did:neunode:xyz",
			]);
		});
	});

	describe("executeMulti", () => {
		it("should return array of data payloads from multiple envelopes", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout:
					'{"data":{"token":"nCompute","balance":"1000"},"success":true}\n{"data":{"token":"nTrain","balance":"500"},"success":true}',
				stderr: "",
			});
			const results = await transport.executeMulti<Record<string, string>>([
				"token",
				"balance",
				"--token",
				"nCompute",
			]);
			expect(results).toHaveLength(2);
			expect(results[0]).toEqual({ token: "nCompute", balance: "1000" });
			expect(results[1]).toEqual({ token: "nTrain", balance: "500" });
		});

		it("should return empty array for empty stdout", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({ stdout: "", stderr: "" });
			const results = await transport.executeMulti(["test"]);
			expect(results).toEqual([]);
		});

		it("should throw on error envelope in multi-output", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout:
					'{"data":{"a":1},"success":true}\n{"error":"broken","success":false}',
				stderr: "",
			});
			await expect(transport.executeMulti(["test"])).rejects.toThrow("broken");
		});

		it("should throw on error envelope in stderr for multi-output", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout: '{"data":{"a":1},"success":true}',
				stderr: '{"error":"stderr error","success":false}',
			});
			await expect(transport.executeMulti(["test"])).rejects.toThrow(
				"stderr error",
			);
		});

		it("should skip lines that are not valid JSON envelopes", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({
				stdout: 'not-json\n{"data":{"ok":true},"success":true}\nalso-not-json',
				stderr: "",
			});
			const results = await transport.executeMulti<Record<string, boolean>>([
				"test",
			]);
			expect(results).toEqual([{ ok: true }]);
		});

		it("should always pass --output json-compact for multi", () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({ stdout: "", stderr: "" });
			transport.executeMulti(["token", "decay-info"]);
			const call = mockExecFile.mock.calls[0];
			expect(call).toBeDefined();
			const args = (call as [string, string[], unknown])[1];
			expect(args).toContain("--output");
			expect(args).toContain("json-compact");
		});

		it("should handle child process error in multi with stderr error envelope", async () => {
			const transport = new CliTransport();
			const err = new Error("exit 1") as NodeJS.ErrnoException & {
				stderr: string;
			};
			err.stderr = '{"error":"multi-fail","success":false}';
			mockExecFile.mockRejectedValue(err);
			await expect(transport.executeMulti(["bad"])).rejects.toThrow(
				"multi-fail",
			);
		});

		it("should handle generic error in multi without stderr", async () => {
			const transport = new CliTransport();
			mockExecFile.mockRejectedValue("multi-error");
			await expect(transport.executeMulti(["bad"])).rejects.toThrow(
				CliTransportError,
			);
		});
	});

	describe("executeRaw", () => {
		it("should return raw stdout without JSON parsing", async () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({ stdout: "raw output here" });
			const result = await transport.executeRaw(["config", "path"]);
			expect(result).toBe("raw output here");
		});

		it("should NOT pass --output json-compact for raw", () => {
			const transport = new CliTransport();
			mockExecFile.mockResolvedValue({ stdout: "raw" });
			transport.executeRaw(["some", "command"]);
			const call = mockExecFile.mock.calls[0];
			expect(call).toBeDefined();
			const args = (call as [string, string[], unknown])[1];
			expect(args).not.toContain("--output");
		});

		it("should still pass global flags for raw", () => {
			const transport = new CliTransport({ identity: "did:neunode:abc" });
			mockExecFile.mockResolvedValue({ stdout: "raw" });
			transport.executeRaw(["cmd"]);
			const call = mockExecFile.mock.calls[0];
			expect(call).toBeDefined();
			const args = (call as [string, string[], unknown])[1];
			expect(args).toContain("--identity");
			expect(args).toContain("did:neunode:abc");
		});

		it("should throw CliTransportError on child process error", async () => {
			const transport = new CliTransport();
			const err = new Error("exit 1") as NodeJS.ErrnoException & {
				stderr: string;
			};
			err.stderr = "raw stderr";
			mockExecFile.mockRejectedValue(err);
			await expect(transport.executeRaw(["bad"])).rejects.toThrow(
				CliTransportError,
			);
		});

		it("should throw CliTransportError on generic error in raw", async () => {
			const transport = new CliTransport();
			mockExecFile.mockRejectedValue("raw-err");
			await expect(transport.executeRaw(["bad"])).rejects.toThrow(
				CliTransportError,
			);
		});
	});
});
