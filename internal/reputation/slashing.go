// Package reputation provides slashing logic for the Neunode L1 blockchain.
// It processes Malachite consensus evidence (double-sign and equivocation),
// penalizes validator stake and reputation score, and triggers governance actions.
package reputation

import (
	"fmt"
	"math/big"
	"sync"
	"time"
)

// ---------- Slashing Constants ----------

// Default slashing parameters (can be overridden by governance).
const (
	// DefaultDoubleSignSlashPercent is the percentage of stake slashed for double-sign.
	DefaultDoubleSignSlashPercent = 5 // 5%
	// DefaultEquivocationSlashPercent is the percentage of stake slashed for equivocation.
	DefaultEquivocationSlashPercent = 3 // 3%
	// DefaultReputationPenalty is the fixed reputation points deducted for any slashing offence.
	DefaultReputationPenalty = 100
	// DefaultJailDuration is the duration a validator is jailed after slashing.
	DefaultJailDuration = 7 * 24 * time.Hour // 7 days
)

// ---------- Evidence Types ----------

// EvidenceType enumerates the kinds of misbehaviour.
type EvidenceType int

const (
	EvidenceTypeDoubleSign    EvidenceType = iota // validator signs two different blocks at same height/round
	EvidenceTypeEquivocation                      // validator signs conflicting messages (e.g., votes)
)

// String returns a human-readable name.
func (e EvidenceType) String() string {
	switch e {
	case EvidenceTypeDoubleSign:
		return "double_sign"
	case EvidenceTypeEquivocation:
		return "equivocation"
	default:
		return "unknown"
	}
}

// SlashingEvidence contains the raw evidence from Malachite consensus layer.
type SlashingEvidence struct {
	Type         EvidenceType
	ValidatorID  string // unique identifier (e.g., hex-encoded public key)
	Height       uint64
	Round        int32
	SignData1    []byte // first signed message
	SignData2    []byte // conflicting signed message
	Timestamp    time.Time
	Metadata     map[string]string // optional extra info
}

// Validate performs basic checks on the evidence.
func (e *SlashingEvidence) Validate() error {
	if e.ValidatorID == "" {
		return fmt.Errorf("slashing evidence: validator ID is empty")
	}
	if len(e.SignData1) == 0 || len(e.SignData2) == 0 {
		return fmt.Errorf("slashing evidence: both sign data fields must be non-empty")
	}
	if e.Timestamp.IsZero() {
		return fmt.Errorf("slashing evidence: timestamp is zero")
	}
	return nil
}

// ---------- Penalty & Result Types ----------

// SlashingPenalty holds the computed penalties for a validator.
type SlashingPenalty struct {
	ValidatorID        string
	StakeSlashAmount   *big.Int // amount of stake to burn/slash
	ReputationPenalty  int64    // reputation points to deduct
	JailDuration       time.Duration
	IsTombstoned       bool // if true, validator is permanently removed
}

// SlashingResult is the outcome of processing an evidence.
type SlashingResult struct {
	Evidence        SlashingEvidence
	Penalty         SlashingPenalty
	GovernanceTxID  string // transaction hash of governance action (if executed)
	SlashedAt       time.Time
	Error           error // non-nil if processing failed
}

// ---------- Interfaces ----------

// StakeKeeper abstracts the staking module to query and modify validator stake.
type StakeKeeper interface {
	// GetStake returns the total stake of a validator (in wei).
	GetStake(validatorID string) (*big.Int, error)
	// SlashStake deducts the given amount from the validator's stake.
	// Returns new total stake after slashing.
	SlashStake(validatorID string, amount *big.Int) (*big.Int, error)
}

// ReputationKeeper abstracts the reputation module to adjust validator scores.
type ReputationKeeper interface {
	// DeductReputation reduces the reputation score of a validator.
	DeductReputation(validatorID string, points int64) error
	// GetReputation returns the current reputation score.
	GetReputation(validatorID string) (int64, error)
}

// GovernanceAction defines a function to submit a slashing proposal on-chain.
type GovernanceAction func(evidence SlashingEvidence, penalty SlashingPenalty) (txID string, err error)

// SlashingManager is the central interface for processing slashing evidence.
type SlashingManager interface {
	// ProcessEvidence handles an incoming SlashingEvidence, verifies it,
	// computes penalties, applies them, and triggers governance.
	// Returns a SlashingResult with full details.
	ProcessEvidence(evidence SlashingEvidence) *SlashingResult
	// GetParameters returns current slashing parameters.
	GetParameters() SlashingParameters
	// UpdateParameters updates slashing parameters (governance only).
	UpdateParameters(params SlashingParameters) error
}

// SlashingParameters holds configurable slashing values.
type SlashingParameters struct {
	DoubleSignSlashPercent   int
	EquivocationSlashPercent int
	ReputationPenalty        int64
	JailDuration             time.Duration
}

// DefaultSlashingParameters returns sensible defaults.
func DefaultSlashingParameters() SlashingParameters {
	return SlashingParameters{
		DoubleSignSlashPercent:   DefaultDoubleSignSlashPercent,
		EquivocationSlashPercent: DefaultEquivocationSlashPercent,
		ReputationPenalty:        DefaultReputationPenalty,
		JailDuration:             DefaultJailDuration,
	}
}

// ---------- Implementation ----------

type slashingManagerImpl struct {
	mu sync.RWMutex

	stakeKeeper      StakeKeeper
	repKeeper        ReputationKeeper
	governanceAction GovernanceAction
	params           SlashingParameters
	processed        map[string]time.Time // tracks evidence by validator+height to prevent double processing
}

// NewSlashingManager creates a new SlashingManager with required dependencies.
func NewSlashingManager(
	stake StakeKeeper,
	rep ReputationKeeper,
	gov GovernanceAction,
) SlashingManager {
	return &slashingManagerImpl{
		stakeKeeper:      stake,
		repKeeper:        rep,
		governanceAction: gov,
		params:           DefaultSlashingParameters(),
		processed:        make(map[string]time.Time),
	}
}

// ProcessEvidence implements SlashingManager.
func (sm *slashingManagerImpl) ProcessEvidence(evidence SlashingEvidence) *SlashingResult {
	result := &SlashingResult{
		Evidence:  evidence,
		SlashedAt: time.Now(),
	}

	// 1. Basic validation
	if err := evidence.Validate(); err != nil {
		result.Error = fmt.Errorf("evidence validation failed: %w", err)
		return result
	}

	// 2. Deduplication: prevent same evidence being processed twice
	sm.mu.Lock()
	key := evidence.ValidatorID + "@" + fmt.Sprintf("%d/%d", evidence.Height, evidence.Round)
	if _, exists := sm.processed[key]; exists {
		sm.mu.Unlock()
		result.Error = fmt.Errorf("evidence already processed for validator %s at height %d", evidence.ValidatorID, evidence.Height)
		return result
	}
	sm.processed[key] = result.SlashedAt
	sm.mu.Unlock()

	// 3. Verify cryptographic signatures (placeholder – real implementation uses BLS/ed25519 verification)
	if err := verifySignatures(evidence); err != nil {
		result.Error = fmt.Errorf("signature verification failed: %w", err)
		return result
	}

	// 4. Compute penalty
	penalty, err := sm.computePenalty(evidence)
	if err != nil {
		result.Error = fmt.Errorf("penalty computation error: %w", err)
		return result
	}
	result.Penalty = *penalty

	// 5. Apply stake slash
	if err := sm.applyStakeSlash(evidence.ValidatorID, penalty.StakeSlashAmount); err != nil {
		result.Error = fmt.Errorf("stake slashing failed: %w", err)
		return result
	}

	// 6. Apply reputation penalty
	if err := sm.applyReputationPenalty(evidence.ValidatorID, penalty.ReputationPenalty); err != nil {
		result.Error = fmt.Errorf("reputation penalty failed: %w", err)
		return result
	}

	// 7. Trigger governance action (async is acceptable, but we record TX ID)
	if sm.governanceAction != nil {
		txID, err := sm.governanceAction(evidence, *penalty)
		if err != nil {
			// Governance failure is logged but does not revert slashing.
			// In production, you might want to rollback or retry.
			result.Error = fmt.Errorf("governance action failed (slashing still applied): %w", err)
		} else {
			result.GovernanceTxID = txID
		}
	}

	return result
}

// GetParameters returns current parameters.
func (sm *slashingManagerImpl) GetParameters() SlashingParameters {
	sm.mu.RLock()
	defer sm.mu.RUnlock()
	return sm.params
}

// UpdateParameters updates parameters (governance only).
func (sm *slashingManagerImpl) UpdateParameters(params SlashingParameters) error {
	// In production, add authorization check (e.g., caller must be governance contract).
	sm.mu.Lock()
	defer sm.mu.Unlock()
	sm.params = params
	return nil
}

// computePenalty calculates the slashing amount and reputation deduction.
func (sm *slashingManagerImpl) computePenalty(evidence SlashingEvidence) (*SlashingPenalty, error) {
	params := sm.GetParameters()

	// Determine slash percentage
	var slashPercent int
	switch evidence.Type {
	case EvidenceTypeDoubleSign:
		slashPercent = params.DoubleSignSlashPercent
	case EvidenceTypeEquivocation:
		slashPercent = params.EquivocationSlashPercent
	default:
		return nil, fmt.Errorf("unknown evidence type %v", evidence.Type)
	}

	// Get current stake
	totalStake, err := sm.stakeKeeper.GetStake(evidence.ValidatorID)
	if err != nil {
		return nil, fmt.Errorf("cannot retrieve stake for %s: %w", evidence.ValidatorID, err)
	}

	// Compute slash amount (percentage of total stake)
	slashAmount := new(big.Int).Mul(totalStake, big.NewInt(int64(slashPercent)))
	slashAmount.Div(slashAmount, big.NewInt(100)) // integer division

	// Reputation deduction
	repPenalty := params.ReputationPenalty

	// Jail duration
	jail := params.JailDuration

	// For double-sign, we tombstone (permanent removal) – policy choice
	tombstoned := evidence.Type == EvidenceTypeDoubleSign

	penalty := &SlashingPenalty{
		ValidatorID:       evidence.ValidatorID,
		StakeSlashAmount:  slashAmount,
		ReputationPenalty: repPenalty,
		JailDuration:      jail,
		IsTombstoned:      tombstoned,
	}
	return penalty, nil
}

// applyStakeSlash performs the actual deduction via StakeKeeper.
func (sm *slashingManagerImpl) applyStakeSlash(validatorID string, amount *big.Int) error {
	if amount == nil || amount.Sign() == 0 {
		return nil // nothing to slash
	}
	_, err := sm.stakeKeeper.SlashStake(validatorID, amount)
	return err
}

// applyReputationPenalty deducts reputation points.
func (sm *slashingManagerImpl) applyReputationPenalty(validatorID string, points int64) error {
	if points <= 0 {
		return nil
	}
	return sm.repKeeper.DeductReputation(validatorID, points)
}

// verifySignatures is a placeholder for cryptographic verification.
// Real implementation should verify the two signed messages against the validator's public key.
func verifySignatures(evidence SlashingEvidence) error {
	// In production, decode public key from evidence.ValidatorID,
	// verify SignData1 and SignData2 are valid signatures on their respective messages.
	// For now, assume valid (no-op).
	return nil
}

// Ensure interface compliance.
var _ SlashingManager = (*slashingManagerImpl)(nil)

// ---------- Example Usage (commented out) ----------
// func main() {
// 	stakeKeeper := NewMockStakeKeeper()
// 	repKeeper := NewMockReputationKeeper()
// 	govAction := func(ev reputation.SlashingEvidence, p reputation.SlashingPenalty) (string, error) {
// 		fmt.Printf("Governance: slashing %s for %v, amount=%s, reputation=%d\n", ev.ValidatorID, ev.Type, p.StakeSlashAmount, p.ReputationPenalty)
// 		return "0xabc123", nil
// 	}
// 	sm := reputation.NewSlashingManager(stakeKeeper, repKeeper, govAction)
//
// 	evidence := reputation.SlashingEvidence{
// 		Type:        reputation.EvidenceTypeDoubleSign,
// 		ValidatorID: "0xdeadbeef",
// 		Height:      100,
// 		Round:       0,
// 		SignData1:   []byte("block1"),
// 		SignData2:   []byte("block2"),
// 		Timestamp:   time.Now(),
// 	}
// 	result := sm.ProcessEvidence(evidence)
// 	if result.Error != nil {
// 		log.Fatalf("slashing failed: %v", result.Error)
// 	}
// 	fmt.Printf("Result: %+v\n", result)
// }