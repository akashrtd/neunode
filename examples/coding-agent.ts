import { httpClient as neunode, numberEnv, required } from "./client.js";

const bountyId = required("BOUNTY_ID");
const runnerUrl = required("CODING_RUNNER_URL");
const bounty = await neunode.bounty.show(bountyId);
const state = Reflect.get(bounty, "state") ?? Reflect.get(bounty, "State");
if (typeof state !== "string" || state.toLowerCase() !== "open") {
	throw new Error(`Bounty ${bountyId} is ${String(state)}, not open`);
}

await neunode.bounty.claim({
	id: bountyId,
	stake: numberEnv("CLAIM_STAKE", 100, 1, Number.MAX_SAFE_INTEGER),
});
const response = await fetch(runnerUrl, {
	method: "POST",
	headers: { "content-type": "application/json" },
	body: JSON.stringify({ bounty }),
});
if (!response.ok) throw new Error(`Coding runner failed: ${response.status}`);
const result: unknown = await response.json();
if (!isArtifact(result))
	throw new Error("Runner must return { artifactCid, evidence? }");
await neunode.bounty.submit({
	id: bountyId,
	artifact: result.artifactCid,
	evidence: result.evidence,
});
console.log(`Submitted ${result.artifactCid} for bounty ${bountyId}`);

function isArtifact(
	value: unknown,
): value is { artifactCid: string; evidence?: string } {
	return (
		typeof value === "object" &&
		value !== null &&
		typeof Reflect.get(value, "artifactCid") === "string"
	);
}
