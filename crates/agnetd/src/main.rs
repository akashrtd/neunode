use anyhow::Result;
use clap::Parser;

mod cli;
mod cmd_bounty;
mod cmd_config;
mod cmd_feed;
mod cmd_identity;
mod cmd_inference;
mod cmd_mesh;
mod cmd_model;
mod cmd_reputation;
mod cmd_token;
mod cmd_train;
mod config;
mod output;
mod state;

use cli::Commands;

#[tokio::main]
async fn main() -> Result<()> {
    let cli_args = cli::Cli::parse();
    init_logging(cli_args.verbose);
    let mut app_state = state::AppState::init(&cli_args)?;

    match &cli_args.command {
        Commands::Identity { command } => cmd_identity::execute(command, &cli_args, &mut app_state),
        Commands::Config { command } => cmd_config::execute(command, &cli_args, &mut app_state),
        Commands::Mesh { command } => cmd_mesh::execute(command, &cli_args, &mut app_state).await,
        Commands::Feed { command } => cmd_feed::execute(command, &cli_args, &mut app_state),
        Commands::Model { command } => cmd_model::execute(command, &cli_args, &mut app_state),
        Commands::Train { command } => cmd_train::execute(command, &cli_args, &mut app_state),
        Commands::Bounty { command } => cmd_bounty::execute(command, &cli_args, &mut app_state),
        Commands::Token { command } => cmd_token::execute(command, &cli_args, &mut app_state),
        Commands::Reputation { command } => {
            cmd_reputation::execute(command, &cli_args, &mut app_state)
        }
        Commands::Inference { command } => {
            cmd_inference::execute(command, &cli_args, &mut app_state)
        }
        Commands::Version => {
            println!("agnetd v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

fn init_logging(verbose: bool) {
    let level = if verbose { "debug" } else { "warn" };
    let env_filter = tracing_subscriber::EnvFilter::try_new(level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    tracing_subscriber::fmt().with_env_filter(env_filter).with_target(false).init();
}
