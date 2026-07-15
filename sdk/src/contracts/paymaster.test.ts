import { decodeAbiParameters, size, slice } from "viem";
import { describe, expect, it } from "vitest";
import {
	agentPaymasterSignatureMagic,
	encodeAgentPaymasterData,
	getAgentSponsorshipTypedData,
} from "./paymaster.js";

const paymaster = "0x00000000000000000000000000000000000000aa";
const signature = `0x${"11".repeat(65)}` as const;

describe("agent paymaster helpers", () => {
	it("encodes the ERC-4337 v0.8 signature-exclusion framing", () => {
		const encoded = encodeAgentPaymasterData({
			paymaster,
			verificationGasLimit: 100_000n,
			postOpGasLimit: 50_000n,
			validUntil: 1_000n,
			validAfter: 100n,
			sponsorLimit: 5_000_000n,
			signature,
		});

		expect(size(encoded)).toBe(223);
		expect(slice(encoded, 0, 20)).toBe(paymaster);
		expect(slice(encoded, -8)).toBe(agentPaymasterSignatureMagic);
		expect(BigInt(slice(encoded, -10, -8))).toBe(65n);
		const [validUntil, validAfter, sponsorLimit] = decodeAbiParameters(
			[{ type: "uint48" }, { type: "uint48" }, { type: "uint256" }],
			slice(encoded, 52, 148),
		);
		expect([validUntil, validAfter, sponsorLimit]).toEqual([
			1_000,
			100,
			5_000_000n,
		]);
	});

	it("rejects values that cannot be represented on the wire", () => {
		expect(() =>
			encodeAgentPaymasterData({
				paymaster,
				verificationGasLimit: 1n << 128n,
				postOpGasLimit: 0n,
				validUntil: 0n,
				validAfter: 0n,
				sponsorLimit: 0n,
				signature,
			}),
		).toThrow(RangeError);
	});

	it("builds the contract's exact sponsorship typed data", () => {
		const typedData = getAgentSponsorshipTypedData({
			chainId: 31337,
			paymaster,
			userOpHash: `0x${"22".repeat(32)}`,
			sponsorLimit: 10n,
			validUntil: 20n,
			validAfter: 5n,
		});
		expect(typedData.domain.name).toBe("Neunode Agent Paymaster");
		expect(typedData.primaryType).toBe("Sponsorship");
		expect(typedData.message.sponsorLimit).toBe(10n);
	});
});
