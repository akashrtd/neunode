/**
 * E2E tests for the 4 Neunode resource-backed token contracts.
 *
 * Covers: ERC-20 basics, minting, transfers, staking, activity tracking, seed tokens.
 * All tests run against Anvil via `npm run test:e2e`.
 */

import { describe, it, expect } from "vitest";
import { createWalletClient, http } from "viem";
import { foundry } from "viem/chains";
import { privateKeyToAccount } from "viem/accounts";
import { useE2E } from "./helpers/fixtures.js";
import { neunodeTokenAbi } from "../../src/contracts/abi/neunode-token.js";
import { ANVIL_ACCOUNTS } from "./helpers/anvil.js";

const { getFixture } = useE2E();

/** Anvil account 1 private key (non-owner) */
const ACCOUNT_1_PK = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as const;

// ────────────────────────────────────────────────────────────────────────────────
// 1. ERC-20 Basics
// ────────────────────────────────────────────────────────────────────────────────

describe("ERC-20 basics", () => {
  it("name() returns correct name for each token", async () => {
    const f = getFixture();

    const computeName = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "name",
    });
    expect(computeName).toBe("Neunode Compute");

    const trainingName = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.trainingToken,
      functionName: "name",
    });
    expect(trainingName).toBe("Neunode Training");

    const bandwidthName = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.bandwidthToken,
      functionName: "name",
    });
    expect(bandwidthName).toBe("Neunode Bandwidth");

    const storageName = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.storageToken,
      functionName: "name",
    });
    expect(storageName).toBe("Neunode Storage");
  });

  it("symbol() returns correct symbol for each token", async () => {
    const f = getFixture();

    const computeSymbol = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "symbol",
    });
    expect(computeSymbol).toBe("nCompute");

    const trainingSymbol = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.trainingToken,
      functionName: "symbol",
    });
    expect(trainingSymbol).toBe("nTrain");

    const bandwidthSymbol = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.bandwidthToken,
      functionName: "symbol",
    });
    expect(bandwidthSymbol).toBe("nBandwidth");

    const storageSymbol = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.storageToken,
      functionName: "symbol",
    });
    expect(storageSymbol).toBe("nStorage");
  });

  it("decimals() returns 18 for all tokens", async () => {
    const f = getFixture();

    for (const addr of [
      f.addresses.computeToken,
      f.addresses.trainingToken,
      f.addresses.bandwidthToken,
      f.addresses.storageToken,
    ]) {
      const decimals = await f.publicClient.readContract({
        abi: neunodeTokenAbi,
        address: addr,
        functionName: "decimals",
      });
      expect(decimals).toBe(18);
    }
  });
});

// ────────────────────────────────────────────────────────────────────────────────
// 2. Minting
// ────────────────────────────────────────────────────────────────────────────────

describe("Minting", () => {
  it("owner can mint tokens to any address", async () => {
    const f = getFixture();
    const recipient = ANVIL_ACCOUNTS[1] as `0x${string}`;
    const amount = 1000n;

    const hash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "mint",
      args: [recipient, amount],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash });

    const balance = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "balanceOf",
      args: [recipient],
    });
    expect(balance).toBe(amount);
  });

  it("non-owner cannot mint", async () => {
    const f = getFixture();

    const nonOwnerAccount = privateKeyToAccount(ACCOUNT_1_PK);
    const nonOwnerClient = createWalletClient({
      account: nonOwnerAccount,
      chain: foundry,
      transport: http("http://127.0.0.1:8545"),
    });

    await expect(
      nonOwnerClient.writeContract({
        abi: neunodeTokenAbi,
        address: f.addresses.computeToken,
        functionName: "mint",
        args: [nonOwnerAccount.address, 100n],
        account: nonOwnerAccount,
      }),
    ).rejects.toThrow();
  });
});

// ────────────────────────────────────────────────────────────────────────────────
// 3. Transfer
// ────────────────────────────────────────────────────────────────────────────────

describe("Transfer", () => {
  it("transfers tokens between accounts", async () => {
    const f = getFixture();
    const recipient = ANVIL_ACCOUNTS[1] as `0x${string}`;
    const mintAmount = 5000n;
    const transferAmount = 2000n;

    // Mint to owner first
    const mintHash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.trainingToken,
      functionName: "mint",
      args: [f.account.address, mintAmount],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: mintHash });

    // Transfer from owner to account 1
    const transferHash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.trainingToken,
      functionName: "transfer",
      args: [recipient, transferAmount],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: transferHash });

    const recipientBalance = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.trainingToken,
      functionName: "balanceOf",
      args: [recipient],
    });
    expect(recipientBalance).toBe(transferAmount);
  });

  it("transfer fails with insufficient balance", async () => {
    const f = getFixture();
    const recipient = ANVIL_ACCOUNTS[1] as `0x${string}`;

    // Owner has no bandwidth tokens — should revert
    await expect(
      f.walletClient.writeContract({
        abi: neunodeTokenAbi,
        address: f.addresses.bandwidthToken,
        functionName: "transfer",
        args: [recipient, 100n],
        account: f.account,
      }),
    ).rejects.toThrow();
  });
});

// ────────────────────────────────────────────────────────────────────────────────
// 4. Staking
// ────────────────────────────────────────────────────────────────────────────────

describe("Staking", () => {
  it("stake(amount) moves tokens to staked balance", async () => {
    const f = getFixture();
    const mintAmount = 10000n;
    const stakeAmount = 3000n;

    // Mint tokens to owner
    const mintHash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.storageToken,
      functionName: "mint",
      args: [f.account.address, mintAmount],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: mintHash });

    // Stake
    const stakeHash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.storageToken,
      functionName: "stake",
      args: [stakeAmount],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: stakeHash });

    // Liquid balance reduced
    const liquidBalance = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.storageToken,
      functionName: "balanceOf",
      args: [f.account.address],
    });
    expect(liquidBalance).toBe(mintAmount - stakeAmount);

    // Staked balance increased
    const stakedBalance = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.storageToken,
      functionName: "stakedBalanceOf",
      args: [f.account.address],
    });
    expect(stakedBalance).toBe(stakeAmount);
  });

  it("stakedBalanceOf(account) reflects staked amount", async () => {
    const f = getFixture();

    // Initially zero
    const initialStaked = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "stakedBalanceOf",
      args: [f.account.address],
    });
    expect(initialStaked).toBe(0n);

    // Mint + stake
    const mintHash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "mint",
      args: [f.account.address, 5000n],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: mintHash });

    const stakeHash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "stake",
      args: [5000n],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: stakeHash });

    const staked = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "stakedBalanceOf",
      args: [f.account.address],
    });
    expect(staked).toBe(5000n);
  });

  it("unstake(amount) returns tokens from staked balance", async () => {
    const f = getFixture();
    const mintAmount = 10000n;
    const stakeAmount = 6000n;
    const unstakeAmount = 2500n;

    // Mint + stake
    const mintHash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.bandwidthToken,
      functionName: "mint",
      args: [f.account.address, mintAmount],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: mintHash });

    const stakeHash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.bandwidthToken,
      functionName: "stake",
      args: [stakeAmount],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: stakeHash });

    // Unstake
    const unstakeHash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.bandwidthToken,
      functionName: "unstake",
      args: [unstakeAmount],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash: unstakeHash });

    const stakedBalance = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.bandwidthToken,
      functionName: "stakedBalanceOf",
      args: [f.account.address],
    });
    expect(stakedBalance).toBe(stakeAmount - unstakeAmount);

    const liquidBalance = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.bandwidthToken,
      functionName: "balanceOf",
      args: [f.account.address],
    });
    expect(liquidBalance).toBe(mintAmount - stakeAmount + unstakeAmount);
  });
});

// ────────────────────────────────────────────────────────────────────────────────
// 5. Activity Tracking
// ────────────────────────────────────────────────────────────────────────────────

describe("Activity tracking", () => {
  it("updateActivity(account) updates lastActivity timestamp", async () => {
    const f = getFixture();
    const account = ANVIL_ACCOUNTS[1] as `0x${string}`;

    // Before: no activity recorded
    const before = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "lastActivity",
      args: [account],
    });
    expect(before).toBe(0n);

    // Update activity
    const hash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "updateActivity",
      args: [account],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash });

    // After: timestamp > 0
    const after = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "lastActivity",
      args: [account],
    });
    expect(after).toBeGreaterThan(0n);
  });

  it("getActivityLevel(account) returns correct level", async () => {
    const f = getFixture();

    // Never-active account → Dead (level 4)
    const deadLevel = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "getActivityLevel",
      args: [ANVIL_ACCOUNTS[2] as `0x${string}`],
    });
    expect(deadLevel).toBe(4);

    // Update activity → Active (level 0) since block.timestamp is "now"
    const hash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "updateActivity",
      args: [f.account.address],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash });

    const activeLevel = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "getActivityLevel",
      args: [f.account.address],
    });
    expect(activeLevel).toBe(0);
  });
});

// ────────────────────────────────────────────────────────────────────────────────
// 6. Seed Tokens
// ────────────────────────────────────────────────────────────────────────────────

describe("Seed tokens", () => {
  it("owner can mintSeed() to any address", async () => {
    const f = getFixture();
    const recipient = ANVIL_ACCOUNTS[1] as `0x${string}`;
    const seedAmount = 500n;

    const hash = await f.walletClient.writeContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "mintSeed",
      args: [recipient, seedAmount],
      account: f.account,
    });
    await f.publicClient.waitForTransactionReceipt({ hash });

    // Seed balance recorded
    const seedBalance = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "seedBalanceOf",
      args: [recipient],
    });
    expect(seedBalance).toBe(seedAmount);

    // Tokens are staked (not in liquid balance)
    const stakedBalance = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "stakedBalanceOf",
      args: [recipient],
    });
    expect(stakedBalance).toBe(seedAmount);

    const liquidBalance = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.computeToken,
      functionName: "balanceOf",
      args: [recipient],
    });
    expect(liquidBalance).toBe(0n);
  });

  it("seedBalanceOf() returns 0 for accounts with no seed tokens", async () => {
    const f = getFixture();

    const seedBalance = await f.publicClient.readContract({
      abi: neunodeTokenAbi,
      address: f.addresses.trainingToken,
      functionName: "seedBalanceOf",
      args: [f.account.address],
    });
    expect(seedBalance).toBe(0n);
  });
});
