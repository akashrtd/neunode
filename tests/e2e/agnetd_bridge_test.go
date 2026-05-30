package e2e

import (
	"context"
	"fmt"
	"math/big"
	"os"
	"testing"
	"time"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
	"github.com/neucloud/agnetd/bridge"
)

// compile-time check: ensure bridge.Client implements the expected interface
var _ interface {
	ListBounties(context.Context) ([]bridge.Bounty, error)
	CreateBounty(context.Context, bridge.BountyParams) (*bridge.Bounty, error)
} = (*bridge.Client)(nil)

// TestAgnetdBridgeE2E exercises the real agnetd bridge against a running L1 chain.
// Prerequisites:
//   - L1 chain (Reth+Malachite) is running and accessible via NEUNODE_L1_URL.
//   - A funded private key is provided via NEUNODE_L1_PRIVATE_KEY.
//   - The BountyManager contract is deployed at address NEUNODE_BOUNTY_CONTRACT.
//   - The agnetd bridge service is reachable (optional, test uses direct contract calls).
func TestAgnetdBridgeE2E(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping e2e test in short mode")
	}

	// ---- setup ----
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
	defer cancel()

	l1URL := mustEnv(t, "NEUNODE_L1_URL")
	pkHex := mustEnv(t, "NEUNODE_L1_PRIVATE_KEY")
	bountyAddr := mustEnv(t, "NEUNODE_BOUNTY_CONTRACT")

	client, err := ethclient.DialContext(ctx, l1URL)
	if err != nil {
		t.Fatalf("failed to dial L1 node: %v", err)
	}
	defer client.Close()

	privateKey, err := crypto.HexToECDSA(pkHex)
	if err != nil {
		t.Fatalf("invalid private key: %v", err)
	}

	chainID, err := client.ChainID(ctx)
	if err != nil {
		t.Fatalf("failed to get chain ID: %v", err)
	}

	auth, err := bind.NewKeyedTransactorWithChainID(privateKey, chainID)
	if err != nil {
		t.Fatalf("failed to create transactor: %v", err)
	}
	auth.GasLimit = 200_000

	// instantiate bridge client pointing to the BountyManager contract
	bridgeClient, err := bridge.NewClient(client, common.HexToAddress(bountyAddr))
	if err != nil {
		t.Fatalf("failed to create bridge client: %v", err)
	}

	// ---- step 1: query existing bounties ----
	t.Log("querying existing bounties...")
	initialBounties, err := bridgeClient.ListBounties(ctx)
	if err != nil {
		t.Fatalf("ListBounties failed: %v", err)
	}
	initialCount := len(initialBounties)
	t.Logf("found %d existing bounties", initialCount)

	// ---- step 2: create a new bounty ----
	t.Log("creating a new bounty...")
	reward := big.NewInt(5_000_000_000_000_000_000) // 5 neu in wei
	params := bridge.BountyParams{
		Title:       "e2e test bounty " + time.Now().Format(time.RFC3339),
		Description: "Created by TestAgnetdBridgeE2E",
		Reward:      reward,
		Deadline:    big.NewInt(time.Now().Add(48 * time.Hour).Unix()),
	}

	createdBounty, tx, err := bridgeClient.CreateBounty(ctx, params, auth)
	if err != nil {
		t.Fatalf("CreateBounty failed: %v", err)
	}
	t.Logf("bounty created with ID %s, tx hash %s", createdBounty.ID.Hex(), tx.Hash().Hex())

	// ---- step 3: wait for inclusion and confirm ----
	t.Log("waiting for transaction to be mined...")
	receipt, err := waitForReceipt(ctx, client, tx.Hash())
	if err != nil {
		t.Fatalf("waiting for receipt failed: %v", err)
	}
	if receipt.Status != types.ReceiptStatusSuccessful {
		t.Fatalf("transaction failed (status %d)", receipt.Status)
	}
	t.Logf("transaction mined in block %d", receipt.BlockNumber.Uint64())

	// ---- step 4: verify bounty is now in the list ----
	t.Log("verifying bounty inclusion...")
	updatedBounties, err := bridgeClient.ListBounties(ctx)
	if err != nil {
		t.Fatalf("ListBounties after creation failed: %v", err)
	}
	if len(updatedBounties) != initialCount+1 {
		t.Fatalf("expected %d bounties, got %d", initialCount+1, len(updatedBounties))
	}

	// Locate the newly created bounty by ID
	var found bool
	for _, b := range updatedBounties {
		if b.ID == createdBounty.ID {
			found = true
			if b.Title != params.Title {
				t.Errorf("title mismatch: got %q, want %q", b.Title, params.Title)
			}
			if b.Reward.Cmp(reward) != 0 {
				t.Errorf("reward mismatch: got %s, want %s", b.Reward, reward)
			}
			break
		}
	}
	if !found {
		t.Fatalf("created bounty (ID %s) not found in the updated list", createdBounty.ID.Hex())
	}

	t.Log("e2e test passed")
}

// mustEnv returns the value of the environment variable or fails the test.
func mustEnv(t *testing.T, key string) string {
	t.Helper()
	val := os.Getenv(key)
	if val == "" {
		t.Fatalf("required environment variable %s not set", key)
	}
	return val
}

// waitForReceipt polls the chain until the transaction receipt is available.
func waitForReceipt(ctx context.Context, client *ethclient.Client, txHash common.Hash) (*types.Receipt, error) {
	const (
		pollInterval = 2 * time.Second
		maxAttempts  = 60 // 2 min total
	)
	for i := 0; i < maxAttempts; i++ {
		receipt, err := client.TransactionReceipt(ctx, txHash)
		if err == nil {
			return receipt, nil
		}
		if err != ethereum.NotFound {
			return nil, fmt.Errorf("unexpected error waiting for receipt: %w", err)
		}
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case <-time.After(pollInterval):
		}
	}
	return nil, fmt.Errorf("transaction %s not mined after %d attempts", txHash.Hex(), maxAttempts)
}

// bridge.Client constructor wrapper – real package would do more.
// For the test to compile we assume a NewClient function exists.
// If not, replace with direct contract binding instantiation.