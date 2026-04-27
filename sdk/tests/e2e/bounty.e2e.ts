import { describe, it, expect } from "vitest";
import { useE2E } from "./helpers/fixtures.js";
import { neunodeBountyAbi } from "../../src/contracts/abi/neunode-bounty.js";
import { neunodeEscrowAbi } from "../../src/contracts/abi/neunode-escrow.js";
import { bountyReviewAbi } from "../../src/contracts/abi/bounty-review.js";
import { neunodeTokenAbi } from "../../src/contracts/abi/neunode-token.js";
import {
  keccak256,
  encodePacked,
  createWalletClient,
  http,
  hashTypedData,
  encodeAbiParameters,
  parseAbiParameters,
} from "viem";
import { foundry } from "viem/chains";
import { privateKeyToAccount } from "viem/accounts";
import type { HDAccount, WalletClient } from "viem";

const { getFixture } = useE2E();

// ─── Anvil accounts ──────────────────────────────────────────────────────
const DEPLOYER = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266" as const;
const PROVIDER = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8" as const;
const REVIEWER_1 = "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC" as const;
const REVIEWER_2 = "0x90F79bf6EB2c4f870365E785982E1f101E93b906" as const;
const REVIEWER_3 = "0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65" as const;

// Anvil private keys for reviewer accounts
const REVIEWER_1_KEY =
  "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a" as const;
const REVIEWER_2_KEY =
  "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6" as const;
const REVIEWER_3_KEY =
  "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733629f4c6a4a91d5d3" as const;

// ─── Helpers ─────────────────────────────────────────────────────────────

const REWARD_AMOUNT = 10000n;
const SUBMISSION_HASH =
  "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef" as const;

function bountyId(label: string): `0x${string}` {
  return keccak256(encodePacked(["string"], [label]));
}

async function futureTimestamp(offsetSeconds: bigint): Promise<bigint> {
  const { publicClient } = getFixture();
  const block = await publicClient.getBlock({ blockTag: "latest" });
  return block.timestamp + offsetSeconds;
}

function makeWalletClient(
  privateKey: `0x${string}`,
) {
  const account = privateKeyToAccount(privateKey);
  return createWalletClient({
    account,
    chain: foundry,
    transport: http("http://127.0.0.1:8545"),
  });
}

/** Mint tokens to `to`, then approve `spender` from `to`. */
async function mintAndApprove(
  to: `0x${string}`,
  spender: `0x${string}`,
  amount: bigint,
) {
  const { walletClient, publicClient, addresses } = getFixture();

  const mintHash = await walletClient.writeContract({
    abi: neunodeTokenAbi,
    address: addresses.computeToken,
    functionName: "mint",
    args: [to, amount],
    account: walletClient.account,
  });
  await publicClient.waitForTransactionReceipt({ hash: mintHash });

  // If `to` is not the deployer, we need a wallet client for `to` to approve
  if (to !== walletClient.account.address) {
    throw new Error("mintAndApprove: non-deployer approver not supported here");
  }

  const approveHash = await walletClient.writeContract({
    abi: neunodeTokenAbi,
    address: addresses.computeToken,
    functionName: "approve",
    args: [spender, amount],
    account: walletClient.account,
  });
  await publicClient.waitForTransactionReceipt({ hash: approveHash });
}

/** Create and claim a bounty in one go, returns the bounty ID. */
async function createAndClaimBounty(label: string): Promise<`0x${string}`> {
  const { walletClient, publicClient, addresses } = getFixture();
  const id = bountyId(label);
  const now = await futureTimestamp(0n);
  const claimDeadline = now + 86400n;
  const workDeadline = now + 604800n;

  await mintAndApprove(walletClient.account.address, addresses.neunodeBounty, REWARD_AMOUNT);

  const hash = await walletClient.writeContract({
    abi: neunodeBountyAbi,
    address: addresses.neunodeBounty,
    functionName: "createBounty",
    args: [id, REWARD_AMOUNT, addresses.computeToken, claimDeadline, workDeadline],
    account: walletClient.account,
  });
  await publicClient.waitForTransactionReceipt({ hash });

  // Claim as PROVIDER — need a wallet client for provider
  const providerClient = makeWalletClient(
    "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
  );
  const claimHash = await providerClient.writeContract({
    abi: neunodeBountyAbi,
    address: addresses.neunodeBounty,
    functionName: "claimBounty",
    args: [id],
    account: providerClient.account,
  });
  await publicClient.waitForTransactionReceipt({ hash: claimHash });

  return id;
}

// ═════════════════════════════════════════════════════════════════════════
// Bounty Lifecycle
// ═════════════════════════════════════════════════════════════════════════

describe("Bounty Lifecycle", () => {
  it("creates a bounty with correct state and emits BountyCreated", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const id = bountyId("create-test");
    const now = await futureTimestamp(0n);
    const claimDeadline = now + 86400n;
    const workDeadline = now + 604800n;

    await mintAndApprove(account.address, addresses.neunodeBounty, REWARD_AMOUNT);

    const hash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "createBounty",
      args: [id, REWARD_AMOUNT, addresses.computeToken, claimDeadline, workDeadline],
      account: account.address,
    });
    const receipt = await publicClient.waitForTransactionReceipt({ hash });

    // Check event
    const logs = receipt.logs;
    expect(logs.length).toBeGreaterThan(0);

    // Check state is Open (0)
    const state = await publicClient.readContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "getBountyState",
      args: [id],
    });
    expect(state).toBe(0); // Open
  });

  it("getBountyState returns 0 (Open) after creation", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const id = bountyId("state-open");
    const now = await futureTimestamp(0n);

    await mintAndApprove(account.address, addresses.neunodeBounty, REWARD_AMOUNT);

    const hash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "createBounty",
      args: [id, REWARD_AMOUNT, addresses.computeToken, now + 86400n, now + 604800n],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash });

    const state = await publicClient.readContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "getBountyState",
      args: [id],
    });
    expect(state).toBe(0);
  });

  it("getBountyFull returns correct details", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const id = bountyId("full-details");
    const now = await futureTimestamp(0n);
    const claimDeadline = now + 86400n;
    const workDeadline = now + 604800n;

    await mintAndApprove(account.address, addresses.neunodeBounty, REWARD_AMOUNT);

    const hash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "createBounty",
      args: [id, REWARD_AMOUNT, addresses.computeToken, claimDeadline, workDeadline],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash });

    const result = await publicClient.readContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "getBountyFull",
      args: [id],
    });

    // result is a tuple: [bountyId, requester, provider, state, reward, rewardToken, claimDeadline, workDeadline, reviewDeadline, created, submissionHash, revisionCount, revisionDeadline, disputeDeadline, useEscrow, providerBond]
    const [
      bountyId_,
      requester,
      provider,
      state,
      reward,
      rewardToken,
      claimDeadline_,
      workDeadline_,
      reviewDeadline_,
      created,
      submissionHash,
      revisionCount,
      revisionDeadline,
      disputeDeadline,
      useEscrow,
      providerBond,
    ] = result as unknown as [
      `0x${string}`,
      `0x${string}`,
      `0x${string}`,
      number,
      bigint,
      `0x${string}`,
      bigint,
      bigint,
      bigint,
      bigint,
      `0x${string}`,
      bigint,
      bigint,
      bigint,
      boolean,
      bigint,
    ];

    expect(bountyId_).toBe(id);
    expect(requester.toLowerCase()).toBe(account.address.toLowerCase());
    expect(provider).toBe("0x0000000000000000000000000000000000000000");
    expect(state).toBe(0); // Open
    expect(reward).toBe(REWARD_AMOUNT);
    expect(rewardToken.toLowerCase()).toBe(addresses.computeToken.toLowerCase());
    expect(claimDeadline_).toBe(claimDeadline);
    expect(workDeadline_).toBe(workDeadline);
    expect(submissionHash).toBe("0x0000000000000000000000000000000000000000000000000000000000000000");
    expect(revisionCount).toBe(0n);
    expect(useEscrow).toBe(false);
    expect(providerBond).toBe(0n);
  });

  it("claimBounty sets provider and state=Claimed(1)", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const id = bountyId("claim-test");
    const now = await futureTimestamp(0n);

    await mintAndApprove(account.address, addresses.neunodeBounty, REWARD_AMOUNT);

    const createHash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "createBounty",
      args: [id, REWARD_AMOUNT, addresses.computeToken, now + 86400n, now + 604800n],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: createHash });

    // Claim as provider
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );
    const claimHash = await providerClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "claimBounty",
      args: [id],
      account: providerClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: claimHash });

    const state = await publicClient.readContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "getBountyState",
      args: [id],
    });
    expect(state).toBe(1); // Claimed

    // Verify provider was set
    const result = await publicClient.readContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "getBountyFull",
      args: [id],
    });
    const full = result as unknown as [`0x${string}`, `0x${string}`, `0x${string}`, ...unknown[]];
    expect(full[2].toLowerCase()).toBe(PROVIDER.toLowerCase());
  });

  it("submitWork sets state=Submitted(2)", async () => {
    const { publicClient, addresses } = getFixture();
    const id = await createAndClaimBounty("submit-test");

    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );
    const submitHash = await providerClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "submitWork",
      args: [id, SUBMISSION_HASH],
      account: providerClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: submitHash });

    const state = await publicClient.readContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "getBountyState",
      args: [id],
    });
    expect(state).toBe(2); // Submitted
  });

  it("acceptSubmission sets state=Accepted(5)", async () => {
    const { walletClient, publicClient, addresses } = getFixture();
    const id = await createAndClaimBounty("accept-test");

    // Submit work
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );
    const submitHash = await providerClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "submitWork",
      args: [id, SUBMISSION_HASH],
      account: providerClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: submitHash });

    // Accept as requester
    const acceptHash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "acceptSubmission",
      args: [id],
      account: walletClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: acceptHash });

    const state = await publicClient.readContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "getBountyState",
      args: [id],
    });
    expect(state).toBe(5); // Accepted
  });

  it("cancelBounty sets state=Cancelled when Open", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const id = bountyId("cancel-test");
    const now = await futureTimestamp(0n);

    await mintAndApprove(account.address, addresses.neunodeBounty, REWARD_AMOUNT);

    const createHash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "createBounty",
      args: [id, REWARD_AMOUNT, addresses.computeToken, now + 86400n, now + 604800n],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: createHash });

    const cancelHash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "cancelBounty",
      args: [id],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: cancelHash });

    const state = await publicClient.readContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "getBountyState",
      args: [id],
    });
    expect(state).toBe(10); // Cancelled
  });

  it("non-requester cannot cancel a claimed bounty", async () => {
    const { publicClient, addresses } = getFixture();
    const id = await createAndClaimBounty("cancel-fail-test");

    // Try to cancel as provider (not the requester)
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );

    await expect(
      providerClient.writeContract({
        abi: neunodeBountyAbi,
        address: addresses.neunodeBounty,
        functionName: "cancelBounty",
        args: [id],
        account: providerClient.account,
      }),
    ).rejects.toThrow();
  });
});

// ═════════════════════════════════════════════════════════════════════════
// Bounty with Escrow (direct escrow — bounty contract holds tokens)
// ═════════════════════════════════════════════════════════════════════════

describe("Bounty with Escrow", () => {
  it("creates escrow via direct createEscrow and funds it", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const id = bountyId("escrow-direct");
    const now = await futureTimestamp(0n);
    const workDeadline = now + 604800n;

    // Mint tokens to deployer and approve escrow
    const mintHash = await walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "mint",
      args: [account.address, REWARD_AMOUNT * 2n],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: mintHash });

    const approveHash = await walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "approve",
      args: [addresses.neunodeEscrow, REWARD_AMOUNT],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: approveHash });

    // Create escrow
    const createHash = await walletClient.writeContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "createEscrow",
      args: [id, addresses.computeToken, REWARD_AMOUNT, workDeadline],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: createHash });

    // Verify escrow state = Created (0)
    const escrowState = await publicClient.readContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "getEscrowState",
      args: [id],
    });
    expect(escrowState).toBe(0); // Created

    // Provider funds escrow (15% bond = 1500 tokens)
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );

    // Mint tokens to provider for bond
    const mintToProviderHash = await walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "mint",
      args: [PROVIDER, 5000n],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: mintToProviderHash });

    // Provider approves escrow
    const providerApproveHash = await providerClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "approve",
      args: [addresses.neunodeEscrow, 2000n],
      account: providerClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: providerApproveHash });

    // Fund escrow
    const bondAmount = 1500n; // 15% of 10000
    const fundHash = await providerClient.writeContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "fundEscrow",
      args: [id, bondAmount],
      account: providerClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: fundHash });

    // Verify escrow state = Funded (1)
    const fundedState = await publicClient.readContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "getEscrowState",
      args: [id],
    });
    expect(fundedState).toBe(1); // Funded
  });

  it("escrow release sends funds to provider on accept", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const id = bountyId("escrow-release");
    const now = await futureTimestamp(0n);
    const workDeadline = now + 604800n;

    // Setup: mint + approve for escrow
    const mintHash = await walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "mint",
      args: [account.address, REWARD_AMOUNT * 2n],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: mintHash });

    const approveHash = await walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "approve",
      args: [addresses.neunodeEscrow, REWARD_AMOUNT],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: approveHash });

    // Create escrow
    await walletClient.writeContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "createEscrow",
      args: [id, addresses.computeToken, REWARD_AMOUNT, workDeadline],
      account: account.address,
    });

    // Provider funds
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );
    await walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "mint",
      args: [PROVIDER, 5000n],
      account: account.address,
    });
    await providerClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "approve",
      args: [addresses.neunodeEscrow, 2000n],
      account: providerClient.account,
    });
    await providerClient.writeContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "fundEscrow",
      args: [id, 1500n],
      account: providerClient.account,
    });

    const providerBalBefore = await publicClient.readContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "balanceOf",
      args: [PROVIDER],
    });

    const releaseHash = await walletClient.writeContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "release",
      args: [id],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: releaseHash });

    const escrowState = await publicClient.readContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "getEscrowState",
      args: [id],
    });
    expect(escrowState).toBe(2);

    const providerBalAfter = await publicClient.readContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "balanceOf",
      args: [PROVIDER],
    });
    expect(providerBalAfter - providerBalBefore).toBe(REWARD_AMOUNT + 1500n);
  });

  it("escrow refund returns funds to requester on reject", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const id = bountyId("escrow-refund");
    const now = await futureTimestamp(0n);
    const workDeadline = now + 604800n;

    // Setup: mint + approve for escrow
    await walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "mint",
      args: [account.address, REWARD_AMOUNT * 2n],
      account: account.address,
    });
    await walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "approve",
      args: [addresses.neunodeEscrow, REWARD_AMOUNT],
      account: account.address,
    });

    // Create escrow
    await walletClient.writeContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "createEscrow",
      args: [id, addresses.computeToken, REWARD_AMOUNT, workDeadline],
      account: account.address,
    });

    // Provider funds
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );
    await walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "mint",
      args: [PROVIDER, 5000n],
      account: account.address,
    });
    await providerClient.writeContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "approve",
      args: [addresses.neunodeEscrow, 2000n],
      account: providerClient.account,
    });
    await providerClient.writeContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "fundEscrow",
      args: [id, 1500n],
      account: providerClient.account,
    });

    // Record requester balance before refund
    const requesterBalBefore = await publicClient.readContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "balanceOf",
      args: [account.address],
    });

    // Refund (requester calls)
    const refundHash = await walletClient.writeContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "refund",
      args: [id],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash: refundHash });

    // Verify escrow state = Refunded (3)
    const escrowState = await publicClient.readContract({
      abi: neunodeEscrowAbi,
      address: addresses.neunodeEscrow,
      functionName: "getEscrowState",
      args: [id],
    });
    expect(escrowState).toBe(3); // Refunded

    // Requester should have received amount + slashed bond
    const requesterBalAfter = await publicClient.readContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "balanceOf",
      args: [account.address],
    });
    expect(requesterBalAfter - requesterBalBefore).toBe(REWARD_AMOUNT + 1500n);
  });

  it("rejectSubmission on bounty refunds reward to requester", async () => {
    const { walletClient, publicClient, addresses } = getFixture();
    const id = await createAndClaimBounty("reject-refund");

    // Submit work
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );
    await providerClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "submitWork",
      args: [id, SUBMISSION_HASH],
      account: providerClient.account,
    });

    // Record requester balance before reject
    const requesterBalBefore = await publicClient.readContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "balanceOf",
      args: [walletClient.account.address],
    });

    // Reject
    const rejectHash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "rejectSubmission",
      args: [id],
      account: walletClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: rejectHash });

    const state = await publicClient.readContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "getBountyState",
      args: [id],
    });
    expect(state).toBe(6); // Rejected

    // Requester should get their tokens back
    const requesterBalAfter = await publicClient.readContract({
      abi: neunodeTokenAbi,
      address: addresses.computeToken,
      functionName: "balanceOf",
      args: [walletClient.account.address],
    });
    expect(requesterBalAfter - requesterBalBefore).toBe(REWARD_AMOUNT);
  });
});

// ═════════════════════════════════════════════════════════════════════════
// Review System
// ═════════════════════════════════════════════════════════════════════════

describe("Review System", () => {
  it("startReview assigns committee and sets state=UnderReview(3)", async () => {
    const { walletClient, publicClient, addresses } = getFixture();
    const id = await createAndClaimBounty("review-start");

    // Submit work
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );
    await providerClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "submitWork",
      args: [id, SUBMISSION_HASH],
      account: providerClient.account,
    });

    // Grant bounty contract DEFAULT_ADMIN_ROLE on review contract
    // (startReview calls assignCommittee which requires DEFAULT_ADMIN_ROLE)
    const grantHash = await walletClient.writeContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "grantRole",
      args: [
        "0x0000000000000000000000000000000000000000000000000000000000000000", // DEFAULT_ADMIN_ROLE
        addresses.neunodeBounty,
      ],
      account: walletClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: grantHash });

    // Start review
    const reviewers = [REVIEWER_1, REVIEWER_2, REVIEWER_3] as const;
    const startHash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "startReview",
      args: [id, reviewers],
      account: walletClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: startHash });

    // Verify bounty state = UnderReview (3)
    const state = await publicClient.readContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "getBountyState",
      args: [id],
    });
    expect(state).toBe(3); // UnderReview

    // Verify committee was assigned
    const committee = await publicClient.readContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "getCommittee",
      args: [id],
    });
    const [cReviewers, cAccept, cReject, cResolved, cAssigned] =
      committee as unknown as [
        `0x${string}`[],
        number,
        number,
        boolean,
        boolean,
      ];
    expect(cReviewers[0].toLowerCase()).toBe(REVIEWER_1.toLowerCase());
    expect(cReviewers[1].toLowerCase()).toBe(REVIEWER_2.toLowerCase());
    expect(cReviewers[2].toLowerCase()).toBe(REVIEWER_3.toLowerCase());
    expect(cAssigned).toBe(true);
    expect(cResolved).toBe(false);
  });

  it("submitReview records a review with EIP-712 signature", async () => {
    const { walletClient, publicClient, addresses } = getFixture();
    const id = await createAndClaimBounty("review-submit");

    // Submit work — MUST await receipt before proceeding
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );
    const submitWorkHash = await providerClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "submitWork",
      args: [id, SUBMISSION_HASH],
      account: providerClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: submitWorkHash });

    // Grant bounty contract admin role on review — MUST await receipt
    const grantHash = await walletClient.writeContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "grantRole",
      args: [
        "0x0000000000000000000000000000000000000000000000000000000000000000",
        addresses.neunodeBounty,
      ],
      account: walletClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: grantHash });

    // Start review — MUST await receipt before signing with nonce
    const reviewers = [REVIEWER_1, REVIEWER_2, REVIEWER_3] as const;
    const startHash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "startReview",
      args: [id, reviewers],
      account: walletClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: startHash });

    // Force deterministic mine to ensure block state is available
    await getFixture().testClient.mine({ blocks: 1 });

    // Build EIP-712 domain from contract
    const domainData = await publicClient.readContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "eip712Domain",
      args: [],
    });
    const [fields, name, version, chainId, verifyingContract, salt, extensions] =
      domainData as unknown as [
        `0x${string}`,
        string,
        string,
        bigint,
        `0x${string}`,
        `0x${string}`,
        bigint[],
      ];

    const domain = {
      name,
      version,
      chainId: Number(chainId),
      verifyingContract,
    } as const;

    const REVIEW_TYPEHASH =
      "BountyReview(bytes32 bountyId,uint8 score,string feedback,uint256 nonce)";

    // Reviewer 1 submits a review (score=80 → accepted)
    const reviewer1Account = privateKeyToAccount(REVIEWER_1_KEY);
    const feedback = "Good work, meets requirements";
    const nonce1 = await publicClient.readContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "nonces",
      args: [REVIEWER_1],
    });

    const structHash1 = keccak256(
      encodeAbiParameters(
        parseAbiParameters("bytes32, bytes32, uint8, bytes32, uint256"),
        [
          keccak256(new TextEncoder().encode(REVIEW_TYPEHASH)),
          id,
          80,
          keccak256(new TextEncoder().encode(feedback)),
          nonce1 as bigint,
        ],
      ),
    );

    const typedDataHash1 = hashTypedData({
      domain,
      types: {
        BountyReview: [
          { name: "bountyId", type: "bytes32" },
          { name: "score", type: "uint8" },
          { name: "feedback", type: "string" },
          { name: "nonce", type: "uint256" },
        ],
      },
      primaryType: "BountyReview",
      message: {
        bountyId: id,
        score: 80,
        feedback,
        nonce: nonce1 as bigint,
      },
    });

    const signature1 = await reviewer1Account.sign({
      hash: typedDataHash1,
    });

    const reviewer1Client = makeWalletClient(REVIEWER_1_KEY);
    const submitHash1 = await reviewer1Client.writeContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "submitReview",
      args: [id, 80, feedback, signature1],
      account: reviewer1Client.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: submitHash1 });

    await getFixture().testClient.mine({ blocks: 1 });

    // Verify review was recorded
    const reviewCount = await publicClient.readContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "getReviewCount",
      args: [id],
    });
    expect(reviewCount).toBe(1n);

    const review = await publicClient.readContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "getReview",
      args: [id, 0n],
    });
    const [rReviewer, rScore, rFeedback] = review as unknown as [
      `0x${string}`,
      number,
      string,
    ];
    expect(rReviewer.toLowerCase()).toBe(REVIEWER_1.toLowerCase());
    expect(rScore).toBe(80);
    expect(rFeedback).toBe(feedback);
  });

  it("isAccepted returns true after 2-of-3 accept", async () => {
    const { walletClient, publicClient, addresses } = getFixture();
    const id = await createAndClaimBounty("review-2of3");

    // Submit work
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );
    await providerClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "submitWork",
      args: [id, SUBMISSION_HASH],
      account: providerClient.account,
    });

    // Grant bounty contract admin role on review
    await walletClient.writeContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "grantRole",
      args: [
        "0x0000000000000000000000000000000000000000000000000000000000000000",
        addresses.neunodeBounty,
      ],
      account: walletClient.account,
    });

    // Start review
    const reviewers = [REVIEWER_1, REVIEWER_2, REVIEWER_3] as const;
    await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "startReview",
      args: [id, reviewers],
      account: walletClient.account,
    });

    // Get EIP-712 domain
    const domainData = await publicClient.readContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "eip712Domain",
      args: [],
    });
    const [, name, version, chainId, verifyingContract] =
      domainData as unknown as [
        `0x${string}`,
        string,
        string,
        bigint,
        `0x${string}`,
        `0x${string}`,
        bigint[],
      ];

    const domain = {
      name,
      version,
      chainId: Number(chainId),
      verifyingContract,
    } as const;

    // Submit 2 accepting reviews (score >= 50)
    const reviewerAccounts = [
      { key: REVIEWER_1_KEY, addr: REVIEWER_1 },
      { key: REVIEWER_2_KEY, addr: REVIEWER_2 },
    ];

    for (const { key, addr } of reviewerAccounts) {
      const account = privateKeyToAccount(key);
      const client = makeWalletClient(key);
      const nonce = await publicClient.readContract({
        abi: bountyReviewAbi,
        address: addresses.bountyReview,
        functionName: "nonces",
        args: [addr],
      });

      const typedDataHash = hashTypedData({
        domain,
        types: {
          BountyReview: [
            { name: "bountyId", type: "bytes32" },
            { name: "score", type: "uint8" },
            { name: "feedback", type: "string" },
            { name: "nonce", type: "uint256" },
          ],
        },
        primaryType: "BountyReview",
        message: {
          bountyId: id,
          score: 75,
          feedback: "Acceptable work",
          nonce: nonce as bigint,
        },
      });

      const signature = await account.sign({ hash: typedDataHash });

      const hash = await client.writeContract({
        abi: bountyReviewAbi,
        address: addresses.bountyReview,
        functionName: "submitReview",
        args: [id, 75, "Acceptable work", signature],
        account: client.account,
      });
      await publicClient.waitForTransactionReceipt({ hash });
    }

    // Check isAccepted
    const accepted = await publicClient.readContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "isAccepted",
      args: [id],
    });
    expect(accepted).toBe(true);

    // Check isResolved
    const resolved = await publicClient.readContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "isResolved",
      args: [id],
    });
    expect(resolved).toBe(true);
  });

  it("processReviewResult updates bounty state based on review", async () => {
    const { walletClient, publicClient, addresses } = getFixture();
    const id = await createAndClaimBounty("review-process");

    // Submit work
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );
    await providerClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "submitWork",
      args: [id, SUBMISSION_HASH],
      account: providerClient.account,
    });

    // Grant bounty contract admin role on review
    await walletClient.writeContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "grantRole",
      args: [
        "0x0000000000000000000000000000000000000000000000000000000000000000",
        addresses.neunodeBounty,
      ],
      account: walletClient.account,
    });

    // Start review
    const reviewers = [REVIEWER_1, REVIEWER_2, REVIEWER_3] as const;
    await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "startReview",
      args: [id, reviewers],
      account: walletClient.account,
    });

    // Get EIP-712 domain
    const domainData = await publicClient.readContract({
      abi: bountyReviewAbi,
      address: addresses.bountyReview,
      functionName: "eip712Domain",
      args: [],
    });
    const [, name, version, chainId, verifyingContract] =
      domainData as unknown as [
        `0x${string}`,
        string,
        string,
        bigint,
        `0x${string}`,
        `0x${string}`,
        bigint[],
      ];

    const domain = {
      name,
      version,
      chainId: Number(chainId),
      verifyingContract,
    } as const;

    // Submit 2 accepting reviews
    const reviewerAccounts = [
      { key: REVIEWER_1_KEY, addr: REVIEWER_1 },
      { key: REVIEWER_2_KEY, addr: REVIEWER_2 },
    ];

    for (const { key, addr } of reviewerAccounts) {
      const account = privateKeyToAccount(key);
      const client = makeWalletClient(key);
      const nonce = await publicClient.readContract({
        abi: bountyReviewAbi,
        address: addresses.bountyReview,
        functionName: "nonces",
        args: [addr],
      });

      const typedDataHash = hashTypedData({
        domain,
        types: {
          BountyReview: [
            { name: "bountyId", type: "bytes32" },
            { name: "score", type: "uint8" },
            { name: "feedback", type: "string" },
            { name: "nonce", type: "uint256" },
          ],
        },
        primaryType: "BountyReview",
        message: {
          bountyId: id,
          score: 90,
          feedback: "Excellent work",
          nonce: nonce as bigint,
        },
      });

      const signature = await account.sign({ hash: typedDataHash });

      const hash = await client.writeContract({
        abi: bountyReviewAbi,
        address: addresses.bountyReview,
        functionName: "submitReview",
        args: [id, 90, "Excellent work", signature],
        account: client.account,
      });
      await publicClient.waitForTransactionReceipt({ hash });
    }

    // Process review result — bounty state should go UnderReview → Accepted
    const processHash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "processReviewResult",
      args: [id],
      account: walletClient.account,
    });
    await publicClient.waitForTransactionReceipt({ hash: processHash });

    const state = await publicClient.readContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "getBountyState",
      args: [id],
    });
    expect(state).toBe(5); // Accepted
  });
});

// ═════════════════════════════════════════════════════════════════════════
// Error Cases
// ═════════════════════════════════════════════════════════════════════════

describe("Error Cases", () => {
  it("cannot claim an already-claimed bounty", async () => {
    const { publicClient, addresses } = getFixture();
    const id = await createAndClaimBounty("double-claim");

    // Try to claim again as a different provider
    const secondProviderClient = makeWalletClient(
      "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a",
    );

    await expect(
      secondProviderClient.writeContract({
        abi: neunodeBountyAbi,
        address: addresses.neunodeBounty,
        functionName: "claimBounty",
        args: [id],
        account: secondProviderClient.account,
      }),
    ).rejects.toThrow();
  });

  it("cannot submit work on an unclaimed bounty", async () => {
    const { walletClient, publicClient, addresses, account } = getFixture();
    const id = bountyId("submit-unclaimed");
    const now = await futureTimestamp(0n);

    // Create bounty only (do NOT claim)
    await mintAndApprove(account.address, addresses.neunodeBounty, REWARD_AMOUNT);
    const hash = await walletClient.writeContract({
      abi: neunodeBountyAbi,
      address: addresses.neunodeBounty,
      functionName: "createBounty",
      args: [id, REWARD_AMOUNT, addresses.computeToken, now + 86400n, now + 604800n],
      account: account.address,
    });
    await publicClient.waitForTransactionReceipt({ hash });

    // Try to submit work
    const providerClient = makeWalletClient(
      "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    );

    await expect(
      providerClient.writeContract({
        abi: neunodeBountyAbi,
        address: addresses.neunodeBounty,
        functionName: "submitWork",
        args: [id, SUBMISSION_HASH],
        account: providerClient.account,
      }),
    ).rejects.toThrow();
  });
});
