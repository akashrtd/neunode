package consensus

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"sync"
	"time"

	"github.com/cometbft/cometbft/abci/types"
	cmtcfg "github.com/cometbft/cometbft/config"
	cmtcrypto "github.com/cometbft/cometbft/crypto"
	cmted25519 "github.com/cometbft/cometbft/crypto/ed25519"
	cmtnode "github.com/cometbft/cometbft/node"
	cmtp2p "github.com/cometbft/cometbft/p2p"
	cmtproto "github.com/cometbft/cometbft/proto/tendermint/types"
	cmttypes "github.com/cometbft/cometbft/types"
	"go.uber.org/zap"
)

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const (
	// DefaultMinimumValidatorPower is the minimum voting power assigned to a
	// validator when the reputation module returns zero or an error.
	DefaultMinimumValidatorPower int64 = 1

	// shutdownTimeout is the maximum duration to wait for the CometBFT node to
	// stop gracefully.
	shutdownTimeout = 30 * time.Second

	// maxValidatorUpdateSize limits the number of validator updates per block
	// to prevent excessively large blocks.
	maxValidatorUpdateSize = 100

	// defaultNodeKeyAlgo is the cryptographic algorithm used for generating
	// node identity keys.
	defaultNodeKeyAlgo = cmted25519.KeyType

	// hexPublicKeyLength is the expected length of a hex-encoded Ed25519
	// public key (32 bytes * 2 hex chars).
	hexPublicKeyLength = 64

	// reputationQueryTimeout sets the maximum time for a single reputation
	// module query.
	reputationQueryTimeout = 5 * time.Second

	// validatorSetCacheTTL defines how long the cached validator set remains
	// valid before a fresh fetch from the reputation module is required.
	validatorSetCacheTTL = 30 * time.Second
)

// ---------------------------------------------------------------------------
// Sentinel errors
// ---------------------------------------------------------------------------

var (
	ErrEngineNil                       = errors.New("engine is nil")
	ErrContextNil                      = errors.New("context cannot be nil")
	ErrEngineAlreadyRunning            = errors.New("consensus engine is already running")
	ErrEngineNotRunning                = errors.New("consensus engine is not running")
	ErrCometBFTConfigRequired          = errors.New("cometBFT configuration is required")
	ErrReputationModuleRequired        = errors.New("reputation module is required")
	ErrInvalidNodeKey                  = errors.New("provided node key is invalid: empty private key")
	ErrGenesisDocMissing               = errors.New("genesis document missing in CometBFT config")
	ErrInvalidValidatorPubKeyHex       = errors.New("invalid validator public key hex encoding")
	ErrFailedToCreateDefaultLogger     = errors.New("failed to create default zap logger")
	ErrFailedToGenerateNodeKey         = errors.New("failed to generate node key")
	ErrFailedToCreateCometBFTNode      = errors.New("cannot create CometBFT node")
	ErrFailedToStartCometBFTNode       = errors.New("cannot start CometBFT node")
	ErrFailedToStopCometBFTNode        = errors.New("cannot stop CometBFT node")
	ErrFailedToFetchValidatorSet       = errors.New("failed to fetch initial validator set")
	ErrInvalidPublicKeyBytes           = errors.New("failed to parse validator public key bytes")
	ErrReputationModuleNil             = errors.New("reputation module is nil")
	ErrInvalidValidatorSetEntry        = errors.New("invalid entry in validator set")
	ErrMaxUpdatesExceeded              = errors.New("validator update count exceeds maximum")
)

// ---------------------------------------------------------------------------
// ReputationModule
// ---------------------------------------------------------------------------

// ReputationModule provides reputation‑weighted voting power for validators.
// All implementations must be safe for concurrent use.
type ReputationModule interface {
	// VotingPower returns the voting power for the validator identified by
	// the raw public key bytes. Returns 0 and an error if the validator is
	// unknown or the query fails.
	VotingPower(ctx context.Context, validatorID []byte) (int64, error)

	// ValidatorSet returns the current full set of validators with their
	// voting powers. The map key is the hex‑encoded, lowercase public key
	// bytes (no prefix). This is called during genesis and may be called
	// periodically to resync the validator set.
	ValidatorSet(ctx context.Context) (map[string]int64, error)
}

// ---------------------------------------------------------------------------
// EngineConfig
// ---------------------------------------------------------------------------

// EngineConfig holds all configuration necessary to create and run the
// Malachite (CometBFT) consensus engine.
type EngineConfig struct {
	// CometBFT is the full CometBFT configuration tree.
	CometBFT *cmtcfg.Config

	// Reputation is the module used to query per‑validator voting power.
	Reputation ReputationModule

	// Logger is the structured logger. If nil, a production logger is created.
	Logger *zap.Logger

	// NodeKey is the optional node identity key. If nil, a new ephemeral key
	// is generated at startup.
	NodeKey *cmtp2p.NodeKey
}

// NewEngineConfig creates an EngineConfig with sensible defaults and validates
// invariants. It returns an error if CometBFT config or Reputation is nil.
func NewEngineConfig(cmtConfig *cmtcfg.Config, rep ReputationModule) (*EngineConfig, error) {
	if cmtConfig == nil {
		return nil, ErrCometBFTConfigRequired
	}
	if rep == nil {
		return nil, ErrReputationModuleRequired
	}
	return &EngineConfig{
		CometBFT:   cmtConfig,
		Reputation: rep,
		Logger:     nil, // caller may set later
		NodeKey:    nil,
	}, nil
}

// Validate checks that the EngineConfig is valid and returns an error if not.
func (cfg *EngineConfig) Validate() error {
	if cfg.CometBFT == nil {
		return ErrCometBFTConfigRequired
	}
	if cfg.Reputation == nil {
		return ErrReputationModuleRequired
	}
	if cfg.CometBFT.Genesis == nil {
		return ErrGenesisDocMissing
	}
	if cfg.NodeKey != nil && len(cfg.NodeKey.PrivKey.Bytes()) == 0 {
		return ErrInvalidNodeKey
	}
	return nil
}

// ---------------------------------------------------------------------------
// reputationApp — internal ABCI application
// ---------------------------------------------------------------------------

// reputationApp implements the CometBFT ABCI application interface.
// It queries the ReputationModule to generate validator set updates.
type reputationApp struct {
	types.BaseApplication

	rep ReputationModule
	logger *zap.Logger

	mu            sync.RWMutex
	validatorSet  map[string]int64 // hex public key → voting power
	cachedSetHash [sha256.Size]byte
	cacheTime     time.Time
}

// newReputationApp creates a new ABCI application backed by the given
// reputation module.
func newReputationApp(rep ReputationModule, logger *zap.Logger) *reputationApp {
	if rep == nil {
		panic("reputation module must not be nil")
	}
	appLogger := logger.With(
		zap.String("component", "abci-app"),
		zap.String("type", "reputation"),
	)
	return &reputationApp{
		rep:          rep,
		logger:       appLogger,
		validatorSet: make(map[string]int64),
	}
}

// fetchValidatorSet fetches the latest set from the reputation module.
// It returns a copy of the map to avoid data races.
func (app *reputationApp) fetchValidatorSet(ctx context.Context) (map[string]int64, error) {
	queryCtx, cancel := context.WithTimeout(ctx, reputationQueryTimeout)
	defer cancel()

	rawSet, err := app.rep.ValidatorSet(queryCtx)
	if err != nil {
		return nil, fmt.Errorf("%w: %w", ErrFailedToFetchValidatorSet, err)
	}

	// Defensive copy and validation
	clone := make(map[string]int64, len(rawSet))
	for pubKeyHex, power := range rawSet {
		if len(pubKeyHex) != hexPublicKeyLength {
			app.logger.Warn("ignoring validator with invalid public key length",
				zap.String("key", pubKeyHex),
				zap.Int("expected_length", hexPublicKeyLength),
				zap.Int("actual_length", len(pubKeyHex)),
			)
			continue
		}
		if _, err := hex.DecodeString(pubKeyHex); err != nil {
			app.logger.Warn("ignoring validator with non‑hex public key",
				zap.String("key", pubKeyHex),
				zap.Error(err),
			)
			continue
		}
		if power < 0 {
			app.logger.Warn("negative voting power clamped to zero",
				zap.String("key", pubKeyHex),
				zap.Int64("power", power),
			)
			power = 0
		}
		clone[pubKeyHex] = power
	}
	return clone, nil
}

// InitChain initialises the validator set on chain start.
func (app *reputationApp) InitChain(ctx context.Context, req *types.RequestInitChain) (*types.ResponseInitChain, error) {
	app.logger.Info("initialising chain with reputation validator set")
	initSet, err := app.fetchValidatorSet(ctx)
	if err != nil {
		return nil, err
	}

	app.mu.Lock()
	app.validatorSet = initSet
	app.cacheTime = time.Now()
	// compute initial hash
	hash := sha256.New()
	for k, v := range initSet {
		hash.Write([]byte(k))
		hash.Write([]byte(fmt.Sprintf("%d", v)))
	}
	copy(app.cachedSetHash[:], hash.Sum(nil))
	app.mu.Unlock()

	app.logger.Info("validator set initialised",
		zap.Int("count", len(initSet)),
		zap.Duration("ttl", validatorSetCacheTTL),
	)

	return &types.ResponseInitChain{
		Validators: app.buildValidatorUpdates(initSet),
	}, nil
}

// FinalizeBlock returns validator updates based on the current validator set.
// It queries the reputation module only if the cached set is stale.
func (app *reputationApp) FinalizeBlock(ctx context.Context, req *types.RequestFinalizeBlock) (*types.ResponseFinalizeBlock, error) {
	app.mu.RLock()
	lastSet := app.validatorSet
	lastTime := app.cacheTime
	lastHash := app.cachedSetHash
	app.mu.RUnlock()

	// If cache is still fresh, return no updates (empty).
	if time.Since(lastTime) < validatorSetCacheTTL {
		return &types.ResponseFinalizeBlock{
			ValidatorUpdates: nil,
		}, nil
	}

	newSet, err := app.fetchValidatorSet(ctx)
	if err != nil {
		app.logger.Error("failed to fetch validator set, reusing cached set",
			zap.Error(err),
		)
		return &types.ResponseFinalizeBlock{
			ValidatorUpdates: nil,
		}, nil
	}

	// Compute hash to detect changes
	hash := sha256.New()
	for k, v := range newSet {
		hash.Write([]byte(k))
		hash.Write([]byte(fmt.Sprintf("%d", v)))
	}
	var newHash [sha256.Size]byte
	copy(newHash[:], hash.Sum(nil))

	app.mu.Lock()
	app.validatorSet = newSet
	app.cachedSetHash = newHash
	app.cacheTime = time.Now()
	app.mu.Unlock()

	// If the set is identical, return no updates.
	if newHash == lastHash {
		return &types.ResponseFinalizeBlock{
			ValidatorUpdates: nil,
		}, nil
	}

	updates := app.computeValidatorUpdates(lastSet, newSet)
	if len(updates) > maxValidatorUpdateSize {
		app.logger.Warn("validator updates exceed limit, truncating",
			zap.Int("count", len(updates)),
			zap.Int("limit", maxValidatorUpdateSize),
		)
		updates = updates[:maxValidatorUpdateSize]
	}

	app.logger.Debug("validator set updated",
		zap.Int("previous_count", len(lastSet)),
		zap.Int("new_count", len(newSet)),
		zap.Int("updates", len(updates)),
	)

	return &types.ResponseFinalizeBlock{
		ValidatorUpdates: updates,
	}, nil
}

// buildValidatorUpdates converts a validator set map into a slice of
// ValidatorUpdate proto messages. Zero‑power validators are excluded.
func (app *reputationApp) buildValidatorUpdates(set map[string]int64) []cmttypes.ValidatorUpdate {
	updates := make([]cmttypes.ValidatorUpdate, 0, len(set))
	for pubKeyHex, power := range set {
		if power <= 0 {
			continue
		}
		pubKeyBytes, err := hex.DecodeString(pubKeyHex)
		if err != nil {
			app.logger.Warn("skipping validator update due to hex decode error",
				zap.String("key", pubKeyHex),
				zap.Error(err),
			)
			continue
		}
		var pubKey cmtcrypto.PubKey
		pubKey, err = cmted25519.PubKeyFromBytes(pubKeyBytes)
		if err != nil {
			app.logger.Warn("skipping validator update due to invalid public key",
				zap.String("key", pubKeyHex),
				zap.Error(err),
			)
			continue
		}
		updates = append(updates, cmttypes.ValidatorUpdate{
			PubKey: cmtproto.PublicKey{
				Sum: &cmtproto.PublicKey_Ed25519{
					Ed25519: pubKey.Bytes(), // raw 32 bytes
				},
			},
			Power: power,
		})
	}
	return updates
}

// computeValidatorUpdates computes the diff between old and new validator
// sets, returning only updates that changed. Entries with power <= 0 are
// treated as removals (power = 0).
func (app *reputationApp) computeValidatorUpdates(oldSet, newSet map[string]int64) []cmttypes.ValidatorUpdate {
	// Collect all keys
	allKeys := make(map[string]struct{}, len(oldSet)+len(newSet))
	for k := range oldSet {
		allKeys[k] = struct{}{}
	}
	for k := range newSet {
		allKeys[k] = struct{}{}
	}

	updates := make([]cmttypes.ValidatorUpdate, 0, len(allKeys))
	for key := range allKeys {
		oldPower := oldSet[key]
		newPower := newSet[key]
		if oldPower == newPower {
			continue
		}
		// build proto update
		pubKeyBytes, err := hex.DecodeString(key)
		if err != nil {
			app.logger.Warn("skipping validator update due to hex decode error",
				zap.String("key", key),
				zap.Error(err),
			)
			continue
		}
		pubKey, err := cmted25519.PubKeyFromBytes(pubKeyBytes)
		if err != nil {
			app.logger.Warn("skipping validator update due to invalid public key",
				zap.String("key", key),
				zap.Error(err),
			)
			continue
		}
		// Use newPower (may be 0 for removal)
		updates = append(updates, cmttypes.ValidatorUpdate{
			PubKey: cmtproto.PublicKey{
				Sum: &cmtproto.PublicKey_Ed25519{
					Ed25519: pubKey.Bytes(),
				},
			},
			Power: newPower,
		})
	}
	return updates
}

// ---------------------------------------------------------------------------
// Engine — CometBFT consensus engine
// ---------------------------------------------------------------------------

// Engine is a managed CometBFT node with reputation‑weighted validator updates.
type Engine struct {
	config *EngineConfig
	app    *reputationApp
	node   *cmtnode.Node
	logger *zap.Logger

	mu      sync.Mutex
	running bool
	stopped chan struct{}
}

// NewEngine creates a new Engine from the given configuration.
func NewEngine(cfg *EngineConfig) (*Engine, error) {
	if err := cfg.Validate(); err != nil {
		return nil, fmt.Errorf("invalid engine config: %w", err)
	}

	logger := cfg.Logger
	if logger == nil {
		var err error
		logger, err = zap.NewProduction()
		if err != nil {
			return nil, fmt.Errorf("%w: %w", ErrFailedToCreateDefaultLogger, err)
		}
	}

	// Ensure a node key exists
	nodeKey := cfg.NodeKey
	if nodeKey == nil {
		var err error
		nodeKey, err = cmtp2p.GenNodeKey(defaultNodeKeyAlgo)
		if err != nil {
			return nil, fmt.Errorf("%w: %w", ErrFailedToGenerateNodeKey, err)
		}
	}

	app := newReputationApp(cfg.Reputation, logger)
	genesisDoc := cfg.CometBFT.Genesis

	// Build the CometBFT node
	node, err := cmtnode.NewNode(
		cfg.CometBFT,
		cmtp2p.LoadOrGenNodeKey(cfg.CometBFT.NodeKeyFile()), // fallback if empty
		genesisDoc.Validators, // initial validators (from genesis)
	)
	if err != nil {
		return nil, fmt.Errorf("%w: %w", ErrFailedToCreateCometBFTNode, err)
	}

	engine := &Engine{
		config:  cfg,
		app:     app,
		node:    node,
		logger:  logger.With(zap.String("component", "consensus-engine")),
		stopped: make(chan struct{}),
	}

	return engine, nil
}

// Start starts the CometBFT node. It blocks until the node is running or an
// error occurs. Returns immediately if the engine is already running.
func (e *Engine) Start(ctx context.Context) error {
	if ctx == nil {
		return ErrContextNil
	}

	e.mu.Lock()
	if e.running {
		e.mu.Unlock()
		return ErrEngineAlreadyRunning
	}
	e.mu.Unlock()

	e.logger.Info("starting consensus engine")

	if err := e.node.Start(); err != nil {
		return fmt.Errorf("%w: %w", ErrFailedToStartCometBFTNode, err)
	}

	e.mu.Lock()
	e.running = true
	e.mu.Unlock()

	// Wait for node to be ready or context cancelled
	select {
	case <-e.node.Ready():
		e.logger.Info("consensus engine is ready")
	case <-ctx.Done():
		e.Stop() // clean up
		return ctx.Err()
	case <-time.After(30 * time.Second):
		e.Stop()
		return errors.New("consensus engine did not become ready within 30 seconds")
	}

	return nil
}

// Stop gracefully stops the CometBFT node. It returns an error if the engine
// is not running or if the stop fails.
func (e *Engine) Stop() error {
	e.mu.Lock()
	defer e.mu.Unlock()
	if !e.running {
		return ErrEngineNotRunning
	}

	e.logger.Info("stopping consensus engine")
	e.node.Stop()
	e.node.Wait()

	// Wait for the node to fully stop
	select {
	case <-e.node.Quit():
		e.logger.Info("consensus engine stopped")
	case <-time.After(shutdownTimeout):
		e.logger.Warn("consensus engine stop timed out")
		return fmt.Errorf("%w: timeout after %v", ErrFailedToStopCometBFTNode, shutdownTimeout)
	}

	e.running = false
	close(e.stopped)
	return nil
}

// IsRunning returns whether the engine is currently running.
func (e *Engine) IsRunning() bool {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.running
}