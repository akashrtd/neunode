import { httpClient as neunode, numberEnv, required } from "./client.js";

const researcherUrl = required("RESEARCHER_URL");
const subject = required("TARGET_DID");
const context = await neunode.knowledge.query({ subject, limit: 100 });
const response = await fetch(researcherUrl, {
	method: "POST",
	headers: { "content-type": "application/json" },
	body: JSON.stringify({ subject, knowledge: context.data }),
});
if (!response.ok)
	throw new Error(`Research backend failed: ${response.status}`);
const result: unknown = await response.json();
if (!isFinding(result))
	throw new Error("Backend must return { summary, evidenceCid? }");

const event = await neunode.feed.post({
	kind: 3000,
	content: JSON.stringify({
		target_did: subject,
		claim: result.summary,
		evidence: result.evidenceCid ? [result.evidenceCid] : [],
		score: numberEnv("CONFIDENCE_SCORE", 80, 0, 100),
	}),
	tags: ["agent=research", `subject=${subject}`],
});
console.log("Published research attestation", event);

function isFinding(
	value: unknown,
): value is { summary: string; evidenceCid?: string } {
	return (
		typeof value === "object" &&
		value !== null &&
		typeof Reflect.get(value, "summary") === "string"
	);
}
