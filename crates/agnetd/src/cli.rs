use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "agnetd", version, about = "Neunode AI Agent CLI")]
#[command(propagate_version = true)]
pub struct Cli {
    /// Output format
    #[arg(long, global = true, default_value = "human", value_name = "FORMAT")]
    pub output: OutputFormat,

    /// Config file path
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<String>,

    /// Network to connect to
    #[arg(long, global = true, default_value = "testnet")]
    pub network: String,

    /// Override active identity
    #[arg(long, global = true)]
    pub identity: Option<String>,

    /// Verbose output (debug logging)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    JsonCompact,
    Ndjson,
}

/// Global flags shared across all commands.
/// Extracted from Cli for cleaner function signatures.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GlobalArgs {
    pub output: OutputFormat,
    pub config: Option<String>,
    pub network: String,
    pub identity: Option<String>,
    pub verbose: bool,
}

impl GlobalArgs {
    pub fn from_cli(cli: &Cli) -> Self {
        Self {
            output: cli.output,
            config: cli.config.clone(),
            network: cli.network.clone(),
            identity: cli.identity.clone(),
            verbose: cli.verbose,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage agent identity (alias: i)
    #[command(alias = "i")]
    Identity {
        #[command(subcommand)]
        command: IdentityCommands,
    },
    /// Manage CLI configuration (alias: cfg)
    #[command(alias = "cfg")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Interactive first-run setup wizard (alias: ini)
    #[command(alias = "ini")]
    Init {
        /// Accept all defaults (non-interactive)
        #[arg(short, long)]
        yes: bool,
    },
    /// Manage P2P mesh network (alias: m)
    #[command(alias = "m")]
    Mesh {
        #[command(subcommand)]
        command: MeshCommands,
    },
    /// Manage social feed (alias: f)
    #[command(alias = "f")]
    Feed {
        #[command(subcommand)]
        command: FeedCommands,
    },
    /// Manage models (alias: mo)
    #[command(alias = "mo")]
    Model {
        #[command(subcommand)]
        command: ModelCommands,
    },
    /// Manage distributed training (alias: t)
    #[command(alias = "t")]
    Train {
        #[command(subcommand)]
        command: TrainCommands,
    },
    /// Manage bounties (alias: b)
    #[command(alias = "b")]
    Bounty {
        #[command(subcommand)]
        command: BountyCommands,
    },
    /// Manage tokens (alias: tk)
    #[command(alias = "tk")]
    Token {
        #[command(subcommand)]
        command: TokenCommands,
    },
    /// Security: input sanitization, circuit breakers (alias: sec)
    #[command(alias = "sec")]
    Security {
        #[command(subcommand)]
        command: SecurityCommands,
    },
    /// Manage agent lifecycle states (alias: lc)
    #[command(alias = "lc")]
    Lifecycle {
        #[command(subcommand)]
        command: LifecycleCommands,
    },
    /// Manage reputation (alias: r)
    #[command(alias = "r")]
    Reputation {
        #[command(subcommand)]
        command: ReputationCommands,
    },
    /// Manage inference marketplace (alias: inf)
    #[command(alias = "inf")]
    Inference {
        #[command(subcommand)]
        command: InferenceCommands,
    },
    /// Query knowledge graph (alias: k)
    #[command(alias = "k")]
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommands,
    },
    /// Manage model lineage and royalties (alias: lin)
    #[command(alias = "lin")]
    Lineage {
        #[command(subcommand)]
        command: LineageCommands,
    },
    /// Verify compute and artifacts (alias: v)
    #[command(alias = "v")]
    Verify {
        #[command(subcommand)]
        command: VerifyCommands,
    },
    /// Discover agents and capabilities (alias: ds)
    #[command(alias = "ds")]
    Discover {
        #[command(subcommand)]
        command: DiscoverCommands,
    },
    /// Select and configure TurboQuant compression (alias: tq)
    #[command(alias = "tq")]
    Turboquant {
        #[command(subcommand)]
        command: TurboquantCommands,
    },
    /// Real-time dashboard (alias: d)
    #[command(alias = "d")]
    Dashboard,
    /// Start web dashboard server (alias: s)
    #[command(alias = "s")]
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,
    },
    /// Show version info
    Version,
}

#[derive(Subcommand, Debug)]
pub enum IdentityCommands {
    /// Create a new agent identity
    Create {
        /// Agent name
        #[arg(short, long)]
        name: String,
        /// DID method (key or neunode)
        #[arg(long, default_value = "key")]
        method: String,
        /// Output directory for keys
        #[arg(long)]
        output_dir: Option<String>,
    },
    /// Show current identity details
    Show {
        /// Specific identity to show (defaults to active)
        #[arg(long)]
        did: Option<String>,
    },
    /// List all identities
    List,
    /// Export identity to file
    Export {
        /// Identity DID to export
        #[arg(long)]
        did: Option<String>,
        /// Output file path
        #[arg(short, long)]
        file: String,
    },
    /// Register active identity on-chain
    RegisterOnChain,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Set a config value
    Set { key: String, value: String },
    /// Get a config value
    Get { key: String },
    /// List all config values
    List,
    /// Show config file path
    Path,
}

#[derive(Subcommand, Debug)]
pub enum MeshCommands {
    /// Start P2P networking
    Start {
        /// Bootstrap peer multiaddrs
        #[arg(long)]
        bootstrap: Vec<String>,
        /// Listen address
        #[arg(long, default_value = "/ip4/0.0.0.0/tcp/41000")]
        listen: String,
    },
    /// Show mesh status
    Status,
    /// List connected peers
    Peers {
        /// Show verbose peer info
        #[arg(long)]
        verbose: bool,
    },
    /// Connect to a specific peer
    Connect {
        /// Peer multiaddr
        addr: String,
    },
    /// Disconnect from a peer
    Disconnect {
        /// Peer ID
        peer_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum FeedCommands {
    /// Post to the feed
    Post {
        /// Event kind (e.g. 1000 for bounty, 2000 for training)
        #[arg(short, long)]
        kind: u32,
        /// Content (JSON string)
        #[arg(short, long)]
        content: String,
        /// Tags (key=value pairs)
        #[arg(short, long)]
        tags: Vec<String>,
    },
    /// List feed events
    List {
        /// Filter by kind
        #[arg(long)]
        kind: Option<u32>,
        /// Filter by author DID
        #[arg(long)]
        author: Option<String>,
        /// Maximum events to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Subscribe to feed events (streaming)
    Subscribe {
        /// Filter by kind
        #[arg(long)]
        kind: Option<u32>,
    },
    /// Show a specific event
    Show {
        /// Event ID (CID)
        event_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ModelCommands {
    /// List available models
    List {
        /// Filter by provider
        #[arg(long)]
        provider: Option<String>,
    },
    /// Show model details
    Show {
        /// Model ID
        model_id: String,
    },
    /// Push a model to the network
    Push {
        /// Model path (local)
        #[arg(short, long)]
        path: String,
        /// Model name
        #[arg(long)]
        name: String,
    },
    /// Remove a model
    Rm {
        /// Model ID
        model_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum TrainCommands {
    /// Start a training job
    Start {
        /// Model to train
        #[arg(long)]
        model: String,
        /// Dataset CID
        #[arg(long)]
        dataset: String,
        /// Training config (JSON)
        #[arg(long)]
        config: Option<String>,
    },
    /// Show training job status
    Status {
        /// Job ID
        #[arg(long)]
        job_id: Option<String>,
    },
    /// Stop a training job
    Stop {
        /// Job ID
        job_id: String,
    },
    /// List training jobs
    List,
    /// Register as a training compute provider
    WorkerRegister {
        /// Number of GPUs
        #[arg(long)]
        gpu_count: u32,
        /// Total GPU memory in GB
        #[arg(long)]
        gpu_memory: f64,
        /// Max model parameters
        #[arg(long)]
        max_params: u64,
        /// Supports bfloat16
        #[arg(long, default_value_t = false)]
        bf16: bool,
    },
    /// List registered training workers
    WorkerList {
        /// Filter by minimum GPU count
        #[arg(long)]
        min_gpu: Option<u32>,
        /// Filter by minimum GPU memory (GB)
        #[arg(long)]
        min_memory: Option<f64>,
    },
    /// Show training coordinator status for a job
    CoordinatorStatus {
        /// Training job ID
        #[arg(long)]
        job_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum BountyCommands {
    /// Create a new bounty
    Create {
        /// Bounty title
        #[arg(short, long)]
        title: String,
        /// Detailed description
        #[arg(short, long)]
        description: String,
        /// Reward amount
        #[arg(short, long)]
        reward: u64,
        /// Token type for reward
        #[arg(short = 'T', long, default_value = "compute")]
        token: String,
        /// Claim deadline in hours
        #[arg(long, default_value = "72")]
        claim_deadline: u64,
        /// Work deadline in hours
        #[arg(long, default_value = "168")]
        work_deadline: u64,
    },
    /// Claim an open bounty
    Claim {
        /// Bounty ID
        #[arg(short, long)]
        id: String,
        /// Stake amount to lock
        #[arg(short, long)]
        stake: u64,
    },
    /// Submit work for a claimed bounty
    Submit {
        /// Bounty ID
        #[arg(short, long)]
        id: String,
        /// Artifact CID or URL
        #[arg(short, long)]
        artifact: String,
        /// Evidence JSON
        #[arg(long)]
        evidence: Option<String>,
    },
    /// Review a bounty submission
    Review {
        /// Bounty ID
        #[arg(short, long)]
        id: String,
        /// Score (0-100)
        #[arg(short, long)]
        score: u8,
        /// Feedback text
        #[arg(short, long)]
        feedback: String,
    },
    /// List bounties
    List {
        /// Filter by state
        #[arg(long)]
        state: Option<String>,
        /// Filter by creator DID
        #[arg(long)]
        creator: Option<String>,
        /// Max results
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Show bounty details
    Show {
        /// Bounty ID
        id: String,
    },
    /// Cancel a bounty
    Cancel {
        /// Bounty ID
        #[arg(short, long)]
        id: String,
        /// Cancellation reason
        #[arg(short, long)]
        reason: Option<String>,
    },
    /// Pay out an accepted bounty
    Pay {
        /// Bounty ID
        #[arg(short, long)]
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum TokenCommands {
    /// Show token balance
    Balance {
        /// Filter by token type
        #[arg(long)]
        token: Option<String>,
    },
    /// Transfer tokens
    Transfer {
        /// Recipient DID
        #[arg(long)]
        to: String,
        /// Amount to transfer
        #[arg(long)]
        amount: u64,
        /// Token type
        #[arg(long, default_value = "compute")]
        token: String,
    },
    /// Stake tokens
    Stake {
        /// Amount to stake
        #[arg(long)]
        amount: u64,
        /// Token type
        #[arg(long, default_value = "compute")]
        token: String,
    },
    /// Unstake tokens
    Unstake {
        /// Amount to unstake
        #[arg(long)]
        amount: u64,
    },
    /// Show staking status
    StakeStatus,
    /// Show decay info and rates
    DecayInfo,
    /// Grant seed tokens to a new agent (staked only)
    Seed {
        /// Target agent DID (defaults to active identity)
        #[arg(long)]
        agent: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum SecurityCommands {
    /// Sanitize untrusted input (feed data, bounty descriptions)
    Sanitize {
        /// Input text to sanitize
        #[arg(short, long)]
        input: String,
        /// Input type (feed | bounty | knowledge | chat)
        #[arg(long, default_value = "feed")]
        kind: String,
    },
    /// Show circuit breaker status
    BreakerStatus,
    /// Manually trip a circuit breaker
    BreakerTrip {
        /// Breaker name (token_volume | reputation | bounty_drain)
        name: String,
    },
    /// Reset a tripped circuit breaker
    BreakerReset {
        /// Breaker name
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum LifecycleCommands {
    /// Show current agent lifecycle state
    Status,
    /// Activate a CREATED agent (stake + register)
    Activate,
    /// Hibernate the active agent (intentional pause)
    Hibernate,
    /// Reactivate from hibernation
    Reactivate,
    /// List all known agent states
    List,
    /// Check for idle/zombie agents and process transitions
    Reap,
}

#[derive(Subcommand, Debug)]
pub enum ReputationCommands {
    /// Show reputation for an agent
    Show {
        /// Agent DID (defaults to active identity)
        #[arg(long)]
        agent: Option<String>,
    },
    /// Attest to another agent
    Attest {
        /// Target agent DID
        #[arg(long)]
        to: String,
        /// Score (0-100)
        #[arg(long)]
        score: u8,
        /// Comment
        #[arg(long)]
        comment: Option<String>,
    },
    /// Show reputation leaderboard
    Leaderboard {
        /// Max results
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Show reputation factor breakdown
    Factors {
        /// Agent DID (defaults to active identity)
        #[arg(long)]
        agent: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum InferenceCommands {
    /// Request inference from a model
    Request {
        /// Model ID
        #[arg(long)]
        model: String,
        /// Prompt text
        #[arg(long)]
        prompt: String,
        /// Max tokens to generate
        #[arg(long, default_value = "512")]
        max_tokens: u32,
        /// Temperature (0.0-2.0)
        #[arg(long, default_value = "0.7")]
        temperature: f64,
    },
    /// List available models
    ListModels {
        /// Filter by provider
        #[arg(long)]
        provider: Option<String>,
    },
    /// List inference providers
    Providers {
        /// Filter by model
        #[arg(long)]
        model: Option<String>,
    },
    /// Show routing for a model request
    Route {
        /// Model ID
        #[arg(long)]
        model: String,
        /// Routing strategy
        #[arg(long, default_value = "cheapest")]
        strategy: String,
    },
    /// Show pricing estimate
    Pricing {
        /// Model ID
        #[arg(long)]
        model: String,
        /// Estimated input tokens
        #[arg(long, default_value = "1000")]
        input_tokens: u32,
        /// Estimated output tokens
        #[arg(long, default_value = "500")]
        output_tokens: u32,
    },
}

#[derive(Subcommand, Debug)]
pub enum KnowledgeCommands {
    /// Query the knowledge graph
    Query {
        /// Filter by subject URI
        #[arg(long)]
        subject: Option<String>,
        /// Filter by predicate URI
        #[arg(long)]
        predicate: Option<String>,
        /// Filter by object URI
        #[arg(long)]
        object: Option<String>,
        /// Filter by graph URI
        #[arg(long)]
        graph: Option<String>,
        /// Max results
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Register an agent in the knowledge graph
    RegisterAgent {
        /// Agent DID
        #[arg(long)]
        did: String,
        /// Comma-separated capabilities
        #[arg(long)]
        capabilities: String,
    },
    /// Register a model in the knowledge graph
    RegisterModel {
        /// Owner DID
        #[arg(long)]
        did: String,
        /// Model CID
        #[arg(long)]
        cid: String,
        /// Parent model CID (optional)
        #[arg(long)]
        parent: Option<String>,
    },
    /// Register a bounty in the knowledge graph
    RegisterBounty {
        /// Bounty ID
        #[arg(long)]
        id: String,
        /// Comma-separated required capabilities
        #[arg(long)]
        capabilities: String,
    },
    /// Record agent joining a training job
    JoinJob {
        /// Agent DID
        #[arg(long)]
        did: String,
        /// Job ID
        #[arg(long)]
        job_id: String,
    },
    /// List ontology classes
    ListClasses,
    /// List ontology predicates
    ListPredicates,
}

#[derive(Subcommand, Debug)]
pub enum LineageCommands {
    /// Register a model in the lineage DAG
    Register {
        /// Model CID (sha256:hex)
        #[arg(long)]
        cid: String,
        /// Comma-separated parent CIDs
        #[arg(long)]
        parents: Option<String>,
        /// Contribution type (pre_training|fine_tune|merge|rl|data|compute)
        #[arg(long, default_value = "fine_tune")]
        contribution_type: String,
        /// LoRA rank (for fine_tune)
        #[arg(long)]
        lora_rank: Option<u32>,
        /// LoRA alpha (for fine_tune)
        #[arg(long)]
        lora_alpha: Option<f64>,
    },
    /// Show model details
    Show {
        /// Model CID
        cid: String,
    },
    /// Show direct parents of a model
    Parents {
        /// Model CID
        cid: String,
    },
    /// Show direct children of a model
    Children {
        /// Model CID
        cid: String,
    },
    /// Show all ancestors of a model
    Ancestors {
        /// Model CID
        cid: String,
    },
    /// Show lineage depth (longest path to root)
    Depth {
        /// Model CID
        cid: String,
    },
    /// Compute royalty distribution for a model
    Royalties {
        /// Serving model CID
        cid: String,
        /// Total amount in basis points
        #[arg(long)]
        amount: u32,
    },
    /// Compute content hash of a file
    Hash {
        /// File path
        #[arg(short, long)]
        file: String,
    },
    /// Verify model signature
    Verify {
        /// Model CID
        cid: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum VerifyCommands {
    /// Run gauntlet adversarial test
    Gauntlet {
        /// Test name
        #[arg(long)]
        test_name: String,
        /// Input artifact hash
        #[arg(long)]
        input_hash: String,
        /// Expected output hash
        #[arg(long)]
        expected_hash: String,
    },
    /// Run spot-check verification
    SpotCheck {
        /// Path to original output file
        #[arg(long)]
        original: String,
        /// Path to recomputed output file
        #[arg(long)]
        recomputed: String,
    },
    /// Compare two RepOps execution results
    Repops {
        /// Comma-separated hashes from execution A
        #[arg(long)]
        hashes_a: String,
        /// Comma-separated hashes from execution B
        #[arg(long)]
        hashes_b: String,
    },
    /// Run bisection to find disagreement point
    Bisection {
        /// Comma-separated claimant hashes
        #[arg(long)]
        claimant: String,
        /// Comma-separated challenger hashes
        #[arg(long)]
        challenger: String,
    },
    /// Verify a TEE attestation quote
    Tee {
        /// Expected measurement hash
        #[arg(long)]
        measurement: String,
        /// Nonce in hex
        #[arg(long)]
        nonce: String,
        /// TEE type (intel_tdx|amd_sev|nvidia_ccn|apple_se)
        #[arg(long, default_value = "intel_tdx")]
        tee_type: String,
    },
    /// Show available verification layers
    Status,
}

#[derive(Subcommand, Debug)]
pub enum TurboquantCommands {
    /// Select the compression strategy for a workload profile
    Compress {
        /// Workload profile: gradient, kv_cache, or custom
        #[arg(long)]
        profile: String,
        /// Vector dimension
        #[arg(long)]
        dimension: usize,
        /// Worker count for gradient profiles
        #[arg(long)]
        workers: Option<usize>,
        /// Available bandwidth in Mbps for gradient profiles
        #[arg(long)]
        bandwidth_mbps: Option<f64>,
        /// Desired bits per element for KV-cache profiles
        #[arg(long)]
        target_bits: Option<f32>,
        /// Exact bit width for custom profiles
        #[arg(long)]
        bits: Option<u8>,
    },
    /// Generate a deterministic scalar quantization codebook
    GenerateCodebook {
        /// Bits per quantized value
        #[arg(long)]
        bits: u32,
        /// Vector dimension
        #[arg(long)]
        dimension: usize,
        /// Maximum Lloyd-Max iterations
        #[arg(long)]
        max_iterations: Option<u32>,
        /// MSE convergence threshold
        #[arg(long)]
        convergence_threshold: Option<f64>,
        /// Distribution sample count
        #[arg(long)]
        num_samples: Option<usize>,
    },
}

#[derive(Subcommand, Debug)]
pub enum DiscoverCommands {
    /// Search for agents matching capabilities
    Search {
        /// Comma-separated required capabilities
        #[arg(short, long)]
        capabilities: String,
        /// Minimum reputation score (0.0-5.0)
        #[arg(long, default_value = "0.0")]
        min_reputation: f64,
        /// Maximum cost per unit
        #[arg(long)]
        max_cost: Option<f64>,
        /// Only online agents
        #[arg(long, default_value_t = false)]
        online_only: bool,
        /// Max results
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Find agents with complementary capabilities to yours
    Complement {
        /// Your comma-separated capabilities
        #[arg(short, long)]
        capabilities: String,
        /// Max results
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Find capability gaps (capabilities with no providers)
    Gaps,
    /// Score a specific agent for a task
    Score {
        /// Agent DID to score
        #[arg(long)]
        agent: String,
        /// Comma-separated required capabilities
        #[arg(short, long)]
        capabilities: String,
    },
    /// Show current discovery weights
    Weights,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_version_command() {
        let cli = Cli::try_parse_from(["agnetd", "version"]).expect("parse version");
        assert!(matches!(cli.command, Commands::Version));
    }

    #[test]
    fn parse_identity_create() {
        let cli = Cli::try_parse_from(["agnetd", "identity", "create", "--name", "test-agent"])
            .expect("parse identity create");
        match cli.command {
            Commands::Identity { command } => match command {
                IdentityCommands::Create { name, method, .. } => {
                    assert_eq!(name, "test-agent");
                    assert_eq!(method, "key");
                }
                _ => panic!("expected Create"),
            },
            _ => panic!("expected Identity"),
        }
    }

    #[test]
    fn parse_identity_create_with_method() {
        let cli = Cli::try_parse_from([
            "agnetd", "identity", "create", "--name", "agent", "--method", "neunode",
        ])
        .expect("parse");
        match cli.command {
            Commands::Identity { command } => match command {
                IdentityCommands::Create { name, method, .. } => {
                    assert_eq!(name, "agent");
                    assert_eq!(method, "neunode");
                }
                _ => panic!("expected Create"),
            },
            _ => panic!("expected Identity"),
        }
    }

    #[test]
    fn parse_identity_alias() {
        let cli = Cli::try_parse_from(["agnetd", "i", "list"]).expect("parse alias");
        assert!(matches!(cli.command, Commands::Identity { .. }));
    }

    #[test]
    fn parse_config_set() {
        let cli = Cli::try_parse_from(["agnetd", "config", "set", "agent.name", "new-name"])
            .expect("parse");
        match cli.command {
            Commands::Config { command } => match command {
                ConfigCommands::Set { key, value } => {
                    assert_eq!(key, "agent.name");
                    assert_eq!(value, "new-name");
                }
                _ => panic!("expected Set"),
            },
            _ => panic!("expected Config"),
        }
    }

    #[test]
    fn parse_config_get() {
        let cli = Cli::try_parse_from(["agnetd", "config", "get", "agent.name"]).expect("parse");
        match cli.command {
            Commands::Config { command } => match command {
                ConfigCommands::Get { key } => {
                    assert_eq!(key, "agent.name");
                }
                _ => panic!("expected Get"),
            },
            _ => panic!("expected Config"),
        }
    }

    #[test]
    fn parse_config_alias() {
        let cli = Cli::try_parse_from(["agnetd", "cfg", "list"]).expect("parse alias");
        assert!(matches!(cli.command, Commands::Config { .. }));
    }

    #[test]
    fn parse_mesh_start() {
        let cli = Cli::try_parse_from([
            "agnetd",
            "mesh",
            "start",
            "--bootstrap",
            "/ip4/1.2.3.4/tcp/4001/p2p/QmABC",
            "--listen",
            "/ip4/0.0.0.0/tcp/41000",
        ])
        .expect("parse");
        match cli.command {
            Commands::Mesh { command } => match command {
                MeshCommands::Start { bootstrap, listen } => {
                    assert_eq!(bootstrap.len(), 1);
                    assert_eq!(listen, "/ip4/0.0.0.0/tcp/41000");
                }
                _ => panic!("expected Start"),
            },
            _ => panic!("expected Mesh"),
        }
    }

    #[test]
    fn parse_mesh_alias() {
        let cli = Cli::try_parse_from(["agnetd", "m", "status"]).expect("parse alias");
        assert!(matches!(cli.command, Commands::Mesh { .. }));
    }

    #[test]
    fn parse_feed_post() {
        let cli = Cli::try_parse_from([
            "agnetd",
            "feed",
            "post",
            "--kind",
            "1000",
            "--content",
            r#"{"title":"test"}"#,
            "--tags",
            "env=test",
        ])
        .expect("parse");
        match cli.command {
            Commands::Feed { command } => match command {
                FeedCommands::Post { kind, content, tags } => {
                    assert_eq!(kind, 1000);
                    assert_eq!(content, r#"{"title":"test"}"#);
                    assert_eq!(tags, vec!["env=test"]);
                }
                _ => panic!("expected Post"),
            },
            _ => panic!("expected Feed"),
        }
    }

    #[test]
    fn parse_feed_list_with_filters() {
        let cli =
            Cli::try_parse_from(["agnetd", "feed", "list", "--kind", "2000", "--limit", "50"])
                .expect("parse");
        match cli.command {
            Commands::Feed { command } => match command {
                FeedCommands::List { kind, limit, .. } => {
                    assert_eq!(kind, Some(2000));
                    assert_eq!(limit, 50);
                }
                _ => panic!("expected List"),
            },
            _ => panic!("expected Feed"),
        }
    }

    #[test]
    fn parse_feed_alias() {
        let cli = Cli::try_parse_from(["agnetd", "f", "list"]).expect("parse alias");
        assert!(matches!(cli.command, Commands::Feed { .. }));
    }

    #[test]
    fn parse_model_list() {
        let cli = Cli::try_parse_from(["agnetd", "model", "list", "--provider", "neunode"])
            .expect("parse");
        match cli.command {
            Commands::Model { command } => match command {
                ModelCommands::List { provider } => {
                    assert_eq!(provider.as_deref(), Some("neunode"));
                }
                _ => panic!("expected List"),
            },
            _ => panic!("expected Model"),
        }
    }

    #[test]
    fn parse_model_alias() {
        let cli = Cli::try_parse_from(["agnetd", "mo", "list"]).expect("parse alias");
        assert!(matches!(cli.command, Commands::Model { .. }));
    }

    #[test]
    fn parse_train_start() {
        let cli = Cli::try_parse_from([
            "agnetd",
            "train",
            "start",
            "--model",
            "llama-3b",
            "--dataset",
            "bafkrei123",
        ])
        .expect("parse");
        match cli.command {
            Commands::Train { command } => match command {
                TrainCommands::Start { model, dataset, .. } => {
                    assert_eq!(model, "llama-3b");
                    assert_eq!(dataset, "bafkrei123");
                }
                _ => panic!("expected Start"),
            },
            _ => panic!("expected Train"),
        }
    }

    #[test]
    fn parse_train_alias() {
        let cli = Cli::try_parse_from(["agnetd", "t", "list"]).expect("parse alias");
        assert!(matches!(cli.command, Commands::Train { .. }));
    }

    #[test]
    fn parse_turboquant_compress() {
        let cli = Cli::try_parse_from([
            "agnetd",
            "turboquant",
            "compress",
            "--profile",
            "kv_cache",
            "--dimension",
            "4096",
            "--target-bits",
            "3.5",
        ])
        .expect("parse");
        match cli.command {
            Commands::Turboquant { command } => match command {
                TurboquantCommands::Compress { profile, dimension, target_bits, .. } => {
                    assert_eq!(profile, "kv_cache");
                    assert_eq!(dimension, 4096);
                    assert_eq!(target_bits, Some(3.5));
                }
                _ => panic!("expected Compress"),
            },
            _ => panic!("expected Turboquant"),
        }
    }

    #[test]
    fn parse_turboquant_alias() {
        let cli = Cli::try_parse_from([
            "agnetd",
            "tq",
            "generate-codebook",
            "--bits",
            "4",
            "--dimension",
            "256",
        ])
        .expect("parse alias");
        assert!(matches!(cli.command, Commands::Turboquant { .. }));
    }

    #[test]
    fn parse_global_flags() {
        let cli = Cli::try_parse_from([
            "agnetd",
            "--output",
            "json",
            "--network",
            "mainnet",
            "--verbose",
            "version",
        ])
        .expect("parse globals");
        assert!(matches!(cli.output, OutputFormat::Json));
        assert_eq!(cli.network, "mainnet");
        assert!(cli.verbose);
    }

    #[test]
    fn parse_output_format_json_compact() {
        let cli =
            Cli::try_parse_from(["agnetd", "--output", "json-compact", "version"]).expect("parse");
        assert!(matches!(cli.output, OutputFormat::JsonCompact));
    }

    #[test]
    fn parse_output_format_ndjson() {
        let cli = Cli::try_parse_from(["agnetd", "--output", "ndjson", "version"]).expect("parse");
        assert!(matches!(cli.output, OutputFormat::Ndjson));
    }

    #[test]
    fn parse_default_output_is_human() {
        let cli = Cli::try_parse_from(["agnetd", "version"]).expect("parse");
        assert!(matches!(cli.output, OutputFormat::Human));
    }

    #[test]
    fn parse_default_network_is_testnet() {
        let cli = Cli::try_parse_from(["agnetd", "version"]).expect("parse");
        assert_eq!(cli.network, "testnet");
    }

    #[test]
    fn parse_config_path() {
        let cli = Cli::try_parse_from(["agnetd", "--config", "/tmp/test.toml", "version"])
            .expect("parse");
        assert_eq!(cli.config.as_deref(), Some("/tmp/test.toml"));
    }

    #[test]
    fn parse_identity_override() {
        let cli =
            Cli::try_parse_from(["agnetd", "--identity", "did:neunode:0xabc", "identity", "list"])
                .expect("parse");
        assert_eq!(cli.identity.as_deref(), Some("did:neunode:0xabc"));
    }

    #[test]
    fn parse_init_default() {
        let cli = Cli::try_parse_from(["agnetd", "init"]).expect("parse init");
        match cli.command {
            Commands::Init { yes } => assert!(!yes),
            _ => panic!("expected Init"),
        }
    }

    #[test]
    fn parse_init_yes_flag() {
        let cli = Cli::try_parse_from(["agnetd", "init", "--yes"]).expect("parse init --yes");
        match cli.command {
            Commands::Init { yes } => assert!(yes),
            _ => panic!("expected Init"),
        }
    }

    #[test]
    fn parse_init_alias() {
        let cli = Cli::try_parse_from(["agnetd", "ini", "--yes"]).expect("parse alias");
        assert!(matches!(cli.command, Commands::Init { yes: true }));
    }
}
