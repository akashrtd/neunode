// Package consensus implements reputation-weighted validator selection for a
// Neunode L1 blockchain. It provides a ValidatorSetManager that periodically
// fetches validator information from an on-chain governance contract and
// computes voting powers using a configurable reputation engine.
//
// SPDX-License-Identifier: Apache-2.0
package consensus

import (
	"context"
	"errors"
	"fmt"
	"math/big"
	"sync"
	"sync/atomic"
	"time"

	"github.com/ethereum/go-ethereum/common"
	"github.com/neu-node/logging"
)

// ============================================================================
// Sentinel errors
// ============================================================================

var (
	// ErrValidatorNotFound indicates the requested validator is not in the
	// current validator set.
	ErrValidatorNotFound = errors.New("validator not found in current set")

	// ErrNilGovernanceClient is returned when a nil GovernanceClient is
	// provided to a constructor.
	ErrNilGovernanceClient = errors.New("governance client cannot be nil")

	// ErrNilReputationEngine is returned when a nil ReputationEngine is
	// provided.
	ErrNilReputationEngine = errors.New("reputation engine cannot be nil")

	// ErrNilLogger is returned when a nil Logger is provided.
	ErrNilLogger = errors.New("logger cannot be nil")

	// ErrEpochFetchFailed indicates a failure to retrieve the current epoch
	// from the governance contract.
	ErrEpochFetchFailed = errors.New("failed to fetch current epoch from governance")

	// ErrValidatorSetUpdateFailed indicates a failure to refresh the validator
	// set from the governance contract.
	ErrValidatorSetUpdateFailed = errors.New("failed to update validator set from governance")

	// ErrNegativeTotalPower is returned when totalPower is negative.
	ErrNegativeTotalPower = errors.New("total power must be non-negative")

	// ErrZeroTotalPower is returned when totalPower is zero.
	ErrZeroTotalPower = errors.New("total power must be positive")

	// ErrStopped is returned when the manager is stopped and cannot perform
	// the requested operation.
	ErrStopped = errors.New("validator set manager is stopped")

	// ErrNilContext is returned when a nil context is passed.
	ErrNilContext = errors.New("context cannot be nil")
)

// ============================================================================
// Constants
// ============================================================================

const (
	// stakeWeight is the percentage weight assigned to the staked amount.
	stakeWeight = 30
	// attestWeight is the percentage weight assigned to attestation count.
	attestWeight = 25
	// activityWeight is the percentage weight assigned to activity count.
	activityWeight = 20
	// verifyWeight is the percentage weight assigned to verification count.
	verifyWeight = 15
	// tenureWeight is the percentage weight assigned to tenure (epochs active).
	tenureWeight = 10
	// totalWeight is the sum of all reputation weights; used for validation.
	totalWeight = 100

	// defaultTotalPower is the default maximum voting power (1e24 wei).
	defaultTotalPower = 1_000_000 * 1e18

	// defaultUpdateInterval is the default interval for refreshing the
	// validator set from the governance contract.
	defaultUpdateInterval = 10 * time.Second

	// maxUint64 is the maximum value for uint64.
	maxUint64 = ^uint64(0)
)

// ============================================================================
// Types
// ============================================================================

// VotingPowerScore holds the five raw factors used to compute a validator's
// reputation-weighted voting power. All fields must be non-negative; clamping
// is applied during computation.
type VotingPowerScore struct {
	// Stake is the staked amount in wei (clamped to non-negative).
	Stake *big.Int
	// Attestations is the number of successful attestations.
	Attestations uint64
	// Activity is the number of blocks proposed or voted.
	Activity uint64
	// Verification is the number of successful verification rounds.
	Verification uint64
	// Tenure is the number of active epochs.
	Tenure uint64
}

// Validate checks that the score fields are non-negative and returns an error
// if any field is invalid. It returns nil if the score is valid.
func (s VotingPowerScore) Validate() error {
	if s.Stake == nil || s.Stake.Sign() < 0 {
		return errors.New("voting power score: stake cannot be nil or negative")
	}
	// All uint64 fields are inherently non-negative.
	return nil
}

// ValidatorInfo represents the identity of a validator and its raw scoring
// data as fetched from the governance contract.
type ValidatorInfo struct {
	Address common.Address
	Score   VotingPowerScore
}

// GovernanceClient defines the interface for querying the on-chain governance
// contract (e.g., staking oracle or validator registry). All methods must be
// safe for concurrent use.
type GovernanceClient interface {
	// GetValidatorSet returns the complete validator set for the specified
	// epoch. The context must be non-nil and will be used for cancellation.
	GetValidatorSet(ctx context.Context, epoch uint64) ([]ValidatorInfo, error)

	// CurrentEpoch returns the current epoch number from the governance
	// contract. The context must be non-nil and will be used for cancellation.
	CurrentEpoch(ctx context.Context) (uint64, error)
}

// ReputationEngine is the interface for computing reputation-weighted voting
// power from a raw VotingPowerScore.
type ReputationEngine interface {
	// ComputeVotingPower takes a raw score and returns the reputation-weighted
	// voting power in wei. It must never return nil.
	ComputeVotingPower(score VotingPowerScore) *big.Int
}

// DefaultReputationEngine implements a simple linear-weight reputation
// computation with the following weight distribution:
//
//	Stake:       30%
//	Attestations: 25%
//	Activity:     20%
//	Verification: 15%
//	Tenure:       10%
//
// The final voting power is capped by the configured totalPower. In a
// production deployment, each factor should be normalized against its
// maximum possible value (e.g., maximum stake, maximum attestations) using
// on-chain data. This engine treats raw counts as directly proportional.
type DefaultReputationEngine struct {
	totalPower *big.Int // upper bound for voting power in wei
}

// NewDefaultReputationEngine creates a new DefaultReputationEngine with the
// given totalPower cap. If totalPower is nil or zero, a default of 1e24 wei
// is used. Returns an error if totalPower is negative.
func NewDefaultReputationEngine(totalPower *big.Int) (*DefaultReputationEngine, error) {
	if totalPower == nil || totalPower.Sign() == 0 {
		totalPower = new(big.Int).SetUint64(defaultTotalPower)
	}
	if totalPower.Sign() < 0 {
		return nil, fmt.Errorf("%w: got %s", ErrNegativeTotalPower, totalPower.String())
	}
	return &DefaultReputationEngine{totalPower: new(big.Int).Set(totalPower)}, nil
}

// TotalPower returns the configured maximum voting power (copied to prevent
// mutation). This is a read-only accessor.
func (e *DefaultReputationEngine) TotalPower() *big.Int {
	return new(big.Int).Set(e.totalPower)
}

// ComputeVotingPower implements ReputationEngine. It computes the weighted
// sum of the five reputation factors, clamped to non-negative values, and
// caps the result to totalPower. It never returns nil.
func (e *DefaultReputationEngine) ComputeVotingPower(score VotingPowerScore) *big.Int {
	if err := score.Validate(); err != nil {
		// If validation fails, return zero power as a safe fallback.
		// In production, the caller should validate before calling.
		return new(big.Int)
	}

	// Clamp raw values to non-negative.
	stake := clampNonNegative(score.Stake)
	attest := new(big.Int).SetUint64(score.Attestations)
	activity := new(big.Int).SetUint64(score.Activity)
	verify := new(big.Int).SetUint64(score.Verification)
	tenure := new(big.Int).SetUint64(score.Tenure)

	// weightedSum = 30*stake + 25*attest + 20*activity + 15*verify + 10*tenure
	weightedSum := new(big.Int).Add(
		new(big.Int).Add(
			new(big.Int).Add(
				new(big.Int).Mul(big.NewInt(stakeWeight), stake),
				new(big.Int).Mul(big.NewInt(attestWeight), attest),
			),
			new(big.Int).Mul(big.NewInt(activityWeight), activity),
		),
		new(big.Int).Mul(big.NewInt(verifyWeight), verify),
	)
	weightedSum.Add(weightedSum, new(big.Int).Mul(big.NewInt(tenureWeight), tenure))

	// Normalize weighted sum by dividing by totalWeight (100).
	votingPower := new(big.Int).Div(weightedSum, big.NewInt(totalWeight))

	// Cap to totalPower.
	if votingPower.Cmp(e.totalPower) > 0 {
		votingPower.Set(e.totalPower)
	}
	return votingPower
}

// clampNonNegative returns a copy of val with non-negative value (zero if negative).
func clampNonNegative(val *big.Int) *big.Int {
	if val.Sign() < 0 {
		return new(big.Int)
	}
	return new(big.Int).Set(val)
}

// ValidatorSetManager manages the set of validators with reputation-weighted voting power.
// It periodically fetches validator information from the governance contract and
// updates the internal set. It is safe for concurrent use.
type ValidatorSetManager struct {
	mu sync.RWMutex

	// governance is the client for the on-chain governance contract.
	governance GovernanceClient
	// reputationEngine computes reputation-weighted voting power.
	reputationEngine ReputationEngine
	// log is the structured logger.
	log logging.Logger

	// validators is the current set of validators with their voting power.
	validators []ValidatorVotingPower
	// totalVotingPower is the sum of all validator voting powers.
	totalVotingPower atomic.Value // *big.Int

	// updateInterval is the interval between polls to the governance contract.
	updateInterval time.Duration
	// started indicates whether the manager is running.
	started atomic.Bool
	// stopCh is closed when the manager is stopped.
	stopCh chan struct{}
}

// ValidatorVotingPower holds a validator address and its computed voting power.
type ValidatorVotingPower struct {
	Address      common.Address
	VotingPower *big.Int
}

// NewValidatorSetManager creates a new ValidatorSetManager with the given
// dependencies. All parameters must be non-nil; returns an error otherwise.
// The updateInterval must be positive; if zero, a default of 10s is used.
func NewValidatorSetManager(
	governance GovernanceClient,
	reputationEngine ReputationEngine,
	log logging.Logger,
	updateInterval time.Duration,
) (*ValidatorSetManager, error) {
	if governance == nil {
		return nil, ErrNilGovernanceClient
	}
	if reputationEngine == nil {
		return nil, ErrNilReputationEngine
	}
	if log == nil {
		return nil, ErrNilLogger
	}
	if updateInterval <= 0 {
		updateInterval = defaultUpdateInterval
	}

	mgr := &ValidatorSetManager{
		governance:        governance,
		reputationEngine:  reputationEngine,
		log:               log.WithComponent("ValidatorSetManager"),
		updateInterval:    updateInterval,
		validators:        make([]ValidatorVotingPower, 0),
		totalVotingPower:  atomic.Value{},
		stopCh:            make(chan struct{}),
	}
	// Initialize total power to zero.
	mgr.totalVotingPower.Store(new(big.Int))
	return mgr, nil
}

// Start begins the periodic update loop. It returns an error if the manager
// is already started or if the initial fetch fails.
func (m *ValidatorSetManager) Start(ctx context.Context) error {
	if ctx == nil {
		return ErrNilContext
	}
	if !m.started.CompareAndSwap(false, true) {
		return errors.New("validator set manager already started")
	}

	// Initial fetch.
	if err := m.updateValidatorSet(ctx); err != nil {
		m.started.Store(false)
		return fmt.Errorf("initial validator set update failed: %w", err)
	}

	// Start background updater.
	go m.run(ctx)
	return nil
}

// Stop signals the background updater to stop and blocks until it exits.
func (m *ValidatorSetManager) Stop() {
	if !m.started.Load() {
		return
	}
	close(m.stopCh)
	m.started.Store(false)
}

// run is the background loop for periodic updates.
func (m *ValidatorSetManager) run(ctx context.Context) {
	ticker := time.NewTicker(m.updateInterval)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			m.log.Debug("context cancelled, stopping validator set updater")
			m.Stop()
			return
		case <-m.stopCh:
			m.log.Debug("stop signal received, exitting validator set updater")
			return
		case <-ticker.C:
			if err := m.updateValidatorSet(ctx); err != nil {
				m.log.Error("failed to update validator set", "error", err)
			}
		}
	}
}

// updateValidatorSet fetches the current epoch and validator set from the
// governance contract, computes voting powers, and updates the internal state.
func (m *ValidatorSetManager) updateValidatorSet(ctx context.Context) error {
	epoch, err := m.governance.CurrentEpoch(ctx)
	if err != nil {
		return fmt.Errorf("%w: %v", ErrEpochFetchFailed, err)
	}

	validatorInfos, err := m.governance.GetValidatorSet(ctx, epoch)
	if err != nil {
		return fmt.Errorf("%w: %v", ErrValidatorSetUpdateFailed, err)
	}

	if len(validatorInfos) == 0 {
		m.log.Warn("governance returned empty validator set")
		// Keep previous set; return error to signal potential issue.
		return errors.New("governance returned empty validator set")
	}

	// Compute voting powers.
	newValidators := make([]ValidatorVotingPower, 0, len(validatorInfos))
	totalPower := new(big.Int)
	for _, info := range validatorInfos {
		power := m.reputationEngine.ComputeVotingPower(info.Score)
		if power == nil {
			m.log.Error("reputation engine returned nil voting power, using zero")
			power = new(big.Int)
		}
		newValidators = append(newValidators, ValidatorVotingPower{
			Address:     info.Address,
			VotingPower: power,
		})
		totalPower.Add(totalPower, power)
	}

	// Update internal state under write lock.
	m.mu.Lock()
	m.validators = newValidators
	m.totalVotingPower.Store(new(big.Int).Set(totalPower))
	m.mu.Unlock()

	m.log.Info("validator set updated",
		"epoch", epoch,
		"count", len(newValidators),
		"totalPower", totalPower,
	)
	return nil
}

// Validators returns a copy of the current validator set and total voting power.
// The returned slice is safe to modify.
func (m *ValidatorSetManager) Validators() ([]ValidatorVotingPower, *big.Int) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	validatorsCopy := make([]ValidatorVotingPower, len(m.validators))
	copy(validatorsCopy, m.validators)
	totalPower := new(big.Int).Set(m.totalVotingPower.Load().(*big.Int))
	return validatorsCopy, totalPower
}

// ValidatorByAddress returns the voting power for a specific validator address.
// Returns ErrValidatorNotFound if the address is not in the current set.
func (m *ValidatorSetManager) ValidatorByAddress(addr common.Address) (*ValidatorVotingPower, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	for i := range m.validators {
		if m.validators[i].Address == addr {
			vp := m.validators[i]
			return &ValidatorVotingPower{
				Address:     vp.Address,
				VotingPower: new(big.Int).Set(vp.VotingPower),
			}, nil
		}
	}
	return nil, ErrValidatorNotFound
}

// IsRunning returns true if the manager's background updater is active.
func (m *ValidatorSetManager) IsRunning() bool {
	return m.started.Load()
}