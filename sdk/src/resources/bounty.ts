import type { NeunodeClient } from "../client/client.js";

export interface BountyCreateParams {
  title: string;
  description: string;
  reward: number;
  token: string;
  claimDeadline?: number;
  workDeadline?: number;
}

export interface BountyCreateResult {
  id: string;
  title: string;
  reward: number;
  token: string;
  state: string;
  claim_deadline: number;
  work_deadline: number;
}

export interface BountyClaimParams {
  id: string;
  stake: number;
}

export interface BountyClaimResult {
  bounty_id: string;
  claimant: string;
  bond: number;
  state: string;
}

export interface BountySubmitParams {
  id: string;
  artifact: string;
  evidence?: string;
}

export interface BountySubmitResult {
  bounty_id: string;
  artifact_cid: string;
  state: string;
}

export interface BountyReviewParams {
  id: string;
  score: number;
  feedback: string;
}

export interface BountyReviewResult {
  bounty_id: string;
  score: number;
  feedback: string;
  state: string;
}

export interface BountyListParams {
  state?: string;
  creator?: string;
  limit?: number;
}

export interface BountyShowResult {
  ID: string;
  State: string;
  Creator: string;
  Claimant: string;
  Reward: string;
  Deadline: string;
  Created: string;
  Escrow: string;
}

export interface BountyCancelResult {
  bounty_id: string;
  state: string;
  reason: string;
}

/** Bounty marketplace for task creation, claiming, submission, and review. */
export interface BountyResource {
  /** Create a new bounty with escrowed reward. */
  create(params: BountyCreateParams): Promise<BountyCreateResult>;
  /** Claim an open bounty by staking tokens. */
  claim(params: BountyClaimParams): Promise<BountyClaimResult>;
  /** Submit work artifact for a claimed bounty. */
  submit(params: BountySubmitParams): Promise<BountySubmitResult>;
  /** Review a submitted bounty and assign a score. */
  review(params: BountyReviewParams): Promise<BountyReviewResult>;
  /** List bounties, optionally filtered by state or creator. */
  list(params?: BountyListParams): Promise<void>;
  /** Show details for a specific bounty. */
  show(id: string): Promise<BountyShowResult>;
  /** Cancel an open bounty and refund escrow. */
  cancel(id: string, reason?: string): Promise<BountyCancelResult>;
}

export function createBountyResource(client: NeunodeClient): BountyResource {
  const cli = client.cli;
  if (!cli) throw new Error("CLI transport required for bounty operations");

  return {
    async create(params: BountyCreateParams): Promise<BountyCreateResult> {
      const args = [
        "bounty", "create",
        "--title", params.title,
        "--description", params.description,
        "--reward", String(params.reward),
        "--token", params.token,
      ];
      if (params.claimDeadline) args.push("--claim-deadline", String(params.claimDeadline));
      if (params.workDeadline) args.push("--work-deadline", String(params.workDeadline));
      return cli.execute<BountyCreateResult>(args);
    },

    async claim(params: BountyClaimParams): Promise<BountyClaimResult> {
      return cli.execute<BountyClaimResult>(["bounty", "claim", "--id", params.id, "--stake", String(params.stake)]);
    },

    async submit(params: BountySubmitParams): Promise<BountySubmitResult> {
      const args = ["bounty", "submit", "--id", params.id, "--artifact", params.artifact];
      if (params.evidence) args.push("--evidence", params.evidence);
      return cli.execute<BountySubmitResult>(args);
    },

    async review(params: BountyReviewParams): Promise<BountyReviewResult> {
      return cli.execute<BountyReviewResult>([
        "bounty", "review", "--id", params.id,
        "--score", String(params.score), "--feedback", params.feedback,
      ]);
    },

    async list(params?: BountyListParams): Promise<void> {
      const args = ["bounty", "list"];
      if (params?.state) args.push("--state", params.state);
      if (params?.creator) args.push("--creator", params.creator);
      if (params?.limit) args.push("--limit", String(params.limit));
      await cli.execute(args);
    },

    async show(id: string): Promise<BountyShowResult> {
      return cli.execute<BountyShowResult>(["bounty", "show", "--id", id]);
    },

    async cancel(id: string, reason?: string): Promise<BountyCancelResult> {
      const args = ["bounty", "cancel", "--id", id];
      if (reason) args.push("--reason", reason);
      return cli.execute<BountyCancelResult>(args);
    },
  };
}
