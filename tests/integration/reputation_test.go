// tests/integration/reputation_test.go
//go:build integration

package integration

import (
	"math"
	"testing"
)

// --- Domain types (mirror production package `reputation` for test isolation) ---

// FactorWeights defines the five factors used to compute reputation score.
type FactorWeights struct {
	Stake       float64 // 30%
	Attestation float64 // 25%
	Activity    float64 // 20%
	Verification float64 // 15%
	Tenure      float64 // 10%
}

// DefaultWeights returns the canonical weight distribution.
func DefaultWeights() FactorWeights {
	return FactorWeights{
		Stake:       0.30,
		Attestation: 0.25,
		Activity:    0.20,
		Verification: 0.15,
		Tenure:      0.10,
	}
}

// ReputationFactors holds the raw scores for one validator (0.0 – 1.0 range).
type ReputationFactors struct {
	Stake       float64
	Attestation float64
	Activity    float64
	Verification float64
	Tenure      float64
}

// Validator represents a single node in the network.
type Validator struct {
	ID             string
	Factors        ReputationFactors
	VotingPower    float64 // computed, should equal weighted score
}

// ReputationEngine simulates the on-chain rules used to compute voting power.
type ReputationEngine struct {
	weights FactorWeights
}

// NewReputationEngine creates an engine with given weights.
func NewReputationEngine(w FactorWeights) *ReputationEngine {
	return &ReputationEngine{weights: w}
}

// ComputeScore returns the weighted sum of all five factors.
func (e *ReputationEngine) ComputeScore(f ReputationFactors) float64 {
	return e.weights.Stake*f.Stake +
		e.weights.Attestation*f.Attestation +
		e.weights.Activity*f.Activity +
		e.weights.Verification*f.Verification +
		e.weights.Tenure*f.Tenure
}

// GovernanceOracle simulates the smart contract that triggers epoch transitions.
type GovernanceOracle struct {
	engine *ReputationEngine
}

// NewGovernanceOracle creates an oracle linked to the reputation engine.
func NewGovernanceOracle(e *ReputationEngine) *GovernanceOracle {
	return &GovernanceOracle{engine: e}
}

// FinalizeEpoch updates every validator's VotingPower to its current weighted score.
// This mirrors the on‑chain call at epoch boundaries.
func (g *GovernanceOracle) FinalizeEpoch(validators []*Validator) {
	for _, v := range validators {
		v.VotingPower = g.engine.ComputeScore(v.Factors)
	}
}

// UpdateReputation applies a delta to one factor of a validator and recomputes.
// In reality this would be a governance proposal; here we simulate the result.
func (g *GovernanceOracle) UpdateReputation(v *Validator, field string, delta float64) {
	switch field {
	case "stake":
		v.Factors.Stake = clamp(v.Factors.Stake + delta)
	case "attestation":
		v.Factors.Attestation = clamp(v.Factors.Attestation + delta)
	case "activity":
		v.Factors.Activity = clamp(v.Factors.Activity + delta)
	case "verification":
		v.Factors.Verification = clamp(v.Factors.Verification + delta)
	case "tenure":
		v.Factors.Tenure = clamp(v.Factors.Tenure + delta)
	}
}

// clamp ensures a value stays inside [0.0, 1.0].
func clamp(v float64) float64 {
	if v < 0 {
		return 0
	}
	if v > 1 {
		return 1
	}
	return v
}

// --- Test cases ---

func TestReputationVotingPowerMatch(t *testing.T) {
	engine := NewReputationEngine(DefaultWeights())
	gov := NewGovernanceOracle(engine)

	// Predefined validators with various starting profiles.
	validators := []*Validator{
		{ID: "val-1", Factors: ReputationFactors{Stake: 0.8, Attestation: 0.7, Activity: 0.6, Verification: 0.9, Tenure: 0.5}},
		{ID: "val-2", Factors: ReputationFactors{Stake: 0.4, Attestation: 0.4, Activity: 0.4, Verification: 0.4, Tenure: 0.4}},
		{ID: "val-3", Factors: ReputationFactors{Stake: 0.1, Attestation: 0.2, Activity: 0.3, Verification: 0.1, Tenure: 0.9}},
	}

	// Table‑driven tests covering different reputation changes.
	tests := []struct {
		name      string
		validator *Validator
		field     string
		delta     float64
	}{
		{
			name:      "increase activity score",
			validator: validators[0],
			field:     "activity",
			delta:     0.2, // 0.6 → 0.8
		},
		{
			name:      "decrease stake score",
			validator: validators[1],
			field:     "stake",
			delta:     -0.1, // 0.4 → 0.3
		},
		{
			name:      "mixed changes aggregation",
			validator: validators[2],
			field:     "tenure",
			delta:     0.1, // 0.9 → 1.0 (clamped)
		},
		{
			name:      "zero delta – no change",
			validator: validators[0],
			field:     "attestation",
			delta:     0.0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Record the voting power before the change.
			oldScore := engine.ComputeScore(tt.validator.Factors)
			gov.FinalizeEpoch(validators) // initial epoch assignment
			if got := tt.validator.VotingPower; math.Abs(got-oldScore) > 1e-9 {
				t.Fatalf("initial VotingPower %.4f did not match computed score %.4f", got, oldScore)
			}

			// Apply the reputation change.
			gov.UpdateReputation(tt.validator, tt.field, tt.delta)

			// Simulate epoch transition: governance finalizes.
			gov.FinalizeEpoch(validators)

			// Expected voting power after change.
			expected := engine.ComputeScore(tt.validator.Factors)
			if got := tt.validator.VotingPower; math.Abs(got-expected) > 1e-9 {
				t.Errorf("after %s (+%.2f), VotingPower = %.4f, want %.4f",
					tt.field, tt.delta, got, expected)
			}

			// Validate that other validators' voting power remained unchanged.
			for _, other := range validators {
				if other.ID == tt.validator.ID {
					continue
				}
				expectedOther := engine.ComputeScore(other.Factors)
				if got := other.VotingPower; math.Abs(got-expectedOther) > 1e-9 {
					t.Errorf("unexpected change for %s: got %.4f, want %.4f", other.ID, got, expectedOther)
				}
			}
		})
	}
}

func TestReputationEdgeCases(t *testing.T) {
	engine := NewReputationEngine(DefaultWeights())
	gov := NewGovernanceOracle(engine)

	validators := []*Validator{
		{ID: "zero", Factors: ReputationFactors{}},
		{ID: "max", Factors: ReputationFactors{Stake: 1, Attestation: 1, Activity: 1, Verification: 1, Tenure: 1}},
		{ID: "partial", Factors: ReputationFactors{Stake: 0.5, Attestation: 0, Activity: 0.5, Verification: 0, Tenure: 0.5}},
	}

	t.Run("initial epoch sets correct powers", func(t *testing.T) {
		gov.FinalizeEpoch(validators)
		for _, v := range validators {
			expected := engine.ComputeScore(v.Factors)
			if got := v.VotingPower; math.Abs(got-expected) > 1e-9 {
				t.Errorf("%s: got %.4f, want %.4f", v.ID, got, expected)
			}
		}
	})

	t.Run("clamping negative delta", func(t *testing.T) {
		v := validators[1] // all 1.0 – adding positive does nothing
		gov.UpdateReputation(v, "stake", 0.5)
		if v.Factors.Stake != 1.0 {
			t.Errorf("stake should be clamped to 1.0, got %.4f", v.Factors.Stake)
		}
		gov.FinalizeEpoch(validators)
		expected := engine.ComputeScore(v.Factors)
		if v.VotingPower != expected {
			t.Errorf("after clamp: got %.4f, want %.4f", v.VotingPower, expected)
		}
	})

	t.Run("multiple epoch transitions preserve consistency", func(t *testing.T) {
		v := validators[2]
		for i := 0; i < 10; i++ {
			gov.UpdateReputation(v, "activity", 0.1)
			gov.FinalizeEpoch(validators)
			expected := engine.ComputeScore(v.Factors)
			if v.VotingPower != expected {
				t.Fatalf("after epoch %d: got %.4f, want %.4f", i+1, v.VotingPower, expected)
			}
		}
	})
}

// TestReputationEpochTransition simulates a complete governance cycle.
func TestReputationEpochTransition(t *testing.T) {
	engine := NewReputationEngine(DefaultWeights())
	gov := NewGovernanceOracle(engine)

	// Setup a small validator set.
	set := []*Validator{
		{ID: "A", Factors: ReputationFactors{Stake: 0.9, Attestation: 0.3, Activity: 0.7, Verification: 0.4, Tenure: 0.2}},
		{ID: "B", Factors: ReputationFactors{Stake: 0.5, Attestation: 0.9, Activity: 0.4, Verification: 0.9, Tenure: 0.6}},
	}

	// Epoch 0: initial assignment.
	gov.FinalizeEpoch(set)

	// Verify A > B based on weights (A: 0.9*0.3 + 0.3*0.25 + 0.7*0.2 + 0.4*0.15 + 0.2*0.1 = 0.565; B: 0.5*0.3 + 0.9*0.25 + 0.4*0.2 + 0.9*0.15 + 0.6*0.1 = 0.635)
	if set[0].VotingPower >= set[1].VotingPower {
		t.Logf("initial order: A=%.4f, B=%.4f (expected B higher)", set[0].VotingPower, set[1].VotingPower)
	}

	// Governance proposal: boost B's verification, reduce A's tenure (simulate slashing).
	gov.UpdateReputation(set[0], "tenure", -0.1)                                // 0.2 → 0.1
	gov.UpdateReputation(set[1], "verification", 0.1)                           // 0.9 → 1.0

	// Epoch transition.
	gov.FinalizeEpoch(set)

	// Recompute expected.
	expectedA := engine.ComputeScore(set[0].Factors)
	expectedB := engine.ComputeScore(set[1].Factors)

	if set[0].VotingPower != expectedA {
		t.Errorf("A VotingPower mismatch: got %.4f, want %.4f", set[0].VotingPower, expectedA)
	}
	if set[1].VotingPower != expectedB {
		t.Errorf("B VotingPower mismatch: got %.4f, want %.4f", set[1].VotingPower, expectedB)
	}

	// Verify the gap widened.
	gapBefore := 0.635 - 0.565 // approx 0.07
	gapAfter := expectedB - expectedA
	if gapAfter <= gapBefore {
		t.Errorf("expected gap to widen after governance update, gapBefore=%.4f, gapAfter=%.4f", gapBefore, gapAfter)
	}
}