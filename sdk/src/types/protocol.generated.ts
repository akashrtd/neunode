// Generated from neunode-core. Run `npm run generate:protocol` to update.
// Do not edit manually.

export const Kind = {
	AgentMetadata: "AgentMetadata",
	CapabilityUpdate: "CapabilityUpdate",
	ReputationChange: "ReputationChange",
	IdentityRotation: "IdentityRotation",
	Lifecycle: "Lifecycle",
	BountyPost: "BountyPost",
	BountyClaim: "BountyClaim",
	BountySubmit: "BountySubmit",
	BountyReview: "BountyReview",
	BountyDispute: "BountyDispute",
	BountyResolved: "BountyResolved",
	EscrowDeposit: "EscrowDeposit",
	EscrowRelease: "EscrowRelease",
	EscrowRefund: "EscrowRefund",
	JobSubmit: "JobSubmit",
	Checkpoint: "Checkpoint",
	TrainingResult: "TrainingResult",
	GradientUpdate: "GradientUpdate",
	EvalScore: "EvalScore",
	Attest: "Attest",
	CounterAttest: "CounterAttest",
	DisputeInit: "DisputeInit",
	VerificationResult: "VerificationResult",
	ModelAnnounce: "ModelAnnounce",
	ServeOffer: "ServeOffer",
	ServeResult: "ServeResult",
	BenchmarkClaim: "BenchmarkClaim",
	Proposal: "Proposal",
	Vote: "Vote",
	Delegate: "Delegate",
	ParameterChange: "ParameterChange",
} as const;

export type Kind = (typeof Kind)[keyof typeof Kind];

export const KindCategory = {
	System: "System",
	Bounty: "Bounty",
	Training: "Training",
	Attestation: "Attestation",
	Inference: "Inference",
	Governance: "Governance",
	Custom: "Custom",
	Unknown: "Unknown",
} as const;

export type KindCategory = (typeof KindCategory)[keyof typeof KindCategory];
