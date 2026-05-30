package engine_api

import (
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"math"
	"sync"
	"time"
)

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const (
	// MaxExtraDataSize is the maximum allowed size for ExtraData (100 KiB).
	MaxExtraDataSize = 100 * 1024

	// MaxTransactionsCount is a safe upper limit for the number of transactions
	// in a single payload (5 million).
	MaxTransactionsCount = 5_000_000

	// MaxWithdrawalValue is the maximum realistic amount for a withdrawal (1<<62).
	MaxWithdrawalValue = 1 << 62

	// MaxBlockGasLimit is the maximum gas limit (2^63-1) per Ethereum spec.
	MaxBlockGasLimit = 1<<63 - 1

	// MinBlockGasLimit is the minimum gas limit (21000 gas for a single transfer).
	MinBlockGasLimit = 21000

	// LogsBloomLength is the required length of the logs bloom filter (2048 bytes).
	LogsBloomLength = 2048

	// BaseFeePerGasLength is the required length of the base fee field (32 bytes).
	BaseFeePerGasLength = 32

	// MaxFutureTimestampOffset defines the maximum allowed offset for timestamps
	// in the future (10 seconds).
	MaxFutureTimestampOffset = 10 * time.Second
)

// ---------------------------------------------------------------------------
// Sentinel Errors
// ---------------------------------------------------------------------------

var (
	ErrNilInput        = errors.New("input is nil")
	ErrZeroHash        = errors.New("hash must not be zero")
	ErrZeroAddress     = errors.New("address must not be zero")
	ErrZeroTimestamp   = errors.New("timestamp must not be zero")
	ErrInvalidLength   = errors.New("invalid length")
	ErrOutOfBounds     = errors.New("value out of bounds")
	ErrExceedsMaxCount = errors.New("count exceeds maximum")
	ErrEmptyTransaction = errors.New("empty transaction not allowed")
	ErrGasLimitBelowMin = errors.New("gas limit below minimum")
	ErrGasLimitAboveMax = errors.New("gas limit above maximum")
	ErrGasUsedExceedsLimit = errors.New("gas used exceeds gas limit")
	ErrTimestampTooFar = errors.New("timestamp too far in the future")
)

// ValidationError provides structured validation failure details.
type ValidationError struct {
	Field string
	Err   error
	Value interface{}
}

func (e *ValidationError) Error() string {
	return fmt.Sprintf("validation failed on field '%s': %v (value=%v)", e.Field, e.Err, e.Value)
}

func (e *ValidationError) Unwrap() error {
	return e.Err
}

// NewValidationError creates a new ValidationError.
func NewValidationError(field string, err error, value interface{}) *ValidationError {
	return &ValidationError{Field: field, Err: err, Value: value}
}

// ---------------------------------------------------------------------------
// Type Aliases
// ---------------------------------------------------------------------------

// PayloadID is an 8‑byte identifier used to reference a payload under construction.
type PayloadID [8]byte

func (p PayloadID) String() string {
	return hex.EncodeToString(p[:])
}

// Bytes returns a copy of the payload ID slice.
func (p PayloadID) Bytes() []byte {
	cp := make([]byte, len(p))
	copy(cp, p[:])
	return cp
}

// Validate checks that the PayloadID is non‑zero.
func (p PayloadID) Validate() error {
	if p == (PayloadID{}) {
		return errors.New("payload ID must not be zero") // no field prefix needed
	}
	return nil
}

// Status is an enum describing the outcome of payload processing.
type Status string

const (
	StatusValid    Status = "VALID"
	StatusInvalid  Status = "INVALID"
	StatusAccepted Status = "ACCEPTED"
	StatusSyncing  Status = "SYNCING"
	StatusUnknown  Status = "UNKNOWN"
)

// Hash is a 32‑byte hash (e.g., block hash, state root).
type Hash [32]byte

func (h Hash) String() string {
	return hex.EncodeToString(h[:])
}

// Bytes returns a copy of the hash slice.
func (h Hash) Bytes() []byte {
	cp := make([]byte, len(h))
	copy(cp, h[:])
	return cp
}

// Validate checks that the hash is non‑zero.
func (h Hash) Validate() error {
	if h == (Hash{}) {
		return ErrZeroHash
	}
	return nil
}

// Address is a 20‑byte Ethereum address.
type Address [20]byte

func (a Address) String() string {
	return hex.EncodeToString(a[:])
}

// Validate checks that the address is non‑zero.
func (a Address) Validate() error {
	if a == (Address{}) {
		return ErrZeroAddress
	}
	return nil
}

// ---------------------------------------------------------------------------
// WithdrawalV1
// ---------------------------------------------------------------------------

// WithdrawalV1 represents an EIP‑4895 withdrawal.
type WithdrawalV1 struct {
	Index          uint64 `json:"index"`
	ValidatorIndex uint64 `json:"validatorIndex"`
	Address        Address `json:"address"`
	Amount         uint64  `json:"amount"`
}

// Validate checks that the withdrawal fields are within reasonable bounds.
func (w *WithdrawalV1) Validate() error {
	if w == nil {
		return NewValidationError("WithdrawalV1", ErrNilInput, nil)
	}
	if w.Address == (Address{}) {
		return NewValidationError("Address", ErrZeroAddress, w.Address)
	}
	if w.Index > math.MaxInt64 || w.ValidatorIndex > math.MaxInt64 || w.Amount > MaxWithdrawalValue {
		return NewValidationError("fields", ErrOutOfBounds,
			fmt.Sprintf("index=%d, validator=%d, amount=%d", w.Index, w.ValidatorIndex, w.Amount))
	}
	return nil
}

// ---------------------------------------------------------------------------
// PayloadAttributesV1
// ---------------------------------------------------------------------------

// PayloadAttributesV1 are attributes for building a new payload.
type PayloadAttributesV1 struct {
	Timestamp             uint64         `json:"timestamp"`
	Random                Hash           `json:"random"`
	SuggestedFeeRecipient Address        `json:"suggestedFeeRecipient"`
	Withdrawals           []WithdrawalV1 `json:"withdrawals,omitempty"`
	ParentBeaconBlockRoot *Hash          `json:"parentBeaconBlockRoot,omitempty"`
}

// Validate performs full validation of PayloadAttributesV1.
func (p *PayloadAttributesV1) Validate() error {
	if p == nil {
		return NewValidationError("PayloadAttributesV1", ErrNilInput, nil)
	}
	if p.Timestamp == 0 {
		return NewValidationError("Timestamp", ErrZeroTimestamp, p.Timestamp)
	}
	now := uint64(time.Now().Unix())
	if p.Timestamp > now+uint64(MaxFutureTimestampOffset.Seconds()) {
		return NewValidationError("Timestamp", ErrTimestampTooFar, p.Timestamp)
	}
	if err := p.SuggestedFeeRecipient.Validate(); err != nil {
		return NewValidationError("SuggestedFeeRecipient", err, p.SuggestedFeeRecipient)
	}
	if err := p.Random.Validate(); err != nil {
		return NewValidationError("Random", err, p.Random)
	}
	for i, w := range p.Withdrawals {
		if err := w.Validate(); err != nil {
			return NewValidationError(fmt.Sprintf("Withdrawals[%d]", i), err, w)
		}
	}
	return nil
}

// ---------------------------------------------------------------------------
// ExecutionPayloadV1
// ---------------------------------------------------------------------------

// ExecutionPayloadV1 is the block payload as defined by the Ethereum Engine API.
type ExecutionPayloadV1 struct {
	ParentHash    Hash           `json:"parentHash"`
	FeeRecipient  Address        `json:"feeRecipient"`
	StateRoot     Hash           `json:"stateRoot"`
	ReceiptsRoot  Hash           `json:"receiptsRoot"`
	LogsBloom     []byte         `json:"logsBloom"`
	PrevRandao    Hash           `json:"prevRandao"`
	BlockNumber   uint64         `json:"blockNumber"`
	GasLimit      uint64         `json:"gasLimit"`
	GasUsed       uint64         `json:"gasUsed"`
	Timestamp     uint64         `json:"timestamp"`
	ExtraData     []byte         `json:"extraData"`
	BaseFeePerGas []byte         `json:"baseFeePerGas"`
	BlockHash     Hash           `json:"blockHash"`
	Transactions  [][]byte       `json:"transactions"`
	Withdrawals   []WithdrawalV1 `json:"withdrawals,omitempty"`
	BlobGasUsed   *uint64        `json:"blobGasUsed,omitempty"`
	ExcessBlobGas *uint64        `json:"excessBlobGas,omitempty"`
}

// validateField is a helper to perform per-field validation with error wrapping.
func (p *ExecutionPayloadV1) validateField(field string, err error) error {
	if err != nil {
		return NewValidationError(field, err, nil) // value could be added via fmt
	}
	return nil
}

// Validate performs comprehensive validation of the execution payload.
// It logs validation failures at debug level for troubleshooting.
func (p *ExecutionPayloadV1) Validate() error {
	if p == nil {
		return NewValidationError("ExecutionPayloadV1", ErrNilInput, nil)
	}

	// Use a logger with structured fields for consistent output.
	logger := slog.Default().With("component", "engine_api", "block_number", p.BlockNumber)

	// Validate hashes and addresses first
	if err := p.ParentHash.Validate(); err != nil {
		logger.Debug("validation failure", "field", "ParentHash", "error", err)
		return NewValidationError("ParentHash", err, p.ParentHash)
	}
	if err := p.FeeRecipient.Validate(); err != nil {
		logger.Debug("validation failure", "field", "FeeRecipient", "error", err)
		return NewValidationError("FeeRecipient", err, p.FeeRecipient)
	}
	if err := p.StateRoot.Validate(); err != nil {
		logger.Debug("validation failure", "field", "StateRoot", "error", err)
		return NewValidationError("StateRoot", err, p.StateRoot)
	}
	if err := p.ReceiptsRoot.Validate(); err != nil {
		logger.Debug("validation failure", "field", "ReceiptsRoot", "error", err)
		return NewValidationError("ReceiptsRoot", err, p.ReceiptsRoot)
	}
	if err := p.PrevRandao.Validate(); err != nil {
		logger.Debug("validation failure", "field", "PrevRandao", "error", err)
		return NewValidationError("PrevRandao", err, p.PrevRandao)
	}
	if err := p.BlockHash.Validate(); err != nil {
		logger.Debug("validation failure", "field", "BlockHash", "error", err)
		return NewValidationError("BlockHash", err, p.BlockHash)
	}

	// Validate LogsBloom length
	if len(p.LogsBloom) != LogsBloomLength {
		err := fmt.Errorf("expected %d bytes, got %d", LogsBloomLength, len(p.LogsBloom))
		logger.Debug("validation failure", "field", "LogsBloom", "error", err)
		return NewValidationError("LogsBloom", ErrInvalidLength, len(p.LogsBloom))
	}

	// Validate GasLimit
	if p.GasLimit < MinBlockGasLimit {
		err := fmt.Errorf("gas limit %d below minimum %d", p.GasLimit, MinBlockGasLimit)
		logger.Debug("validation failure", "field", "GasLimit", "error", err)
		return NewValidationError("GasLimit", ErrGasLimitBelowMin, p.GasLimit)
	}
	if p.GasLimit > MaxBlockGasLimit {
		err := fmt.Errorf("gas limit %d above maximum %d", p.GasLimit, MaxBlockGasLimit)
		logger.Debug("validation failure", "field", "GasLimit", "error", err)
		return NewValidationError("GasLimit", ErrGasLimitAboveMax, p.GasLimit)
	}

	// Validate GasUsed
	if p.GasUsed > p.GasLimit {
		err := fmt.Errorf("gas used %d exceeds gas limit %d", p.GasUsed, p.GasLimit)
		logger.Debug("validation failure", "field", "GasUsed", "error", err)
		return NewValidationError("GasUsed", ErrGasUsedExceedsLimit, p.GasUsed)
	}

	// Validate Timestamp
	if p.Timestamp == 0 {
		logger.Debug("validation failure", "field", "Timestamp", "error", ErrZeroTimestamp)
		return NewValidationError("Timestamp", ErrZeroTimestamp, p.Timestamp)
	}
	now := uint64(time.Now().Unix())
	if p.Timestamp > now+uint64(MaxFutureTimestampOffset.Seconds()) {
		err := fmt.Errorf("timestamp %d > now+10s (%d)", p.Timestamp, now+uint64(MaxFutureTimestampOffset.Seconds()))
		logger.Debug("validation failure", "field", "Timestamp", "error", err)
		return NewValidationError("Timestamp", ErrTimestampTooFar, p.Timestamp)
	}

	// Validate ExtraData
	if len(p.ExtraData) > MaxExtraDataSize {
		err := fmt.Errorf("extra data length %d exceeds max %d", len(p.ExtraData), MaxExtraDataSize)
		logger.Debug("validation failure", "field", "ExtraData", "error", err)
		return NewValidationError("ExtraData", ErrInvalidLength, len(p.ExtraData))
	}

	// Validate BaseFeePerGas
	if len(p.BaseFeePerGas) != BaseFeePerGasLength {
		err := fmt.Errorf("base fee per gas length %d, expected %d", len(p.BaseFeePerGas), BaseFeePerGasLength)
		logger.Debug("validation failure", "field", "BaseFeePerGas", "error", err)
		return NewValidationError("BaseFeePerGas", ErrInvalidLength, len(p.BaseFeePerGas))
	}

	// Validate transactions count
	if len(p.Transactions) > MaxTransactionsCount {
		err := fmt.Errorf("transaction count %d exceeds max %d", len(p.Transactions), MaxTransactionsCount)
		logger.Debug("validation failure", "field", "Transactions", "error", err)
		return NewValidationError("Transactions", ErrExceedsMaxCount, len(p.Transactions))
	}

	// Validate each transaction (non-empty)
	for i, tx := range p.Transactions {
		if len(tx) == 0 {
			logger.Debug("validation failure", "field", fmt.Sprintf("Transactions[%d]", i), "error", ErrEmptyTransaction)
			return NewValidationError(fmt.Sprintf("Transactions[%d]", i), ErrEmptyTransaction, nil)
		}
	}

	// Validate withdrawals
	for i, w := range p.Withdrawals {
		if err := w.Validate(); err != nil {
			logger.Debug("validation failure", "field", fmt.Sprintf("Withdrawals[%d]", i), "error", err)
			return NewValidationError(fmt.Sprintf("Withdrawals[%d]", i), err, w)
		}
	}

	// Validate optional blob gas fields (only bounds check)
	if p.BlobGasUsed != nil && *p.BlobGasUsed > math.MaxUint64 {
		logger.Debug("validation failure", "field", "BlobGasUsed", "error", ErrOutOfBounds)
		return NewValidationError("BlobGasUsed", ErrOutOfBounds, *p.BlobGasUsed)
	}
	if p.ExcessBlobGas != nil && *p.ExcessBlobGas > math.MaxUint64 {
		logger.Debug("validation failure", "field", "ExcessBlobGas", "error", ErrOutOfBounds)
		return NewValidationError("ExcessBlobGas", ErrOutOfBounds, *p.ExcessBlobGas)
	}

	return nil
}

// ---------------------------------------------------------------------------
// UnmarshalJSON helpers (performance optimization: reuse buffers)
// ---------------------------------------------------------------------------

// Pool for temporary byte slices to reduce allocations when unmarshaling frequently.
var byteSlicePool = sync.Pool{
	New: func() interface{} {
		b := make([]byte, 0, 2048) // typical size for logsBloom
		return &b
	},
}

// UnmarshalJSON overrides default unmarshaling for ExecutionPayloadV1
// to allow custom validation after parsing (optional, but can be added).
// We keep the standard implementation for readability and rely on Validate() later.

// ---------------------------------------------------------------------------
// JSON serialization helpers (optional, for performance)
// ---------------------------------------------------------------------------

// MarshalJSON implements custom marshaling with pre-allocated buffer.
// (Not implemented here to keep code focused; can be added if profiling shows bottlenecks.)

// ---------------------------------------------------------------------------
// Example usage of logging in production validation flow
// ---------------------------------------------------------------------------

// ValidatePayloadWithLogging wraps Validate with additional logging context.
// This can be used at higher layers for consistent observability.
func ValidatePayloadWithLogging(p *ExecutionPayloadV1) error {
	logger := slog.Default().With(
		"component", "engine_api",
		"block_number", p.BlockNumber,
		"block_hash", p.BlockHash.String(),
	)
	start := time.Now()
	err := p.Validate()
	if err != nil {
		logger.Warn("payload validation failed", "error", err, "duration", time.Since(start))
	} else {
		logger.Debug("payload validation passed", "duration", time.Since(start))
	}
	return err
}