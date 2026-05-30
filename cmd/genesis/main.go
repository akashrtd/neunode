package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"math/big"
	"os"
)

// ELGenesisConfig represents the execution layer genesis (Reth).
type ELGenesisConfig struct {
	ChainID             uint64         `json:"chainId"`
	ShanghaiBlock       uint64         `json:"shanghaiBlock,omitempty"`
	CancunBlock         uint64         `json:"cancunBlock,omitempty"`
	PragueBlock         uint64         `json:"pragueBlock,omitempty"`
	VerkleBlock         uint64         `json:"verkleBlock,omitempty"`
	TerminalTotalDifficulty *uint64    `json:"terminalTotalDifficulty,omitempty"`
	TerminalTotalDifficultyPassed bool `json:"terminalTotalDifficultyPassed,omitempty"`
}

// GenesisAllocEntry represents an account allocation (balance, code, storage).
type GenesisAllocEntry struct {
	Balance *string           `json:"balance,omitempty"` // hex string
	Code    *string           `json:"code,omitempty"`    // hex string
	Storage map[string]string `json:"storage,omitempty"`
	Nonce   *uint64           `json:"nonce,omitempty"`
}

// CLValidator represents a Malachite/CometBFT validator.
type CLValidator struct {
	Address string `json:"address"` // hex-encoded validator address
	PubKey  string `json:"pub_key"` // type + value (JSON object)
	Power   string `json:"power"`   // voting power as string
	Name    string `json:"name,omitempty"`
}

// CLGenesisConfig represents the consensus layer genesis (Malachite).
type CLGenesisConfig struct {
	Validators []CLValidator `json:"validators"`
}

// Genesis is the unified genesis structure for Reth + Malachite.
type Genesis struct {
	Config       ELGenesisConfig             `json:"config"`
	Nonce        uint64                      `json:"nonce"`
	Timestamp    uint64                      `json:"timestamp"`
	ExtraData    string                      `json:"extraData"`
	GasLimit     uint64                      `json:"gasLimit"`
	Difficulty   *uint64                     `json:"difficulty,omitempty"`
	MixHash      string                      `json:"mixHash"`
	Coinbase     string                      `json:"coinbase"`
	Alloc        map[string]GenesisAllocEntry `json:"alloc"`
	Number       uint64                      `json:"number"`
	GasUsed      uint64                      `json:"gasUsed"`
	ParentHash   string                      `json:"parentHash"`
	BaseFee      *uint64                     `json:"baseFeePerGas,omitempty"`
	ExcessBlobGas *uint64                    `json:"excessBlobGas,omitempty"`
	BlobGasUsed  *uint64                     `json:"blobGasUsed,omitempty"`

	// Consensus layer validators (injected during genesis generation).
	Validators []CLValidator `json:"validators,omitempty"`
}

func mustHexToWei(neu string) string {
	// 1 neu = 10^18 wei. Convert integer string to hex.
	n := new(big.Int)
	n, ok := n.SetString(neu, 10)
	if !ok {
		panic(fmt.Sprintf("invalid neu amount: %s", neu))
	}
	// multiply by 10^18
	n.Mul(n, big.NewInt(1e18))
	return "0x" + n.Text(16)
}

func ptrUint64(v uint64) *uint64 { return &v }

func main() {
	// Command-line flags
	chainID := flag.Uint64("chain-id", 7070, "Chain ID for the Neunode L1")
	output := flag.String("output", "genesis.json", "Path to output genesis file")
	timestamp := flag.Uint64("timestamp", 0, "Genesis timestamp (0 = default)")
	gasLimit := flag.Uint64("gas-limit", 30000000, "Genesis gas limit")
	coinbase := flag.String("coinbase", "0x0000000000000000000000000000000000000000", "Coinbase address")
	extraData := flag.String("extra-data", "Neunode Genesis", "Extra data field")
	baseFee := flag.Uint64("base-fee", 1, "Initial base fee per gas (in wei)")

	var validators validatorFlags
	flag.Var(&validators, "validator", "Validator in format address,pubkey_json,power (can be repeated)")

	// Predeployed contracts: address:codehash or address:codefile? For simplicity use inline hex.
	var predeploys predeployFlags
	flag.Var(&predeploys, "predeploy", "Predeployed contract: address,code_hex (can be repeated)")

	// Initial neu allocations: address:amount_in_neu
	var allocations allocFlags
	flag.Var(&allocations, "alloc", "Initial allocation: address,amount_neu (can be repeated)")

	flag.Parse()

	// Build EL config
	elConfig := ELGenesisConfig{
		ChainID:       *chainID,
		ShanghaiBlock: 0,
		CancunBlock:   0,
		PragueBlock:   0,
		VerkleBlock:   0,
		TerminalTotalDifficulty: ptrUint64(0),
		TerminalTotalDifficultyPassed: true,
	}

	// Build alloc map
	alloc := make(map[string]GenesisAllocEntry)

	// Add predeployed contracts
	for _, p := range predeploys {
		code := p.code
		alloc[p.address] = GenesisAllocEntry{
			Balance: ptrStr("0x0"),
			Code:    &code,
			Nonce:   ptrUint64(1),
		}
	}

	// Add initial neu allocations (convert to wei)
	for _, a := range allocations {
		wei := mustHexToWei(a.amount)
		alloc[a.address] = GenesisAllocEntry{
			Balance: &wei,
			Nonce:   ptrUint64(0),
		}
	}

	// Build CL validators
	var cls []CLValidator
	for _, v := range validators {
		cls = append(cls, CLValidator{
			Address: v.address,
			PubKey:  v.pubKey,
			Power:   v.power,
		})
	}

	// Construct genesis
	gen := Genesis{
		Config:     elConfig,
		Nonce:      0,
		Timestamp:  *timestamp,
		ExtraData:  "0x" + bytesToHex([]byte(*extraData)),
		GasLimit:   *gasLimit,
		Difficulty: ptrUint64(1),
		MixHash:    "0x0000000000000000000000000000000000000000000000000000000000000000",
		Coinbase:   *coinbase,
		Alloc:      alloc,
		Number:     0,
		GasUsed:    0,
		ParentHash: "0x0000000000000000000000000000000000000000000000000000000000000000",
		BaseFee:    ptrUint64(*baseFee),

		Validators: cls,
	}

	// Marshal to JSON with indentation
	data, err := json.MarshalIndent(gen, "", "  ")
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error marshaling genesis: %v\n", err)
		os.Exit(1)
	}

	// Write to output
	if err := os.WriteFile(*output, data, 0644); err != nil {
		fmt.Fprintf(os.Stderr, "Error writing genesis file: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Genesis written to %s with %d allocations, %d predeploys, %d validators\n",
		*output, len(allocations), len(predeploys), len(validators))
}

// Helper types for flags

type validatorEntry struct {
	address string
	pubKey  string
	power   string
}

type validatorFlags []validatorEntry

func (v *validatorFlags) String() string {
	return ""
}

func (v *validatorFlags) Set(value string) error {
	// Format address,pub_key_json,power
	parts := split3(value)
	if parts == nil {
		return fmt.Errorf("invalid validator format, expected address,pubkey_json,power")
	}
	*v = append(*v, validatorEntry{address: parts[0], pubKey: parts[1], power: parts[2]})
	return nil
}

type predeployEntry struct {
	address string
	code    string
}

type predeployFlags []predeployEntry

func (p *predeployFlags) String() string { return "" }

func (p *predeployFlags) Set(value string) error {
	parts := split2(value)
	if parts == nil {
		return fmt.Errorf("invalid predeploy format, expected address,code_hex")
	}
	*p = append(*p, predeployEntry{address: parts[0], code: parts[1]})
	return nil
}

type allocEntry struct {
	address string
	amount  string
}

type allocFlags []allocEntry

func (a *allocFlags) String() string { return "" }

func (a *allocFlags) Set(value string) error {
	parts := split2(value)
	if parts == nil {
		return fmt.Errorf("invalid alloc format, expected address,amount_neu")
	}
	*a = append(*a, allocEntry{address: parts[0], amount: parts[1]})
	return nil
}

// Utility functions

func split2(s string) []string {
	for i := 0; i < len(s); i++ {
		if s[i] == ',' {
			return []string{s[:i], s[i+1:]}
		}
	}
	return nil
}

func split3(s string) []string {
	first := -1
	second := -1
	for i := 0; i < len(s); i++ {
		if s[i] == ',' {
			if first == -1 {
				first = i
			} else if second == -1 {
				second = i
				return []string{s[:first], s[first+1 : second], s[second+1:]}
			}
		}
	}
	return nil
}

func bytesToHex(b []byte) string {
	const hexChars = "0123456789abcdef"
	res := make([]byte, len(b)*2)
	for i, v := range b {
		res[i*2] = hexChars[v>>4]
		res[i*2+1] = hexChars[v&0xf]
	}
	return string(res)
}

func ptrStr(s string) *string { return &s }