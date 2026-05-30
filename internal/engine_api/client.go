package engineapi

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"math"
	"math/rand"
	"net/http"
	"net/url"
	"sync"
	"sync/atomic"
	"time"
)

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const (
	defaultTimeout          = 30 * time.Second
	defaultRetryMax         = 3
	defaultRetryDelayBase   = 500 * time.Millisecond
	defaultRetryDelayMax    = 5 * time.Second
	maxResponseSize         = 10 * 1024 * 1024 // 10 MiB
	defaultMaxIdleConns     = 10
	defaultIdleConnTimeout  = 90 * time.Second
	defaultJitterFraction   = 0.2 // ±20% jitter
	jsonrpcVersion          = "2.0"
	jsonrpcContentType      = "application/json"
)

var errEmptyResult = errors.New("engine_api: empty result")

// ---------------------------------------------------------------------------
// Sentinel errors for retryable detection.
// ---------------------------------------------------------------------------

// ErrTerminal is returned when a request should not be retried.
type ErrTerminal struct {
	Err error
}

func (e *ErrTerminal) Error() string { return fmt.Sprintf("engine_api: terminal: %v", e.Err) }
func (e *ErrTerminal) Unwrap() error { return e.Err }

// ErrRetryable indicates a transient error that may succeed on retry.
type ErrRetryable struct {
	Err error
}

func (e *ErrRetryable) Error() string { return fmt.Sprintf("engine_api: retryable: %v", e.Err) }
func (e *ErrRetryable) Unwrap() error { return e.Err }

// ---------------------------------------------------------------------------
// Error types for JSON‑RPC response errors.
// ---------------------------------------------------------------------------

// RPCError represents a JSON‑RPC error.
type RPCError struct {
	Code    int
	Message string
	Method  string
}

func (e *RPCError) Error() string {
	return fmt.Sprintf("engine_api: %s: JSON‑RPC error %d: %s", e.Method, e.Code, e.Message)
}

// IsRPCError returns true if err is *RPCError.
func IsRPCError(err error) bool {
	var e *RPCError
	return errors.As(err, &e)
}

// ---------------------------------------------------------------------------
// Option & config
// ---------------------------------------------------------------------------

// Option configures an EngineClient.
type Option func(*config)

type config struct {
	timeout        time.Duration
	retryMax       int
	retryDelayBase time.Duration
	retryDelayMax  time.Duration
	logger         *slog.Logger
}

// WithTimeout sets the HTTP client timeout. Must be positive; defaults to 30s.
func WithTimeout(d time.Duration) Option {
	return func(c *config) {
		if d > 0 {
			c.timeout = d
		}
	}
}

// WithRetryMax sets the maximum retry attempts. Default is 3.
func WithRetryMax(n int) Option {
	return func(c *config) {
		if n >= 0 {
			c.retryMax = n
		}
	}
}

// WithRetryDelay sets base and max delays for exponential backoff. Both must be positive.
func WithRetryDelay(base, max time.Duration) Option {
	return func(c *config) {
		if base > 0 {
			c.retryDelayBase = base
		}
		if max > 0 && max >= base {
			c.retryDelayMax = max
		}
	}
}

// WithLogger sets a structured logger. If nil, slog.Default() is used.
func WithLogger(l *slog.Logger) Option {
	return func(c *config) {
		if l != nil {
			c.logger = l
		}
	}
}

// ---------------------------------------------------------------------------
// JSON‑RPC types
// ---------------------------------------------------------------------------

type jsonRPCRequest struct {
	JSONRPC string        `json:"jsonrpc"`
	Method  string        `json:"method"`
	Params  []interface{} `json:"params"`
	ID      int64         `json:"id"`
}

type jsonRPCResponse struct {
	JSONRPC string          `json:"jsonrpc"`
	ID      int64           `json:"id"`
	Result  json.RawMessage `json:"result,omitempty"`
	Error   *struct {
		Code    int    `json:"code"`
		Message string `json:"message"`
	} `json:"error,omitempty"`
}

// ---------------------------------------------------------------------------
// Engine API types (subset – compatible with Engine API v1/v2/v3)
// ---------------------------------------------------------------------------

// EnginePayload contains the fields for engine_newPayload (v1/v2/v3).
type EnginePayload struct {
	ParentHash    string        `json:"parentHash"`
	FeeRecipient  string        `json:"feeRecipient"`
	StateRoot     string        `json:"stateRoot"`
	ReceiptsRoot  string        `json:"receiptsRoot"`
	LogsBloom     string        `json:"logsBloom"`
	PrevRandao    string        `json:"prevRandao"`
	BlockNumber   string        `json:"blockNumber"`
	GasLimit      string        `json:"gasLimit"`
	GasUsed       string        `json:"gasUsed"`
	Timestamp     string        `json:"timestamp"`
	ExtraData     string        `json:"extraData"`
	BaseFeePerGas string        `json:"baseFeePerGas"`
	BlockHash     string        `json:"blockHash"`
	Transactions  []string      `json:"transactions"`
	Withdrawals   []Withdrawal  `json:"withdrawals,omitempty"`
}

// Withdrawal represents a withdrawal object.
type Withdrawal struct {
	Index   string `json:"index"`
	Address string `json:"address"`
	Amount  string `json:"amount"`
}

// PayloadStatus is the result of payload validation.
type PayloadStatus struct {
	Status          string `json:"status"`
	LatestValidHash string `json:"latestValidHash,omitempty"`
	ValidationError string `json:"validationError,omitempty"`
}

// ForkchoiceState describes the head of the chain from the CL perspective.
type ForkchoiceState struct {
	HeadBlockHash      string `json:"headBlockHash"`
	SafeBlockHash      string `json:"safeBlockHash"`
	FinalizedBlockHash string `json:"finalizedBlockHash"`
}

// ForkchoiceResponse is the result of engine_forkchoiceUpdated.
type ForkchoiceResponse struct {
	PayloadStatus PayloadStatus `json:"payloadStatus"`
	PayloadID     string        `json:"payloadId,omitempty"`
}

// ExchangeCapabilitiesResponse holds the result of engine_exchangeCapabilities.
type ExchangeCapabilitiesResponse struct {
	Capabilities []string `json:"capabilities"`
}

// ExecutionPayloadBodyV1 holds execution payload body.
type ExecutionPayloadBodyV1 struct {
	Transactions []string     `json:"transactions"`
	Withdrawals  []Withdrawal `json:"withdrawals,omitempty"`
}

// ---------------------------------------------------------------------------
// EngineClient
// ---------------------------------------------------------------------------

// EngineClient is a thread‑safe HTTP client for the Ethereum Engine API (Reth).
// It provides methods for payload submission, forkchoice updates, capability negotiation
// and optimistic payload building.
type EngineClient struct {
	endpoint string
	client   *http.Client
	cfg      config
	logger   *slog.Logger
	reqID    atomic.Int64 // JSON‑RPC request ID generator
}

// NewEngineClient creates a new client. The endpoint must be a valid http/https URL.
func NewEngineClient(endpoint string, opts ...Option) (*EngineClient, error) {
	if endpoint == "" {
		return nil, errors.New("engine_api: endpoint must not be empty")
	}
	u, err := url.Parse(endpoint)
	if err != nil {
		return nil, fmt.Errorf("engine_api: invalid endpoint URL: %w", err)
	}
	if u.Scheme != "http" && u.Scheme != "https" {
		return nil, fmt.Errorf("engine_api: endpoint scheme must be http or https, got %q", u.Scheme)
	}
	if u.Host == "" {
		return nil, errors.New("engine_api: endpoint must include a host")
	}

	cfg := config{
		timeout:        defaultTimeout,
		retryMax:       defaultRetryMax,
		retryDelayBase: defaultRetryDelayBase,
		retryDelayMax:  defaultRetryDelayMax,
		logger:         slog.Default(),
	}
	for _, o := range opts {
		o(&cfg)
	}

	tr := &http.Transport{
		MaxIdleConns:        defaultMaxIdleConns,
		IdleConnTimeout:     defaultIdleConnTimeout,
		DisableCompression:  false,
		ForceAttemptHTTP2:   true,
	}

	client := &http.Client{
		Timeout:   cfg.timeout,
		Transport: tr,
	}

	logger := cfg.logger.With("component", "engine_client", "endpoint", endpoint)

	return &EngineClient{
		endpoint: endpoint,
		client:   client,
		cfg:      cfg,
		logger:   logger,
	}, nil
}

// ---------------------------------------------------------------------------
// core JSON‑RPC call with retry & context support
// ---------------------------------------------------------------------------

// callContext performs a JSON‑RPC request with retry logic and context propagation.
// It returns the raw result bytes, or an error indicating terminal/retryable failures.
func (c *EngineClient) callContext(ctx context.Context, method string, params []interface{}) (json.RawMessage, error) {
	reqID := c.reqID.Add(1)
	reqBody := jsonRPCRequest{
		JSONRPC: jsonrpcVersion,
		Method:  method,
		Params:  params,
		ID:      reqID,
	}

	bodyBytes, err := json.Marshal(reqBody)
	if err != nil {
		return nil, &ErrTerminal{Err: fmt.Errorf("marshal request: %w", err)}
	}

	var respBytes []byte
	err = c.retry(ctx, func(ctx context.Context) error {
		req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.endpoint, bytes.NewReader(bodyBytes))
		if err != nil {
			return &ErrTerminal{Err: fmt.Errorf("create request: %w", err)}
		}
		req.Header.Set("Content-Type", jsonrpcContentType)
		req.Header.Set("Accept", jsonrpcContentType)

		resp, err := c.client.Do(req)
		if err != nil {
			// Network/timeout errors are retryable
			return &ErrRetryable{Err: fmt.Errorf("http do: %w", err)}
		}
		defer resp.Body.Close()

		if resp.StatusCode == http.StatusServiceUnavailable || resp.StatusCode == http.StatusTooManyRequests {
			// Retry on 503/429
			body, _ := io.ReadAll(io.LimitReader(resp.Body, maxResponseSize))
			return &ErrRetryable{Err: fmt.Errorf("http %d: %s", resp.StatusCode, string(body))}
		}
		if resp.StatusCode < 200 || resp.StatusCode >= 300 {
			body, _ := io.ReadAll(io.LimitReader(resp.Body, maxResponseSize))
			return &ErrTerminal{Err: fmt.Errorf("http %d: %s", resp.StatusCode, string(body))}
		}

		respBytes, err = io.ReadAll(io.LimitReader(resp.Body, maxResponseSize))
		if err != nil {
			return &ErrRetryable{Err: fmt.Errorf("read body: %w", err)}
		}
		return nil
	})
	if err != nil {
		return nil, err
	}

	var rpcResp jsonRPCResponse
	if err := json.Unmarshal(respBytes, &rpcResp); err != nil {
		return nil, &ErrTerminal{Err: fmt.Errorf("unmarshal response: %w", err)}
	}
	if rpcResp.Error != nil {
		// Standard JSON‑RPC errors are terminal (e.g., method not found, invalid params)
		return nil, &RPCError{
			Code:    rpcResp.Error.Code,
			Message: rpcResp.Error.Message,
			Method:  method,
		}
	}
	if len(rpcResp.Result) == 0 {
		return nil, errEmptyResult
	}
	return rpcResp.Result, nil
}

// retry executes the given function with exponential backoff and jitter.
// It respects context cancellation. The function f should return *ErrTerminal or *ErrRetryable.
func (c *EngineClient) retry(ctx context.Context, f func(context.Context) error) error {
	var lastErr error
	for attempt := 0; attempt <= c.cfg.retryMax; attempt++ {
		if attempt > 0 {
			// Calculate delay with exponential backoff and jitter
			delay := float64(c.cfg.retryDelayBase) * math.Pow(2, float64(attempt-1))
			if delay > float64(c.cfg.retryDelayMax) {
				delay = float64(c.cfg.retryDelayMax)
			}
			// Add jitter: ± defaultJitterFraction
			jitter := delay * defaultJitterFraction
			delay = delay - jitter + 2*jitter*rand.Float64()
			select {
			case <-ctx.Done():
				return ctx.Err()
			case <-time.After(time.Duration(delay)):
			}
		}

		err := f(ctx)
		if err == nil {
			return nil
		}

		lastErr = err
		var terminal *ErrTerminal
		if errors.As(err, &terminal) {
			return terminal.Err
		}
		// If it's not retryable, break (shouldn't happen, but be safe)
		var retryable *ErrRetryable
		if !errors.As(err, &retryable) {
			return err
		}

		// Log retry attempt
		c.logger.Debug("retrying request", "attempt", attempt+1, "max", c.cfg.retryMax, "error", retryable.Err)
	}
	return fmt.Errorf("engine_api: max retries exceeded: %w", lastErr)
}

// ---------------------------------------------------------------------------
// Public Engine API methods
// ---------------------------------------------------------------------------

// NewPayload sends engine_newPayload to the execution engine.
// It validates and optionally executes a block payload.
func (c *EngineClient) NewPayload(ctx context.Context, payload EnginePayload) (*PayloadStatus, error) {
	const method = "engine_newPayload"
	raw, err := c.callContext(ctx, method, []interface{}{payload})
	if err != nil {
		return nil, fmt.Errorf("%s: %w", method, err)
	}

	var status PayloadStatus
	if err := json.Unmarshal(raw, &status); err != nil {
		return nil, fmt.Errorf("%s: unmarshal result: %w", method, err)
	}
	return &status, nil
}

// ForkchoiceUpdated sends engine_forkchoiceUpdated to the execution engine.
// It updates the forkchoice state and optionally triggers payload building.
func (c *EngineClient) ForkchoiceUpdated(ctx context.Context, fcs ForkchoiceState, payloadAttributes interface{}) (*ForkchoiceResponse, error) {
	const method = "engine_forkchoiceUpdated"
	params := []interface{}{fcs}
	if payloadAttributes != nil {
		params = append(params, payloadAttributes)
	}

	raw, err := c.callContext(ctx, method, params)
	if err != nil {
		return nil, fmt.Errorf("%s: %w", method, err)
	}

	var resp ForkchoiceResponse
	if err := json.Unmarshal(raw, &resp); err != nil {
		return nil, fmt.Errorf("%s: unmarshal result: %w", method, err)
	}
	return &resp, nil
}

// GetPayload retrieves a payload built by the engine after a forkchoiceUpdated call with payload attributes.
func (c *EngineClient) GetPayload(ctx context.Context, payloadID string) (*EnginePayload, error) {
	const method = "engine_getPayloadV2"
	raw, err := c.callContext(ctx, method, []interface{}{payloadID})
	if err != nil {
		return nil, fmt.Errorf("%s: %w", method, err)
	}

	var payload EnginePayload
	if err := json.Unmarshal(raw, &payload); err != nil {
		return nil, fmt.Errorf("%s: unmarshal result: %w", method, err)
	}
	return &payload, nil
}

// ExchangeCapabilities negotiates supported Engine API methods with the execution engine.
func (c *EngineClient) ExchangeCapabilities(ctx context.Context, capabilities []string) (*ExchangeCapabilitiesResponse, error) {
	const method = "engine_exchangeCapabilities"
	raw, err := c.callContext(ctx, method, []interface{}{capabilities})
	if err != nil {
		return nil, fmt.Errorf("%s: %w", method, err)
	}

	var resp ExchangeCapabilitiesResponse
	if err := json.Unmarshal(raw, &resp); err != nil {
		return nil, fmt.Errorf("%s: unmarshal result: %w", method, err)
	}
	return &resp, nil
}

// GetPayloadBodiesByHash retrieves execution payload bodies by block hash.
// It is available starting from Engine API v3.
func (c *EngineClient) GetPayloadBodiesByHash(ctx context.Context, hashes []string) ([]*ExecutionPayloadBodyV1, error) {
	const method = "engine_getPayloadBodiesByHashV1"
	raw, err := c.callContext(ctx, method, []interface{}{hashes})
	if err != nil {
		return nil, fmt.Errorf("%s: %w", method, err)
	}

	var bodies []*ExecutionPayloadBodyV1
	if err := json.Unmarshal(raw, &bodies); err != nil {
		return nil, fmt.Errorf("%s: unmarshal result: %w", method, err)
	}
	return bodies, nil
}

// GetPayloadBodiesByRange retrieves execution payload bodies by block number range.
// It is available starting from Engine API v3.
func (c *EngineClient) GetPayloadBodiesByRange(ctx context.Context, start, count uint64) ([]*ExecutionPayloadBodyV1, error) {
	const method = "engine_getPayloadBodiesByRangeV1"
	params := []interface{}{fmt.Sprintf("0x%x", start), fmt.Sprintf("0x%x", count)}
	raw, err := c.callContext(ctx, method, params)
	if err != nil {
		return nil, fmt.Errorf("%s: %w", method, err)
	}

	var bodies []*ExecutionPayloadBodyV1
	if err := json.Unmarshal(raw, &bodies); err != nil {
		return nil, fmt.Errorf("%s: unmarshal result: %w", method, err)
	}
	return bodies, nil
}

// Close shuts down the underlying HTTP client's idle connections.
func (c *EngineClient) Close() error {
	c.client.CloseIdleConnections()
	return nil
}