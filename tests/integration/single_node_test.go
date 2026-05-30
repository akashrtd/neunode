// tests/integration/single_node_test.go
package integration

import (
	"context"
	"crypto/ecdsa"
	"fmt"
	"math/big"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"

	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
	"github.com/ethereum/go-ethereum/params"
)

// Constants for the integration test
const (
	// Default RPC port for the single-node Neunode L1 (Reth+Malachite)
	defaultRPCURL = "http://127.0.0.1:8545"
	// Pre-funded account private key for testing (hex without 0x)
	// This is a well-known test key used in dev environments.
	testPrivateKeyHex = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
	// Test contract bytecode: simple storage contract (Solidity -> bytes)
	// pragma solidity ^0.8.0; contract Storage { uint256 public value; function set(uint256 _v) public { value = _v; } }
	storageContractBytecode = "608060405234801561001057600080fd5b5060f58061001f6000396000f3fe60806040526004361060495760003560e01c806360fe47b114604e5780636d4ce63c14607857600080fd5b005b348015605957600080fd5b50607860048036036020811015606e57600080fd5b5035600055565b005b348015608357600080fd5b50608a609c565b6040518082815260200191505060405180910390f35b6000549056fea2646970667358221220e1b3b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b7b64736f6c63430007060033"

	// Gas limit for transactions
	defaultGasLimit = 200000
)

// TestSingleNodeChain is the main integration test.
// It boots a single‑node Neunode L1 (Reth + Malachite), deploys a test contract,
// sends a transaction, and verifies block production using `neu` as gas.
func TestSingleNodeChain(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
	defer cancel()

	// 1. Determine how to start the node
	rpcURL := os.Getenv("NEUNODE_RPC_URL")
	if rpcURL == "" {
		rpcURL = defaultRPCURL
		// Start the node if not already running
		startNode(t, ctx)
	}

	// 2. Connect to the chain
	client := mustDial(t, ctx, rpcURL)
	defer client.Close()

	// 3. Get the pre‑funded account
	privateKey := mustParsePrivateKey(t, testPrivateKeyHex)
	fromAddress := crypto.PubkeyToAddress(privateKey.PublicKey)

	// 4. Check that the account has balance (should be pre‑funded with `neu`)
	balance, err := client.BalanceAt(ctx, fromAddress, nil)
	if err != nil {
		t.Fatalf("failed to get balance: %v", err)
	}
	if balance.Cmp(big.NewInt(0)) == 0 {
		t.Fatal("pre‑funded account has zero balance; cannot send transactions")
	}
	t.Logf("account %s has balance: %s wei", fromAddress.Hex(), balance.String())

	// 5. Deploy the test contract
	t.Log("Deploying test contract...")
	contractAddress, tx, _, err := deployContract(ctx, client, privateKey)
	if err != nil {
		t.Fatalf("failed to deploy contract: %v", err)
	}
	t.Logf("contract deployed at %s (tx: %s)", contractAddress.Hex(), tx.Hash().Hex())

	// 6. Wait for deployment to be mined
	receipt, err := bind.WaitMined(ctx, client, tx)
	if err != nil {
		t.Fatalf("failed to mine deployment: %v", err)
	}
	if receipt.Status != types.ReceiptStatusSuccessful {
		t.Fatalf("deployment transaction failed with status %d", receipt.Status)
	}
	t.Logf("deployment confirmed in block %d", receipt.BlockNumber.Uint64())

	// 7. Send a transaction calling the contract (set value to 42)
	t.Log("Sending state‑changing transaction...")
	value := big.NewInt(42)
	setTx, err := sendSetTransaction(ctx, client, privateKey, contractAddress, value)
	if err != nil {
		t.Fatalf("failed to send set transaction: %v", err)
	}
	setReceipt, err := bind.WaitMined(ctx, client, setTx)
	if err != nil {
		t.Fatalf("failed to mine set transaction: %v", err)
	}
	if setReceipt.Status != types.ReceiptStatusSuccessful {
		t.Fatalf("set transaction failed with status %d", setReceipt.Status)
	}
	t.Logf("set transaction confirmed in block %d (gas used: %d)", setReceipt.BlockNumber.Uint64(), setReceipt.GasUsed)

	// 8. Verify block production: we should have seen at least 2 blocks (deploy + set)
	latestBlock, err := client.BlockNumber(ctx)
	if err != nil {
		t.Fatalf("failed to get latest block number: %v", err)
	}
	if latestBlock < setReceipt.BlockNumber.Uint64() {
		t.Fatalf("latest block %d is behind the set receipt block %d", latestBlock, setReceipt.BlockNumber.Uint64())
	}
	t.Logf("chain is producing blocks; latest block: %d", latestBlock)

	// 9. Verify we can read the stored value (optional sanity check)
	// We'll call the "value()" getter via eth_call
	callData := common.Hex2Bytes("6d4ce63c") // keccak256("value()")
	result, err := client.CallContract(ctx, ethereum.CallMsg{
		To:   &contractAddress,
		Data: callData,
	}, setReceipt.BlockNumber)
	if err != nil {
		t.Fatalf("failed to call value(): %v", err)
	}
	readValue := new(big.Int).SetBytes(result)
	if readValue.Cmp(value) != 0 {
		t.Fatalf("expected stored value %d, got %d", value, readValue)
	}
	t.Logf("contract value read correctly: %d", readValue)

	t.Log("✅ Single‑node chain integration test passed")
}

// startNode attempts to start the local Neunode L1 binary.
// It looks for a script or binary in standard locations.
func startNode(t *testing.T, ctx context.Context) {
	t.Helper()

	// Look for a dev startup script or binary
	paths := []string{
		"../scripts/start-single-node.sh",
		"../target/release/neud", // If using a compiled Rust binary
		"../../scripts/start-devnet.sh",
	}

	var foundPath string
	for _, p := range paths {
		abs, _ := filepath.Abs(p)
		if _, err := os.Stat(abs); err == nil {
			foundPath = abs
			break
		}
	}

	if foundPath == "" {
		// In CI, the node might already be running (e.g., via Docker Compose).
		// We'll just wait for the RPC to be available.
		t.Log("No startup script found; assuming node is already running or will be started externally")
		waitForRPC(t, ctx, defaultRPCURL, 30*time.Second)
		return
	}

	t.Logf("Starting node using %s", foundPath)
	cmd := exec.CommandContext(ctx, foundPath)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start node: %v", err)
	}

	// Wait for the RPC endpoint to become available
	waitForRPC(t, ctx, defaultRPCURL, 60*time.Second)

	// Cleanup on test end
	t.Cleanup(func() {
		if err := cmd.Process.Signal(os.Interrupt); err != nil {
			t.Logf("failed to interrupt node: %v", err)
			cmd.Process.Kill()
		}
		cmd.Wait()
	})
}

// mustDial connects to the Ethereum JSON‑RPC endpoint.
func mustDial(t *testing.T, ctx context.Context, url string) *ethclient.Client {
	t.Helper()
	client, err := ethclient.DialContext(ctx, url)
	if err != nil {
		t.Fatalf("failed to dial %s: %v", url, err)
	}
	return client
}

// mustParsePrivateKey parses a hex private key.
func mustParsePrivateKey(t *testing.T, hex string) *ecdsa.PrivateKey {
	t.Helper()
	privateKey, err := crypto.HexToECDSA(hex)
	if err != nil {
		t.Fatalf("failed to parse private key: %v", err)
	}
	return privateKey
}

// deployContract deploys the storage contract and returns its address, the deployment tx, and a bound contract.
func deployContract(ctx context.Context, client *ethclient.Client, privateKey *ecdsa.PrivateKey) (common.Address, *types.Transaction, *bind.BoundContract, error) {
	chainID, err := client.ChainID(ctx)
	if err != nil {
		return common.Address{}, nil, nil, fmt.Errorf("get chain ID: %w", err)
	}

	auth, err := bind.NewKeyedTransactorWithChainID(privateKey, chainID)
	if err != nil {
		return common.Address{}, nil, nil, fmt.Errorf("create transactor: %w", err)
	}
	auth.GasLimit = defaultGasLimit
	auth.GasPrice = big.NewInt(1 * params.GWei) // Use minimal gas price since 'neu' is cheap

	// Deploy the contract
	address, tx, contract, err := bind.DeployContract(auth, common.Hex2Bytes(storageContractBytecode), client)
	if err != nil {
		return common.Address{}, nil, nil, fmt.Errorf("deploy contract: %w", err)
	}
	return address, tx, contract, nil
}

// sendSetTransaction builds and sends a transaction to call `set(uint256)` on the contract.
func sendSetTransaction(ctx context.Context, client *ethclient.Client, privateKey *ecdsa.PrivateKey, contractAddress common.Address, value *big.Int) (*types.Transaction, error) {
	chainID, err := client.ChainID(ctx)
	if err != nil {
		return nil, fmt.Errorf("get chain ID: %w", err)
	}

	auth, err := bind.NewKeyedTransactorWithChainID(privateKey, chainID)
	if err != nil {
		return nil, fmt.Errorf("create transactor: %w", err)
	}
	auth.GasLimit = defaultGasLimit
	auth.GasPrice = big.NewInt(1 * params.GWei)

	// ABI encode `set(uint256)` call: first 4 bytes = keccak256("set(uint256)") = 0x60fe47b1
	// We'll use go‑ethereum's crypto.Keccak256 to be safe.
	methodID := crypto.Keccak256([]byte("set(uint256)"))[:4]
	// Pack argument as 32‑byte big‑endian
	argPadded := common.LeftPadBytes(value.Bytes(), 32)
	data := append(methodID, argPadded...)

	nonce, err := client.PendingNonceAt(ctx, auth.From)
	if err != nil {
		return nil, fmt.Errorf("get nonce: %w", err)
	}

	tx := types.NewTransaction(nonce, contractAddress, big.NewInt(0), auth.GasLimit, auth.GasPrice, data)
	signedTx, err := auth.Signer(auth.From, tx)
	if err != nil {
		return nil, fmt.Errorf("sign tx: %w", err)
	}

	if err := client.SendTransaction(ctx, signedTx); err != nil {
		return nil, fmt.Errorf("send tx: %w", err)
	}

	return signedTx, nil
}

// waitForRPC polls the given RPC URL until it responds or timeout.
func waitForRPC(t *testing.T, ctx context.Context, url string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		select {
		case <-ctx.Done():
			t.Fatalf("context cancelled while waiting for RPC %s: %v", url, ctx.Err())
		default:
		}

		client, err := ethclient.Dial(url)
		if err == nil {
			if _, err := client.BlockNumber(ctx); err == nil {
				client.Close()
				return
			}
			client.Close()
		}
		time.Sleep(500 * time.Millisecond)
	}
	t.Fatalf("RPC endpoint %s did not become ready within %v", url, timeout)
}

// ethereum.CallMsg is not imported directly; we need to use a small shim.
// Define a minimal type to avoid importing go-ethereum's params again.
type callMsg struct {
	To   *common.Address
	Data []byte
}

func (c callMsg) ToAddress() common.Address {
	if c.To == nil {
		return common.Address{}
	}
	return *c.To
}