package reputation

import (
	"fmt"
	"math"
	"time"
)

// Weights for the five reputation factors.
const (
	WeightStake    = 0.30
	WeightAttest   = 0.25
	WeightActivity = 0.20
	WeightVerify   = 0.15
	WeightTenure   = 0.10
)

// Factors holds the raw data used to compute a validator's reputation.
type Factors struct {
	Stake        uint64        // amount of stake (smallest unit of neu token)
	Attestations uint64        // cumulative valid attestations
	Activities   uint64        // blocks proposed / validated successfully
	Verifications uint64       // successful verification events
	Tenure       time.Duration // time since first registration
}

// MaxFactors defines the maximum (ideal) values for each factor.
// Used to normalise each factor to a 0‑1 range.
type MaxFactors struct {
	Stake        uint64
	Attestations uint64
	Activities   uint64
	Verifications uint64
	Tenure       time.Duration
}

// Score contains the normalised sub‑scores and the final total score.
type Score struct {
	StakeScore    float64
	AttestScore   float64
	ActivityScore float64
	VerifyScore   float64
	TenureScore   float64
	Total         float64
}

// ComputeScore calculates the weighted reputation score from raw factors and
// their maximum expected values. Each factor is normalised to [0,1] before
// applying the weights.
// Returns an error if any maximum value is zero or negative.
func ComputeScore(factors Factors, maxFactors MaxFactors) (Score, error) {
	if err := validateMaxFactors(maxFactors); err != nil {
		return Score{}, fmt.Errorf("reputation: invalid max factors: %w", err)
	}

	s := Score{
		StakeScore:    normalise(factors.Stake, maxFactors.Stake),
		AttestScore:   normalise(factors.Attestations, maxFactors.Attestations),
		ActivityScore: normalise(factors.Activities, maxFactors.Activities),
		VerifyScore:   normalise(factors.Verifications, maxFactors.Verifications),
		TenureScore:   normaliseDuration(factors.Tenure, maxFactors.Tenure),
	}

	s.Total = WeightStake*s.StakeScore +
		WeightAttest*s.AttestScore +
		WeightActivity*s.ActivityScore +
		WeightVerify*s.VerifyScore +
		WeightTenure*s.TenureScore

	return s, nil
}

// VotingPower calculates voting power as reputation score * stake.
// The result is truncated to the nearest integer (rounds half away from zero).
// A score of 1.0 gives full voting power equal to the stake amount.
func VotingPower(score float64, stake uint64) uint64 {
	return uint64(math.Round(score * float64(stake)))
}

// normalise returns value / max, clamped to [0,1]. Both are uint64.
func normalise(value, max uint64) float64 {
	if max == 0 {
		return 0
	}
	v := float64(value) / float64(max)
	if v < 0 {
		return 0
	}
	if v > 1 {
		return 1
	}
	return v
}

// normaliseDuration returns d / max, clamped to [0,1].
func normaliseDuration(d, max time.Duration) float64 {
	if max <= 0 {
		return 0
	}
	v := float64(d) / float64(max)
	if v < 0 {
		return 0
	}
	if v > 1 {
		return 1
	}
	return v
}

// validateMaxFactors checks that all maximum values are positive.
func validateMaxFactors(m MaxFactors) error {
	if m.Stake == 0 {
		return fmt.Errorf("max stake must be > 0")
	}
	if m.Attestations == 0 {
		return fmt.Errorf("max attestations must be > 0")
	}
	if m.Activities == 0 {
		return fmt.Errorf("max activities must be > 0")
	}
	if m.Verifications == 0 {
		return fmt.Errorf("max verifications must be > 0")
	}
	if m.Tenure <= 0 {
		return fmt.Errorf("max tenure must be > 0")
	}
	return nil
}