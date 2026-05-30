// Package main implements the Neunode L1 node, bootstrapping the execution,
// consensus, engine API, and reputation modules. It provides signal handling,
// validation, structured logging, and graceful shutdown.
//
// Architecture:
//
//	Reth EL ←→ Engine API ←→ Malachite CL (CometBFT)
//	                             ↓
//	               Reputation‑weighted validator set
//
// Every component is started in dependency order and stopped in reverse.
// The node exits with a non‑zero code if any component fails to start.
//
// # Configuration
//
// Configuration is provided via command‑line flags (see main() for defaults).
// All flags are validated before any component is constructed.
//
// # Lifecycle
//
// Components are started sequentially: reputation → consensus → execution → engine API.
// They are stopped in reverse order on SIGINT/SIGHUP/SIGTERM or any start failure.
package main

import (
	"context"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"flag"
	"fmt"
	"log/slog"
	"math/big"
	"net"
	"os"
	"os/signal"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"
)

// ---------------------------------------------------------------------------
// Core interfaces – each corresponds to a real package in production.
// ---------------------------------------------------------------------------

// executionLayer abstracts the Reth execution layer (EVM).
type executionLayer interface {
	// Start begins the execution engine. It blocks until the engine is ready
	// or returns an error. The context should carry a deadline for startup.
	Start(ctx context.Context) error
	// Stop initiates graceful shutdown. The context timeout governs how long
	// to wait before forcing termination.
	Stop(ctx context.Context) error
}

// consensusLayer abstracts the Malachite/BFT consensus layer.
type consensusLayer interface {
	Start(ctx context.Context) error
	Stop(ctx context.Context) error
}

// engineAPI abstracts the Engine API shim bridging EL and CL.
type engineAPI interface {
	Start(ctx context.Context) error
	Stop(ctx context.Context) error
}

// reputationModule computes validator voting power from five factors.
type reputationModule interface {
	Start(ctx context.Context) error
	Stop(ctx context.Context) error
}

// component is a generic interface for any lifecycle‑managed component.
type component interface {
	Name() string
	Start(ctx context.Context) error
	Stop(ctx context.Context) error
}

// componentManager orchestrates startup and shutdown of multiple components.
type componentManager struct {
	mu      sync.Mutex
	order   []component
	started []component
}

// newComponentManager creates a new componentManager ready for use.
func newComponentManager() *componentManager {
	return &componentManager{
		order:   make([]component, 0, 4),
		started: make([]component, 0, 4),
	}
}

// Add registers a component for lifecycle management. It is safe for
// concurrent calls only prior to StartAll.
func (cm *componentManager) Add(c component) {
	cm.mu.Lock()
	defer cm.mu.Unlock()
	cm.order = append(cm.order, c)
}

// StartAll starts components in order. If any fails, it stops already started ones.
func (cm *componentManager) StartAll(ctx context.Context) error {
	cm.mu.Lock()
	defer cm.mu.Unlock()

	for _, c := range cm.order {
		slog.Info("starting component", "component", c.Name())
		if err := c.Start(ctx); err != nil {
			slog.Error("component start failed",
				"component", c.Name(),
				"error", err,
			)
			// Stop already started components in reverse order.
			stopCtx, cancel := context.WithTimeout(
				context.Background(),
				shutdownTimeout,
			)
			defer cancel()
			for i := len(cm.started) - 1; i >= 0; i-- {
				if sErr := cm.started[i].Stop(stopCtx); sErr != nil {
					slog.Error("component stop error during rollback",
						"component", cm.started[i].Name(),
						"error", sErr,
					)
				}
			}
			return fmt.Errorf("component %s: %w", c.Name(), err)
		}
		cm.started = append(cm.started, c)
	}
	return nil
}

// StopAll stops all started components in reverse order.
func (cm *componentManager) StopAll(ctx context.Context) error {
	cm.mu.Lock()
	defer cm.mu.Unlock()

	var errs []error
	for i := len(cm.started) - 1; i >= 0; i-- {
		c := cm.started[i]
		slog.Info("stopping component", "component", c.Name())
		if err := c.Stop(ctx); err != nil {
			slog.Error("component stop failed",
				"component", c.Name(),
				"error", err,
			)
			errs = append(errs, fmt.Errorf("component %s: %w", c.Name(), err))
		}
	}
	return errors.Join(errs...)
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const (
	// defaultRPCPort is the default JSON‑RPC port for the Engine API.
	defaultRPCPort = "8545"
	// defaultListenAddr is the default P2P listener address.
	defaultListenAddr = "0.0.0.0:30303"
	// startupTimeout is how long to wait for each component to start.
	startupTimeout = 30 * time.Second
	// shutdownTimeout is how long to wait for each component to stop.
	shutdownTimeout = 15 * time.Second
	// privateKeyHexLen is the expected hex length of an ECDSA private key (without 0x prefix).
	privateKeyHexLen = 64
)

// nodeConfig holds all parameters needed to start a node.
// Every field is validated in Validate() before any component is created.
type nodeConfig struct {
	// Path to the genesis JSON file (must exist and be readable).
	genesisFile string

	// Directory for chain data (must exist or be creatable).
	dataDir string

	// P2P listener address in host:port form.
	listenAddr string

	// Engine API JSON‑RPC port (string for flag, validated as integer).
	rpcPort string

	// Hex‑encoded ECDSA private key (64 hex chars, unprefixed).
	validatorKey string

	// Comma‑separated list of bootstrap peer addresses (empty allowed).
	bootstrapPeers string

	// Parsed list of bootstrap peer addresses.
	bootstrapPeersList []string

	// Parsed private key (populated during Validate).
	parsedKey *ecdsa.PrivateKey
}

// Validate checks all configuration fields and returns a combined error
// listing every problem. It never panics and normalizes some fields.
func (cfg *nodeConfig) Validate() error {
	var errs []string

	// --genesis must point to an existing file
	if cfg.genesisFile == "" {
		errs = append(errs, "--genesis must not be empty")
	} else {
		if info, err := os.Stat(cfg.genesisFile); err != nil {
			errs = append(errs, fmt.Sprintf("--genesis file %q: %v", cfg.genesisFile, err))
		} else if info.IsDir() {
			errs = append(errs, fmt.Sprintf("--genesis file %q is a directory", cfg.genesisFile))
		}
	}

	// --data-dir must be creatable (we can attempt creation later)
	if cfg.dataDir == "" {
		errs = append(errs, "--data-dir must not be empty")
	} else {
		absPath, err := filepath.Abs(cfg.dataDir)
		if err != nil {
			errs = append(errs, fmt.Sprintf("--data-dir path is invalid: %v", err))
		} else {
			parent := filepath.Dir(absPath)
			if parent != "" && parent != string(filepath.Separator) {
				parentInfo, parentErr := os.Stat(parent)
				if os.IsNotExist(parentErr) || (parentErr == nil && !parentInfo.IsDir()) {
					errs = append(errs, fmt.Sprintf("parent directory %q for --data-dir does not exist", parent))
				}
			}
		}
	}

	// --listen-addr must be a valid host:port
	host, port, err := parseHostPort(cfg.listenAddr, defaultListenAddr)
	if err != nil {
		errs = append(errs, fmt.Sprintf("--listen-addr: %v", err))
	} else {
		// Replace with normalized values
		cfg.listenAddr = net.JoinHostPort(host, strconv.Itoa(port))
	}

	// --rpc-port must be a valid port number
	if portNum, err := strconv.Atoi(cfg.rpcPort); err != nil {
		errs = append(errs, fmt.Sprintf("--rpc-port %q is not a valid integer: %v", cfg.rpcPort, err))
	} else if portNum < 1 || portNum > 65535 {
		errs = append(errs, fmt.Sprintf("--rpc-port must be between 1 and 65535, got %d", portNum))
	}

	// --validator-key must be a valid hex-encoded ECDSA key
	if cfg.validatorKey == "" {
		errs = append(errs, "--validator-key must not be empty")
	} else {
		key, err := parsePrivateKey(cfg.validatorKey)
		if err != nil {
			errs = append(errs, fmt.Sprintf("--validator-key: %v", err))
		} else {
			cfg.parsedKey = key
		}
	}

	// --bootstrap-peers: split and validate each address
	if cfg.bootstrapPeers != "" {
		peers := strings.Split(cfg.bootstrapPeers, ",")
		for i, peer := range peers {
			peer = strings.TrimSpace(peer)
			if peer == "" {
				errs = append(errs, fmt.Sprintf("--bootstrap-peers[%d] is empty", i))
				continue
			}
			_, _, err := parseHostPort(peer, "")
			if err != nil {
				errs = append(errs, fmt.Sprintf("--bootstrap-peers[%d] %q: %v", i, peer, err))
			} else {
				cfg.bootstrapPeersList = append(cfg.bootstrapPeersList, peer)
			}
		}
	}

	if len(errs) > 0 {
		return errors.New("configuration errors:\n  " + strings.Join(errs, "\n  "))
	}
	return nil
}

// parseHostPort validates and returns host and port from an address string.
// If addr is empty, it uses defaultAddr as fallback. Returns host, port, error.
func parseHostPort(addr, defaultAddr string) (string, int, error) {
	if addr == "" {
		addr = defaultAddr
	}
	host, portStr, err := net.SplitHostPort(addr)
	if err != nil {
		return "", 0, fmt.Errorf("invalid host:port format: %w", err)
	}
	port, err := strconv.Atoi(portStr)
	if err != nil {
		return "", 0, fmt.Errorf("invalid port %q: %w", portStr, err)
	}
	if port < 1 || port > 65535 {
		return "", 0, fmt.Errorf("port %d out of range [1,65535]", port)
	}
	if host == "" {
		host = "0.0.0.0"
	}
	// Validate host is an IP or resolvable name
	if ip := net.ParseIP(host); ip == nil {
		// Try DNS resolution (timeout limited to avoid hangs)
		resolveCtx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		defer cancel()
		if _, err := net.DefaultResolver.LookupHost(resolveCtx, host); err != nil {
			return "", 0, fmt.Errorf("host %q is not valid: %w", host, err)
		}
	}
	return host, port, nil
}

// parsePrivateKey parses a hex string into an *ecdsa.PrivateKey.
// The input must be 64 hex characters without 0x prefix.
func parsePrivateKey(hexStr string) (*ecdsa.PrivateKey, error) {
	// Remove optional 0x prefix
	hexStr = strings.TrimPrefix(hexStr, "0x")
	hexStr = strings.TrimPrefix(hexStr, "0X")
	if len(hexStr) != privateKeyHexLen {
		return nil, fmt.Errorf("expected %d hex characters, got %d", privateKeyHexLen, len(hexStr))
	}
	keyBytes, err := hex.DecodeString(hexStr)
	if err != nil {
		return nil, fmt.Errorf("hex decode failed: %w", err)
	}
	// Use big.Int to set the scalar
	k := new(big.Int).SetBytes(keyBytes)
	if k.Sign() == 0 || k.Cmp(new(big.Int).Sub(elliptic.P256().Params().N, big.NewInt(1))) >= 0 {
		return nil, errors.New("private key scalar out of valid range")
	}
	privKey := new(ecdsa.PrivateKey)
	privKey.PublicKey.Curve = elliptic.P256()
	privKey.D = k
	privKey.PublicKey.X, privKey.PublicKey.Y = privKey.PublicKey.Curve.ScalarBaseMult(k.Bytes())
	return privKey, nil
}

// ---------------------------------------------------------------------------
// Stub component implementations (for demonstration / testing)
// ---------------------------------------------------------------------------

// simpleComponent is a minimal implementation of component for testing.
type simpleComponent struct {
	name  string
	sleep time.Duration
}

func (c *simpleComponent) Name() string { return c.name }

func (c *simpleComponent) Start(ctx context.Context) error {
	select {
	case <-time.After(c.sleep):
		slog.Info("component started", "component", c.name)
		return nil
	case <-ctx.Done():
		return fmt.Errorf("component %s: start cancelled: %w", c.name, ctx.Err())
	}
}

func (c *simpleComponent) Stop(ctx context.Context) error {
	select {
	case <-time.After(100 * time.Millisecond):
		slog.Info("component stopped", "component", c.name)
		return nil
	case <-ctx.Done():
		return fmt.Errorf("component %s: stop cancelled: %w", c.name, ctx.Err())
	}
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

func main() {
	// Configure structured JSON logging
	slog.SetDefault(slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	})))

	if err := run(); err != nil {
		slog.Error("node failed", "error", err)
		os.Exit(1)
	}
}

func run() error {
	// ----- Parse flags ---------------------------------------------------
	cfg := nodeConfig{}
	flag.StringVar(&cfg.genesisFile, "genesis", "", "Path to genesis JSON file (required)")
	flag.StringVar(&cfg.dataDir, "data-dir", "", "Chain data directory (required)")
	flag.StringVar(&cfg.listenAddr, "listen-addr", defaultListenAddr, "P2P listen address host:port")
	flag.StringVar(&cfg.rpcPort, "rpc-port", defaultRPCPort, "Engine API JSON-RPC port")
	flag.StringVar(&cfg.validatorKey, "validator-key", "", "Hex-encoded ECDSA validator private key (64 hex chars, no 0x)")
	flag.StringVar(&cfg.bootstrapPeers, "bootstrap-peers", "", "Comma-separated list of bootstrap peer addresses")
	flag.Parse()

	// ----- Validate configuration ----------------------------------------
	if err := cfg.Validate(); err != nil {
		return fmt.Errorf("invalid configuration: %w", err)
	}

	// ----- Prepare lifecycle manager -------------------------------------
	cm := newComponentManager()

	// Add real components here; for now we use stubs.
	// In production: create reputation, consensus, execution, engineAPI instances
	reputation := &simpleComponent{name: "reputation", sleep: 1 * time.Second}
	consensus := &simpleComponent{name: "consensus (Malachite)", sleep: 2 * time.Second}
	execution := &simpleComponent{name: "execution (Reth)", sleep: 3 * time.Second}
	engine := &simpleComponent{name: "engine API", sleep: 500 * time.Millisecond}

	// Order: reputation → consensus → execution → engine API
	cm.Add(reputation)
	cm.Add(consensus)
	cm.Add(execution)
	cm.Add(engine)

	// ----- Signal handling ------------------------------------------------
	ctx, cancel := context.WithTimeout(context.Background(), startupTimeout)
	defer cancel()

	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGHUP, syscall.SIGTERM)

	go func() {
		select {
		case sig := <-sigCh:
			slog.Warn("signal received, initiating graceful shutdown", "signal", sig)
			cancel()
		case <-ctx.Done():
			// Timeout or manual cancel
		}
	}()

	// ----- Start all components -------------------------------------------
	if err := cm.StartAll(ctx); err != nil {
		slog.Error("failed to start all components", "error", err)
		// cm.StartAll already stops started components on failure
		return err
	}

	slog.Info("all components started successfully",
		"listen_addr", cfg.listenAddr,
		"rpc_port", cfg.rpcPort,
		"data_dir", cfg.dataDir,
	)

	// ----- Wait for shutdown signal ---------------------------------------
	// Block until context is cancelled (signal or timeout)
	<-ctx.Done()
	slog.Info("shutdown requested",
		"reason", context.Cause(ctx),
	)

	// ----- Graceful shutdown with timeout ---------------------------------
	shutdownCtx, shutdownCancel := context.WithTimeout(
		context.Background(),
		shutdownTimeout,
	)
	defer shutdownCancel()

	if err := cm.StopAll(shutdownCtx); err != nil {
		slog.Error("errors during graceful shutdown", "error", err)
		return err
	}

	slog.Info("node shutdown complete")
	return nil
}