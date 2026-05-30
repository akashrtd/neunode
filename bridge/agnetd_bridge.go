package bridge

import (
	"context"
	"crypto/ecdsa"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"log"
	"math/big"
	"net/http"
	"sync"
	"sync/atomic"
	"time"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
	"github.com/gorilla/websocket"
)

// ============================================================================
// Constants
// ============================================================================

const (
	// L1 WebSocket endpoint – replace with actual Reth WS URL
	L1WSEndpoint     = "ws://localhost:8546"
	L1HTTPEndpoint   = "http://localhost:8545"
	ReconnectInterval = 5 * time.Second
	WriteTimeout      = 10 * time.Second
	ReadLimit         = 65536
)

// Predeployed contract addresses (must match actual deployment)
var (
	AgentStateAddr   = common.HexToAddress("0x0000000000000000000000000000000000000001")
	BountyContractAddr = common.HexToAddress("0x0000000000000000000000000000000000000002")
	TokenContractAddr  = common.HexToAddress("0x0000000000000000000000000000000000000003")
)

// Event topic hashes (keccak256 of event signature)
var (
	AgentStateChangedTopic = crypto.Keccak256Hash([]byte("AgentStateChanged(address,uint256,bytes32)"))
	BountyCreatedTopic    = crypto.Keccak256Hash([]byte("BountyCreated(uint256,address,uint256)"))
	BountyClaimedTopic    = crypto.Keccak256Hash([]byte("BountyClaimed(uint256,address)"))
	TransferTopic         = crypto.Keccak256Hash([]byte("Transfer(address,address,uint256)"))
)

// ============================================================================
// Types
// ============================================================================

// AgentState holds on-chain agent state.
type AgentState struct {
	Agent  common.Address `json:"agent"`
	Nonce  uint64         `json:"nonce"`
	State  [32]byte       `json:"state"` // e.g., Merkle root
	Active bool           `json:"active"`
}

// Bounty represents an on-chain bounty.
type Bounty struct {
	ID       *big.Int       `json:"id"`
	Creator  common.Address `json:"creator"`
	Amount   *big.Int       `json:"amount"`
	Claimed  bool           `json:"claimed"`
	Claimant common.Address `json:"claimant,omitempty"`
}

// TokenBalance holds balance of an address for the `neu` token (native or ERC20).
type TokenBalance struct {
	Address common.Address `json:"address"`
	Balance *big.Int       `json:"balance"`
}

// TransactionRequest describes a transaction to be sent.
type TransactionRequest struct {
	To       common.Address `json:"to"`
	Value    *big.Int       `json:"value"`
	Data     []byte         `json:"data"`
	GasLimit uint64         `json:"gasLimit"`
	GasPrice *big.Int       `json:"gasPrice,omitempty"` // if nil, client estimates
}

// Bridge is the main bridge module for agnetd ↔ L1 communication.
type Bridge struct {
	mu            sync.RWMutex
	wsConn        *websocket.Conn
	httpClient    *ethclient.Client
	signer        *ecdsa.PrivateKey
	signerAddr    common.Address
	nextNonce     uint64
	closeCh       chan struct{}
	reconnectCh   chan struct{}
	subscriptions map[common.Hash][]EventCallback // topic -> callbacks
	logger        *log.Logger
	running       atomic.Bool
}

// EventCallback is called when a matching event log is received.
type EventCallback func(types.Log)

// Option allows configuring the bridge.
type Option func(*Bridge)

// WithLogger sets a custom logger.
func WithLogger(l *log.Logger) Option {
	return func(b *Bridge) {
		b.logger = l
	}
}

// WithSigner sets the ECDSA signer (private key) for transaction signing.
func WithSigner(key *ecdsa.PrivateKey) Option {
	return func(b *Bridge) {
		b.signer = key
		b.signerAddr = crypto.PubkeyToAddress(key.PublicKey)
	}
}

// ============================================================================
// Constructor
// ============================================================================

// NewBridge creates a Bridge instance. Start() must be called to begin operation.
func NewBridge(opts ...Option) (*Bridge, error) {
	httpClient, err := ethclient.DialContext(context.Background(), L1HTTPEndpoint)
	if err != nil {
		return nil, fmt.Errorf("bridge: failed to dial HTTP client: %w", err)
	}

	b := &Bridge{
		httpClient:    httpClient,
		closeCh:       make(chan struct{}),
		reconnectCh:   make(chan struct{}, 1),
		subscriptions: make(map[common.Hash][]EventCallback),
		logger:        log.New(log.Writer(), "[bridge] ", log.LstdFlags),
	}

	for _, opt := range opts {
		opt(b)
	}

	return b, nil
}

// ============================================================================
// Public API
// ============================================================================

// Start connects to L1 WebSocket and begins event subscription and reconnection.
// Blocks until the context is cancelled or a fatal error occurs.
func (b *Bridge) Start(ctx context.Context) error {
	if !b.running.CompareAndSwap(false, true) {
		return fmt.Errorf("bridge: already running")
	}
	defer b.running.Store(false)

	// Connect initially
	if err := b.connect(ctx); err != nil {
		return fmt.Errorf("bridge: initial connection failed: %w", err)
	}

	// Start event listening goroutine
	go b.eventLoop(ctx)

	// Main loop: handle reconnections and context cancellation
	for {
		select {
		case <-ctx.Done():
			b.logger.Println("bridge: context done, shutting down")
			b.shutdown()
			return ctx.Err()
		case <-b.reconnectCh:
			b.logger.Println("bridge: reconnecting...")
			for {
				if err := b.connect(ctx); err != nil {
					b.logger.Printf("bridge: reconnection failed: %v, retrying in %v", err, ReconnectInterval)
					select {
					case <-time.After(ReconnectInterval):
						continue
					case <-ctx.Done():
						return ctx.Err()
					}
				}
				break
			}
			b.logger.Println("bridge: reconnected")
		}
	}
}

// Subscribe adds a callback for a specific event topic hash.
// Returns an unsubscribe function.
func (b *Bridge) Subscribe(topic common.Hash, callback EventCallback) func() {
	b.mu.Lock()
	defer b.mu.Unlock()
	b.subscriptions[topic] = append(b.subscriptions[topic], callback)

	// Return unsub function
	idx := len(b.subscriptions[topic]) - 1
	return func() {
		b.mu.Lock()
		defer b.mu.Unlock()
		slice := b.subscriptions[topic]
		if idx < len(slice) {
			b.subscriptions[topic] = append(slice[:idx], slice[idx+1:]...)
		}
		// Clean up empty topic
		if len(b.subscriptions[topic]) == 0 {
			delete(b.subscriptions, topic)
		}
	}
}

// ---------------------------------------------------------------------------
// Query Interface
// ---------------------------------------------------------------------------

// GetAgentState fetches the latest on-chain state of an agent.
func (b *Bridge) GetAgentState(ctx context.Context, agentAddr common.Address) (*AgentState, error) {
	// We assume AgentState contract has a method `getAgentState(address) returns (uint64 nonce, bytes32 state, bool active)`
	// Encode call data manually to avoid importing full ABI.
	callData := append([]byte{0x8b, 0x3b, 0x3b, 0x3b}, agentAddr.Hash().Bytes()[:]...) // placeholder
	// Better: use abi pack from go-ethereum/accounts/abi but we need ABI. For production, embed ABI.
	// Simplified: we use eth_call with known method signature.
	// We'll pack using standard method: getAgentState(address)
	// Keccak256("getAgentState(address)") -> 0x8b... we'll use computed.
	// Load the ABI from a generated file in production.
	// For brevity, assume we have a function that packs calls correctly.
	// Replace with proper packing.
	packed, err := packGetAgentState(agentAddr)
	if err != nil {
		return nil, fmt.Errorf("bridge: pack call data: %w", err)
	}

	msg := ethereum.CallMsg{
		To:   &AgentStateAddr,
		Data: packed,
	}
	result, err := b.httpClient.CallContract(ctx, msg, nil)
	if err != nil {
		return nil, fmt.Errorf("bridge: call contract: %w", err)
	}
	return unpackAgentState(result)
}

// GetBounties retrieves all active bounties.
func (b *Bridge) GetBounties(ctx context.Context) ([]Bounty, error) {
	// Placeholder: in production, call Bounty contract method like `getAllBounties()`
	// For now, we'll read from events stored in DB; but return empty.
	return nil, fmt.Errorf("not implemented: use event subscription or direct contract call")
}

// GetTokenBalance returns the `neu` token balance for a given address.
// If native token, use eth_getBalance.
// If ERC20, call token contract.
func (b *Bridge) GetTokenBalance(ctx context.Context, addr common.Address) (*TokenBalance, error) {
	// Since `neu` is the native gas token (ETH equivalent), we use eth_getBalance.
	bal, err := b.httpClient.BalanceAt(ctx, addr, nil)
	if err != nil {
		return nil, fmt.Errorf("bridge: get balance: %w", err)
	}
	return &TokenBalance{Address: addr, Balance: bal}, nil
}

// ---------------------------------------------------------------------------
// Transaction Writing
// ---------------------------------------------------------------------------

// SendTransaction signs and broadcasts a transaction. Returns transaction hash.
func (b *Bridge) SendTransaction(ctx context.Context, req TransactionRequest) (common.Hash, error) {
	if b.signer == nil {
		return common.Hash{}, fmt.Errorf("bridge: no signer configured")
	}

	nonce, err := b.getNextNonce(ctx)
	if err != nil {
		return common.Hash{}, fmt.Errorf("bridge: get nonce: %w", err)
	}

	gasLimit := req.GasLimit
	if gasLimit == 0 {
		// Estimate gas
		msg := ethereum.CallMsg{
			From:  b.signerAddr,
			To:    &req.To,
			Value: req.Value,
			Data:  req.Data,
		}
		estimated, err := b.httpClient.EstimateGas(ctx, msg)
		if err != nil {
			return common.Hash{}, fmt.Errorf("bridge: estimate gas: %w", err)
		}
		gasLimit = estimated
	}

	gasPrice := req.GasPrice
	if gasPrice == nil {
		gasPrice, err = b.httpClient.SuggestGasPrice(ctx)
		if err != nil {
			return common.Hash{}, fmt.Errorf("bridge: suggest gas price: %w", err)
		}
	}

	chainID, err := b.httpClient.ChainID(ctx)
	if err != nil {
		return common.Hash{}, fmt.Errorf("bridge: chain id: %w", err)
	}

	tx := types.NewTransaction(
		nonce,
		req.To,
		req.Value,
		gasLimit,
		gasPrice,
		req.Data,
	)

	signedTx, err := types.SignTx(tx, types.NewEIP155Signer(chainID), b.signer)
	if err != nil {
		return common.Hash{}, fmt.Errorf("bridge: sign tx: %w", err)
	}

	if err := b.httpClient.SendTransaction(ctx, signedTx); err != nil {
		return common.Hash{}, fmt.Errorf("bridge: send tx: %w", err)
	}

	// Update local nonce tracker
	atomic.AddUint64(&b.nextNonce, 1)

	return signedTx.Hash(), nil
}

// ============================================================================
// Internal Helpers
// ============================================================================

// connect establishes a WebSocket connection and subscribes to all configured topics.
func (b *Bridge) connect(ctx context.Context) error {
	b.mu.Lock()
	defer b.mu.Unlock()

	// Close existing connection
	if b.wsConn != nil {
		b.wsConn.Close()
	}

	dialer := websocket.DefaultDialer
	conn, _, err := dialer.DialContext(ctx, L1WSEndpoint, nil)
	if err != nil {
		return fmt.Errorf("websocket dial: %w", err)
	}
	conn.SetReadLimit(ReadLimit)
	b.wsConn = conn

	// Subscribe to new block headers to catch events? Actually we need event logs.
	// Use eth_subscribe for logs filtering all contracts.
	// Build subscription request for all relevant addresses.
	params := map[string]interface{}{
		"address": []common.Address{AgentStateAddr, BountyContractAddr, TokenContractAddr},
	}
	subReq := map[string]interface{}{
		"jsonrpc": "2.0",
		"id":      1,
		"method":  "eth_subscribe",
		"params":  []interface{}{"logs", params},
	}

	if err := conn.WriteJSON(subReq); err != nil {
		return fmt.Errorf("subscribe write: %w", err)
	}

	// Read subscription ID response
	var subResp struct {
		JSONRPC string `json:"jsonrpc"`
		ID      int    `json:"id"`
		Result  string `json:"result"`
	}
	if err := conn.ReadJSON(&subResp); err != nil {
		return fmt.Errorf("subscribe response: %w", err)
	}
	b.logger.Printf("bridge: subscribed with id %s", subResp.Result)

	// Start ping/pong
	go b.keepAlive(ctx)

	return nil
}

// eventLoop reads logs from the WebSocket connection and dispatches them.
func (b *Bridge) eventLoop(ctx context.Context) {
	for {
		select {
		case <-ctx.Done():
			return
		default:
		}

		b.mu.RLock()
		conn := b.wsConn
		b.mu.RUnlock()

		if conn == nil {
			select {
			case <-time.After(100 * time.Millisecond):
			case <-ctx.Done():
				return
			}
			continue
		}

		_, message, err := conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseNormalClosure) {
				b.logger.Printf("bridge: websocket error: %v", err)
			}
			// Signal reconnection
			select {
			case b.reconnectCh <- struct{}{}:
			default:
			}
			return
		}

		// Parse subscription notification
		var notification struct {
			Params struct {
				Subscription string  `json:"subscription"`
				Result       types.Log `json:"result"`
			} `json:"params"`
		}
		if err := json.Unmarshal(message, &notification); err != nil {
			b.logger.Printf("bridge: unmarshal notification: %v", err)
			continue
		}

		log := notification.Result
		b.dispatchLog(log)
	}
}

// dispatchLog calls all registered callbacks for the log's topics.
func (b *Bridge) dispatchLog(log types.Log) {
	b.mu.RLock()
	defer b.mu.RUnlock()
	for _, topic := range log.Topics {
		if callbacks, ok := b.subscriptions[topic]; ok {
			for _, cb := range callbacks {
				// Fire and forget – callback should not block.
				go cb(log)
			}
		}
	}
}

// keepAlive sends periodic ping frames.
func (b *Bridge) keepAlive(ctx context.Context) {
	ticker := time.NewTicker(30 * time.Second)
	defer ticker.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			b.mu.RLock()
			conn := b.wsConn
			b.mu.RUnlock()
			if conn != nil {
				if err := conn.WriteControl(websocket.PingMessage, []byte{}, time.Now().Add(WriteTimeout)); err != nil {
					b.logger.Printf("bridge: ping error: %v", err)
					b.reconnect()
				}
			}
		}
	}
}

// reconnect signals the main loop to reconnect.
func (b *Bridge) reconnect() {
	select {
	case b.reconnectCh <- struct{}{}:
	default:
	}
}

// shutdown closes all connections and resources.
func (b *Bridge) shutdown() {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.wsConn != nil {
		b.wsConn.Close()
	}
	if b.httpClient != nil {
		b.httpClient.Close()
	}
}

// getNextNonce returns the current nonce and increments local tracker.
// It fetches the account nonce on first call.
func (b *Bridge) getNextNonce(ctx context.Context) (uint64, error) {
	if n := atomic.LoadUint64(&b.nextNonce); n != 0 {
		return atomic.AddUint64(&b.nextNonce, 0), nil
	}
	nonce, err := b.httpClient.NonceAt(ctx, b.signerAddr, nil)
	if err != nil {
		return 0, err
	}
	atomic.StoreUint64(&b.nextNonce, nonce)
	return nonce, nil
}

// packGetAgentState encodes the call data for getAgentState(address).
func packGetAgentState(addr common.Address) ([]byte, error) {
	// method = keccak256("getAgentState(address)")[:4]
	method := crypto.Keccak256([]byte("getAgentState(address)"))[:4]
	data := make([]byte, 36)
	copy(data[0:4], method)
	copy(data[16:36], addr.Bytes())
	return data, nil
}

// unpackAgentState decodes the return data of getAgentState.
func unpackAgentState(data []byte) (*AgentState, error) {
	if len(data) < 64 {
		return nil, fmt.Errorf("bridge: invalid response length %d", len(data))
	}
	state := &AgentState{}
	state.Nonce = new(big.Int).SetBytes(data[0:32]).Uint64()
	copy(state.State[:], data[32:64])
	state.Active = len(data) > 64 && data[64] != 0
	return state, nil
}

// ============================================================================
// Custom types for JSON unmarshal of big.Int
// ============================================================================

// bigIntJSON helps unmarshal large numbers from hex.
type bigIntJSON struct {
	*big.Int
}

func (b *bigIntJSON) UnmarshalJSON(p []byte) error {
	if string(p) == "null" {
		return nil
	}
	z := big.NewInt(0)
	_, ok := z.SetString(string(p), 0) // hex or decimal
	if !ok {
		// Try to parse as hex number from JSON string "0x..."
		var s string
		if err := json.Unmarshal(p, &s); err != nil {
			return err
		}
		s = s[2:] // remove "0x"
		if _, ok := z.SetString(s, 16); !ok {
			return fmt.Errorf("bridge: cannot parse big.Int: %s", p)
		}
	}
	b.Int = z
	return nil
}