package integration

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/api/types/network"
	"github.com/docker/go-connections/nat"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	tc "github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/exec"
	"github.com/testcontainers/testcontainers-go/network"
	"github.com/testcontainers/testcontainers-go/wait"
)

// TestMultiValidatorBFT verifies the Neunode L1 chain with three validators:
// BFT finality, fork choice resilience, and catch-up sync.
func TestMultiValidatorBFT(t *testing.T) {
	ctx := context.Background()

	// 1. Create a shared Docker network for validator communication
	net, err := network.NewNetwork(ctx, network.WithCheckDuplicate())
	require.NoError(t, err, "failed to create docker network")
	defer net.Remove(ctx)

	// 2. Start three validators (Reth + Malachite) as combined containers.
	validators := make([]tc.Container, 3)
	ports := []string{"8545", "26656", "26657"} // EL RPC, CL P2P, CL RPC
	for i := range validators {
		name := fmt.Sprintf("validator-%d", i+1)
		validatorC, err := startValidator(ctx, name, net.Name, ports, i)
		require.NoError(t, err, "failed to start %s", name)
		validators[i] = validatorC
		defer validatorC.Terminate(ctx)
	}

	// 3. Wait for the chain to reach finality (at least 5 committed blocks)
	t.Log("Waiting for BFT finality...")
	err = waitForFinality(ctx, validators, 5, 2*time.Minute)
	require.NoError(t, err, "chain did not reach finality within timeout")

	// 4. Verify all validators agree on the same block height and hash (finality)
	t.Run("BFTFinality", func(t *testing.T) {
		heights, err := fetchBlockHeights(ctx, validators)
		require.NoError(t, err)
		t.Logf("Block heights: %v", heights)
		for i := 1; i < len(heights); i++ {
			assert.Equal(t, heights[0], heights[i], "validator %d diverged on block height", i+1)
		}
		// Verify consensus on block hash via engine API (simplified: check last block hash)
		hashes, err := fetchLatestBlockHashes(ctx, validators)
		require.NoError(t, err)
		for i := 1; i < len(hashes); i++ {
			assert.Equal(t, hashes[0], hashes[i], "validator %d diverged on block hash", i+1)
		}
	})

	// 5. Fork choice test: isolate validator-3, let validators 1 and 2 produce a fork,
	//    then reconnect validator-3 and verify it adopts the canonical chain.
	t.Run("ForkChoice", func(t *testing.T) {
		// Pause validator-3
		err := validators[2].Pause(ctx)
		require.NoError(t, err, "failed to pause validator-3")
		t.Log("Validator-3 paused")

		// Wait for validators 1 and 2 to produce 3+ blocks on their fork
		time.Sleep(10 * time.Second)
		heightAfterFork, err := fetchBlockHeight(ctx, validators[0])
		require.NoError(t, err)
		t.Logf("Active validators reached height %d", heightAfterFork)

		// Resume validator-3
		err = validators[2].Unpause(ctx)
		require.NoError(t, err, "failed to unpause validator-3")
		t.Log("Validator-3 resumed")

		// Wait for it to catch up and adopt the fork
		time.Sleep(15 * time.Second)

		// Check that validator-3 now has same height as the active ones (or higher)
		heights, err := fetchBlockHeights(ctx, validators)
		require.NoError(t, err)
		t.Logf("After fork: heights = %v", heights)
		assert.GreaterOrEqual(t, heights[2], heightAfterFork,
			"validator-3 did not adopt the canonical fork")
		hashes, err := fetchLatestBlockHashes(ctx, validators)
		require.NoError(t, err)
		assert.Equal(t, hashes[0], hashes[2],
			"validator-3 does not share the same latest block hash as the active set")
	})

	// 6. Sync test: add a fourth validator and verify it catches up to current height.
	t.Run("Sync", func(t *testing.T) {
		syncValidator, err := startValidator(ctx, "validator-sync", net.Name, ports, 3)
		require.NoError(t, err, "failed to start sync validator")
		defer syncValidator.Terminate(ctx)

		// Wait for sync (up to 2 minutes)
		time.Sleep(30 * time.Second) // initial wait for p2p handshake
		err = waitForFinality(ctx, append(validators, syncValidator), 1, 60*time.Second)
		require.NoError(t, err, "sync validator did not catch up")

		heights, err := fetchBlockHeights(ctx, append(validators, syncValidator))
		require.NoError(t, err)
		t.Logf("After sync: heights = %v", heights)
		assert.InDelta(t, heights[0], heights[3], 2,
			"sync validator's height differs by more than 2 blocks from main validators")
	})
}

// startValidator starts a Neunode validator container with Reth + Malachite.
func startValidator(ctx context.Context, name, networkName string, ports []string, index int) (tc.Container, error) {
	// Use the official Neunode image (assumes built locally via Docker Compose).
	// Image definition: neunode/validator:latest
	req := tc.ContainerRequest{
		Image:        "neunode/validator:latest",
		Hostname:     name,
		Name:         name,
		ExposedPorts: ports,
		Networks: []string{networkName},
		Env: map[string]string{
			"NEUNODE_VALIDATOR_INDEX": fmt.Sprintf("%d", index),
			"NEUNODE_CHAIN_ID":        "neunode-dev-1",
			"NEUNODE_VALIDATORS":      "validator-1,validator-2,validator-3,validator-sync",
			"NEUNODE_PERSISTENT_PEERS": fmt.Sprintf("validator-1:%s,validator-2:%s,validator-3:%s,validator-sync:%s",
				"26656", "26656", "26656", "26656"),
		},
		WaitingFor: wait.ForAll(
			wait.ForHTTP("/health").WithPort(nat.Port(ports[0])),
			wait.ForLog("consensus reached height"),
		).WithDeadline(60 * time.Second),
		// Customize resources (optional)
		HostConfigModifier: func(hc *container.HostConfig) {
			hc.Memory = 512 * 1024 * 1024 // 512 MB
			hc.CPUCount = 0.5
		},
	}

	container, err := tc.GenericContainer(ctx, tc.GenericContainerRequest{
		ContainerRequest: req,
		Started:          true,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to start %s: %w", name, err)
	}
	return container, nil
}

// waitForFinality blocks until N validators report committed block height >= minHeight.
func waitForFinality(ctx context.Context, validators []tc.Container, minHeight int, timeout time.Duration) error {
	ctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	ticker := time.NewTicker(2 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return fmt.Errorf("timeout waiting for finality (min height %d)", minHeight)
		case <-ticker.C:
			ready := 0
			for _, v := range validators {
				height, err := fetchBlockHeight(ctx, v)
				if err != nil || height < minHeight {
					continue
				}
				ready++
			}
			if ready == len(validators) {
				return nil
			}
		}
	}
}

// fetchBlockHeight retrieves the current block height from a validator's JSON-RPC endpoint.
func fetchBlockHeight(ctx context.Context, container tc.Container) (int, error) {
	host, err := container.Host(ctx)
	if err != nil {
		return 0, err
	}
	port, err := container.MappedPort(ctx, "8545")
	if err != nil {
		return 0, err
	}
	url := fmt.Sprintf("http://%s:%s/eth/v1/beacon/headers/head", host, port.Port())
	req, err := http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		return 0, err
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return 0, err
	}
	defer resp.Body.Close()
	// Parse simplified JSON response (expected structure)
	var head struct {
		Header struct {
			Message struct {
				Slot string `json:"slot"`
			} `json:"message"`
		} `json:"header"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&head); err != nil {
		return 0, err
	}
	var height int
	fmt.Sscanf(head.Header.Message.Slot, "%d", &height) // slot ~ height
	return height, nil
}

// fetchBlockHeights returns block heights for all validators.
func fetchBlockHeights(ctx context.Context, validators []tc.Container) ([]int, error) {
	heights := make([]int, len(validators))
	for i, v := range validators {
		h, err := fetchBlockHeight(ctx, v)
		if err != nil {
			return nil, fmt.Errorf("validator %d: %w", i, err)
		}
		heights[i] = h
	}
	return heights, nil
}

// fetchLatestBlockHashes retrieves the latest block hash from each validator's engine API.
func fetchLatestBlockHashes(ctx context.Context, validators []tc.Container) ([]string, error) {
	hashes := make([]string, len(validators))
	for i, v := range validators {
		hash, err := fetchBlockHash(ctx, v)
		if err != nil {
			return nil, fmt.Errorf("validator %d: %w", i, err)
		}
		hashes[i] = hash
	}
	return hashes, nil
}

// fetchBlockHash gets the latest block hash from a validator.
func fetchBlockHash(ctx context.Context, container tc.Container) (string, error) {
	host, err := container.Host(ctx)
	if err != nil {
		return "", err
	}
	port, err := container.MappedPort(ctx, "8545")
	if err != nil {
		return "", err
	}
	url := fmt.Sprintf("http://%s:%s/eth/v1/beacon/headers/head", host, port.Port())
	req, err := http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		return "", err
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()
	var head struct {
		Header struct {
			Message struct {
				Hash string `json:"hash"`
			} `json:"message"`
		} `json:"header"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&head); err != nil {
		return "", err
	}
	return head.Header.Message.Hash, nil
}