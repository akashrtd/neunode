import { describe, expect, expectTypeOf, it } from "vitest";
import { EscrowState, ReviewOutcome, VerificationLayer } from "./bounty.js";
import { OutputFormat } from "./cli-output.js";
import type {
	BountyId,
	CID,
	Did,
	EventId,
	Hash256,
	JobId,
	ModelId,
	PeerId,
	Sequence,
	Signature,
	Timestamp,
	TokenAmount,
} from "./core.js";
import {
	ActivityLevel,
	AgentLifecycle,
	BountyState,
	Kind,
	KindCategory,
	TokenType,
} from "./core.js";
import { ExitCode } from "./errors.js";
import type { BountyEvent, NodeEvent } from "./index.js";
import {
	FinishReason,
	MessageRole,
	ProviderStatus,
	RoutingStrategy,
} from "./inference.js";
import { ContributionType, TrainingStatus } from "./model.js";
import { ReputationGrade } from "./reputation.js";

describe("Branded types compile", () => {
	it("Did is a branded string", () => {
		const did = "did:neunode:abc123" as Did;
		expectTypeOf(did).toBeString();
	});

	it("CID is a branded string", () => {
		const cid = "QmX7b..." as CID;
		expectTypeOf(cid).toBeString();
	});

	it("PeerId is a branded string", () => {
		const peerId = "12D3KooW..." as PeerId;
		expectTypeOf(peerId).toBeString();
	});

	it("BountyId is a branded string", () => {
		const id = "bnty_123" as BountyId;
		expectTypeOf(id).toBeString();
	});

	it("EventId is a branded string", () => {
		const id = "evt_abc" as EventId;
		expectTypeOf(id).toBeString();
	});

	it("ModelId is a branded string", () => {
		const id = "llama-3b" as ModelId;
		expectTypeOf(id).toBeString();
	});

	it("JobId is a branded string", () => {
		const id = "job_123" as JobId;
		expectTypeOf(id).toBeString();
	});

	it("Hash256 is a branded string", () => {
		const hash = "abc123def..." as Hash256;
		expectTypeOf(hash).toBeString();
	});

	it("Signature is a branded string", () => {
		const sig = "base64sig..." as Signature;
		expectTypeOf(sig).toBeString();
	});

	it("Timestamp is a branded number", () => {
		const ts = 1_700_000_000 as Timestamp;
		expectTypeOf(ts).toBeNumber();
	});

	it("Sequence is a branded number", () => {
		const seq = 1 as Sequence;
		expectTypeOf(seq).toBeNumber();
	});

	it("TokenAmount is a branded bigint", () => {
		const amount = 1000n as TokenAmount;
		expectTypeOf(amount).toBeBigInt();
	});
});

describe("Kind enum", () => {
	const kindKeys = [
		"AgentMetadata",
		"CapabilityUpdate",
		"ReputationChange",
		"IdentityRotation",
		"Lifecycle",
		"BountyPost",
		"BountyClaim",
		"BountySubmit",
		"BountyReview",
		"BountyDispute",
		"BountyResolved",
		"EscrowDeposit",
		"EscrowRelease",
		"EscrowRefund",
		"JobSubmit",
		"Checkpoint",
		"TrainingResult",
		"GradientUpdate",
		"EvalScore",
		"Attest",
		"CounterAttest",
		"DisputeInit",
		"VerificationResult",
		"ModelAnnounce",
		"ServeOffer",
		"ServeResult",
		"BenchmarkClaim",
		"Proposal",
		"Vote",
		"Delegate",
		"ParameterChange",
	] as const;

	it("should have all 31 variants", () => {
		expect(Object.keys(Kind)).toHaveLength(31);
	});

	it.each(kindKeys)("should have Kind.%s", (key) => {
		expect(Kind[key]).toBe(key);
	});

	it("values should equal their keys (string serialization)", () => {
		for (const key of kindKeys) {
			expect(Kind[key]).toBe(key);
		}
	});
});

describe("KindCategory enum", () => {
	const categories = [
		"System",
		"Bounty",
		"Training",
		"Attestation",
		"Inference",
		"Governance",
		"Custom",
		"Unknown",
	] as const;

	it("should have all 8 variants", () => {
		expect(Object.keys(KindCategory)).toHaveLength(8);
	});

	it.each(categories)("should have KindCategory.%s", (cat) => {
		expect(KindCategory[cat]).toBe(cat);
	});
});

describe("TokenType", () => {
	it("should have 4 token types", () => {
		expect(Object.keys(TokenType)).toHaveLength(4);
	});

	it("should contain Compute, Train, Bandwidth, Storage", () => {
		expect(TokenType.Compute).toBe("Compute");
		expect(TokenType.Train).toBe("Train");
		expect(TokenType.Bandwidth).toBe("Bandwidth");
		expect(TokenType.Storage).toBe("Storage");
	});
});

describe("AgentLifecycle", () => {
	it("should have 5 states", () => {
		expect(Object.keys(AgentLifecycle)).toHaveLength(5);
	});

	it("should contain Created, Active, Idle, Zombie, Dead", () => {
		expect(AgentLifecycle.Created).toBe("Created");
		expect(AgentLifecycle.Active).toBe("Active");
		expect(AgentLifecycle.Idle).toBe("Idle");
		expect(AgentLifecycle.Zombie).toBe("Zombie");
		expect(AgentLifecycle.Dead).toBe("Dead");
	});
});

describe("BountyState", () => {
	const states = [
		"Open",
		"Claimed",
		"Submitted",
		"UnderReview",
		"Revision",
		"Accepted",
		"Rejected",
		"Disputed",
		"Paid",
		"Expired",
		"Cancelled",
	] as const;

	it("should have 11 states", () => {
		expect(Object.keys(BountyState)).toHaveLength(11);
	});

	it.each(states)("should have BountyState.%s", (state) => {
		expect(BountyState[state]).toBe(state);
	});
});

describe("ActivityLevel", () => {
	it("should have 5 levels", () => {
		expect(Object.keys(ActivityLevel)).toHaveLength(5);
	});

	it("should contain Active, Moderate, Low, Inactive, Dead", () => {
		expect(ActivityLevel.Active).toBe("Active");
		expect(ActivityLevel.Moderate).toBe("Moderate");
		expect(ActivityLevel.Low).toBe("Low");
		expect(ActivityLevel.Inactive).toBe("Inactive");
		expect(ActivityLevel.Dead).toBe("Dead");
	});
});

describe("Const enum objects", () => {
	it("ReputationGrade should have A through F", () => {
		expect(Object.keys(ReputationGrade)).toHaveLength(5);
		expect(ReputationGrade.A).toBe("A");
		expect(ReputationGrade.F).toBe("F");
	});

	it("EscrowState should have Funded, Released, Refunded, Disputed", () => {
		expect(Object.keys(EscrowState)).toHaveLength(4);
	});

	it("ReviewOutcome should have Approved, Rejected, NeedsRevision", () => {
		expect(Object.keys(ReviewOutcome)).toHaveLength(3);
	});

	it("VerificationLayer should have 4 layers", () => {
		expect(Object.keys(VerificationLayer)).toHaveLength(4);
		expect(VerificationLayer.Layer1).toBe("Layer1");
		expect(VerificationLayer.Layer4).toBe("Layer4");
	});

	it("MessageRole should have 4 roles", () => {
		expect(Object.keys(MessageRole)).toHaveLength(4);
		expect(MessageRole.System).toBe("system");
		expect(MessageRole.User).toBe("user");
	});

	it("FinishReason should have 4 reasons", () => {
		expect(Object.keys(FinishReason)).toHaveLength(4);
	});

	it("ProviderStatus should have 3 statuses", () => {
		expect(Object.keys(ProviderStatus)).toHaveLength(3);
	});

	it("RoutingStrategy should have 5 strategies", () => {
		expect(Object.keys(RoutingStrategy)).toHaveLength(5);
	});

	it("TrainingStatus should have 5 statuses", () => {
		expect(Object.keys(TrainingStatus)).toHaveLength(5);
	});

	it("ContributionType should have 6 types", () => {
		expect(Object.keys(ContributionType)).toHaveLength(6);
	});

	it("OutputFormat should have 4 formats", () => {
		expect(Object.keys(OutputFormat)).toHaveLength(4);
	});

	it("ExitCode should have correct numeric values", () => {
		expect(ExitCode.Success).toBe(0);
		expect(ExitCode.GeneralError).toBe(1);
		expect(ExitCode.UsageError).toBe(2);
		expect(ExitCode.NetworkError).toBe(10);
		expect(ExitCode.Timeout).toBe(11);
		expect(ExitCode.AuthError).toBe(20);
		expect(ExitCode.InsufficientResources).toBe(30);
		expect(ExitCode.NotFound).toBe(40);
		expect(ExitCode.RateLimit).toBe(50);
		expect(ExitCode.Conflict).toBe(60);
	});
});

describe("Discriminated union narrowing", () => {
	describe("BountyEvent", () => {
		it("should narrow to Claim type", () => {
			const event: BountyEvent = {
				type: "Claim",
				claimant: "did:neunode:abc" as never,
				bond: 100n as never,
			};
			if (event.type === "Claim") {
				expect(event.claimant).toBeDefined();
				expect(event.bond).toBeDefined();
			}
		});

		it("should narrow to Submit type", () => {
			const event: BountyEvent = {
				type: "Submit",
				artifact_hash: "sha256:abc" as never,
			};
			if (event.type === "Submit") {
				expect(event.artifact_hash).toBeDefined();
			}
		});

		it("should narrow to SubmitReview type", () => {
			const event: BountyEvent = {
				type: "SubmitReview",
				reviewer: "did:neunode:rev" as never,
				score: 9,
				notes: "Great",
			};
			if (event.type === "SubmitReview") {
				expect(event.reviewer).toBeDefined();
				expect(event.score).toBe(9);
			}
		});

		it("should narrow to Dispute type with reason", () => {
			const event: BountyEvent = { type: "Dispute", reason: "bad quality" };
			if (event.type === "Dispute") {
				expect(event.reason).toBe("bad quality");
			}
		});

		it("should narrow to Cancel type", () => {
			const event: BountyEvent = { type: "Cancel" };
			expect(event.type).toBe("Cancel");
		});

		it("should narrow to Expire type", () => {
			const event: BountyEvent = { type: "Expire" };
			expect(event.type).toBe("Expire");
		});
	});

	describe("NodeEvent", () => {
		it("should narrow to GossipsubMessage type", () => {
			const event: NodeEvent = {
				type: "GossipsubMessage",
				source: "12D3KooW..." as never,
				topic: "bounty",
				data: new Uint8Array([1, 2, 3]),
			};
			if (event.type === "GossipsubMessage") {
				expect(event.topic).toBe("bounty");
				expect(event.data).toBeInstanceOf(Uint8Array);
			}
		});

		it("should narrow to PeerConnected type", () => {
			const event: NodeEvent = {
				type: "PeerConnected",
				peer_id: "12D3KooW..." as never,
			};
			if (event.type === "PeerConnected") {
				expect(event.peer_id).toBeDefined();
			}
		});

		it("should narrow to PeerDisconnected type", () => {
			const event: NodeEvent = {
				type: "PeerDisconnected",
				peer_id: "12D3KooW..." as never,
			};
			if (event.type === "PeerDisconnected") {
				expect(event.peer_id).toBeDefined();
			}
		});

		it("should narrow to IdentifyReceived type", () => {
			const event: NodeEvent = {
				type: "IdentifyReceived",
				peer_id: "12D3KooW..." as never,
				agent_version: "agnetd/0.1",
			};
			if (event.type === "IdentifyReceived") {
				expect(event.agent_version).toBe("agnetd/0.1");
			}
		});

		it("should narrow to PingResult type", () => {
			const event: NodeEvent = {
				type: "PingResult",
				peer_id: "12D3KooW..." as never,
				rtt_ms: 42,
			};
			if (event.type === "PingResult") {
				expect(event.rtt_ms).toBe(42);
			}
		});

		it("should narrow GossipsubMessage without source", () => {
			const event: NodeEvent = {
				type: "GossipsubMessage",
				topic: "feed",
				data: new Uint8Array([]),
			};
			if (event.type === "GossipsubMessage") {
				expect(event.source).toBeUndefined();
			}
		});
	});
});
