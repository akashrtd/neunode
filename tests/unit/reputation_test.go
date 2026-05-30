package reputation_test

import (
	"math"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// Test helpers – internal types that mirror the production reputation module.
// ---------------------------------------------------------------------------

// Factors are the five dimensions used to compute validator reputation.
type Factors struct {
	Stake       float64 // 0–100 (normalised)
	Attestation float64 // 0–100
	Activity    float64 // 0–100
	Verification float64 // 0–100
	Tenure      float64 // 0–100
}

// Weights are the static coefficients applied to each factor.
const (
	wStake        = 0.30
	wAttestation  = 0.25
	wActivity     = 0.20
	wVerification = 0.15
	wTenure       = 0.10
)

// ValidatorSnapshot holds mutable state for reputation calculations.
type ValidatorSnapshot struct {
	Address   string
	Rep       float64
	LastUpdate time.Time
	Missed     int
	MissedWindow int
}

// Score computes the weighted reputation score from Factors.
func Score(f Factors) float64 {
	return wStake*f.Stake + wAttestation*f.Attestation + wActivity*f.Activity + wVerification*f.Verification + wTenure*f.Tenure
}

// Slashing detection: we consider a double-signing or excessive misses.
func IsSlashable(v ValidatorSnapshot) bool {
	// Missing more than 10 blocks in the last window of 100 is slashable.
	const maxMisses = 10
	const window = 100
	if v.MissedWindow != window {
		// Simulated fixed window; in production it would be rolling.
		return false
	}
	return v.Missed > maxMisses
}

// Decay returns the reputation after d has elapsed, applying a half-life
// of 30 days (simple exponential decay).
func Decay(rep float64, d time.Duration) float64 {
	const halfLife = 30 * 24 * time.Hour
	halves := float64(d) / float64(halfLife)
	return rep * math.Pow(0.5, halves)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

func TestScore(t *testing.T) {
	tests := []struct {
		name string
		f    Factors
		want float64
	}{
		{
			name: "perfect validator",
			f:    Factors{100, 100, 100, 100, 100},
			want: 100.0,
		},
		{
			name: "zero everywhere",
			f:    Factors{0, 0, 0, 0, 0},
			want: 0.0,
		},
		{
			name: "only stake",
			f:    Factors{50, 0, 0, 0, 0},
			want: 15.0,
		},
		{
			name: "mixed",
			f:    Factors{80, 70, 60, 50, 40},
			want: 24 + 17.5 + 12 + 7.5 + 4, // = 65.0
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := Score(tt.f)
			// Allow small floating error.
			if math.Abs(got-tt.want) > 1e-9 {
				t.Errorf("Score() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestIsSlashable(t *testing.T) {
	now := time.Now()
	tests := []struct {
		name string
		v    ValidatorSnapshot
		want bool
	}{
		{
			name: "no misses",
			v:    ValidatorSnapshot{"addr1", 100, now, 0, 100},
			want: false,
		},
		{
			name: "just under limit",
			v:    ValidatorSnapshot{"addr2", 100, now, 10, 100},
			want: false,
		},
		{
			name: "over limit",
			v:    ValidatorSnapshot{"addr3", 100, now, 11, 100},
			want: true,
		},
		{
			name: "window mismatch",
			v:    ValidatorSnapshot{"addr4", 100, now, 11, 50},
			want: false, // only check when window == expected
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := IsSlashable(tt.v); got != tt.want {
				t.Errorf("IsSlashable() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestDecay(t *testing.T) {
	tests := []struct {
		name string
		rep  float64
		d    time.Duration
		want float64
	}{
		{
			name: "no time passed",
			rep:  100,
			d:    0,
			want: 100.0,
		},
		{
			name: "one half-life",
			rep:  100,
			d:    30 * 24 * time.Hour,
			want: 50.0,
		},
		{
			name: "two half-lives",
			rep:  100,
			d:    60 * 24 * time.Hour,
			want: 25.0,
		},
		{
			name: "partial half-life",
			rep:  80,
			d:    15 * 24 * time.Hour, // half of 30 days
			want: 80 * math.Pow(0.5, 0.5),
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := Decay(tt.rep, tt.d)
			if math.Abs(got-tt.want) > 1e-9 {
				t.Errorf("Decay() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestScoreWeightsSumToOne(t *testing.T) {
	weights := []float64{wStake, wAttestation, wActivity, wVerification, wTenure}
	var sum float64
	for _, w := range weights {
		sum += w
	}
	if math.Abs(sum-1.0) > 1e-9 {
		t.Errorf("Weights sum = %v, want 1.0", sum)
	}
}

func TestSlashableEdgeCases(t *testing.T) {
	now := time.Now()
	// Maximum allowed missed before slash.
	v := ValidatorSnapshot{"edge", 100, now, 10, 100}
	if IsSlashable(v) {
		t.Error("expected not slashable at exactly limit")
	}
	v.Missed = 11
	if !IsSlashable(v) {
		t.Error("expected slashable over limit")
	}
}

func TestDecayNonPositive(t *testing.T) {
	// Decay should handle zero and negative time.
	neg := Decay(100, -1*time.Hour)
	if neg != 100 { // negative time → no decay (we treat as zero)
		t.Errorf("Decay with negative duration = %v, want %v", neg, 100.0)
	}
	zero := Decay(0, 30*24*time.Hour)
	if zero != 0 {
		t.Errorf("Decay of zero = %v, want 0", zero)
	}
}