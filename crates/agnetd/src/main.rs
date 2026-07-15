use std::process::ExitCode;

use clap::Parser;

mod api;
mod bounty_service;
mod cli;
mod cmd_bounty;
mod cmd_config;
mod cmd_dashboard;
mod cmd_discover;
mod cmd_feed;
mod cmd_identity;
mod cmd_inference;
mod cmd_init;
mod cmd_knowledge;
mod cmd_lifecycle;
mod cmd_lineage;
mod cmd_mesh;
mod cmd_model;
mod cmd_reputation;
mod cmd_security;
mod cmd_serve;
mod cmd_token;
mod cmd_train;
mod cmd_turboquant;
mod cmd_verify;
mod config;
mod error;
mod feed_wire;
mod mesh_handle;
mod output;
mod state;
mod turboquant_service;
mod util;

#[cfg(test)]
mod testutil;

use cli::Commands;

fn main() -> ExitCode {
    let cli_args = match cli::Cli::try_parse() {
        Ok(args) => args,
        Err(e) => {
            e.print().unwrap();
            return ExitCode::from(2);
        }
    };

    init_logging(cli_args.verbose);

    let global_args = cli::GlobalArgs::from_cli(&cli_args);

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("fatal: failed to create tokio runtime: {e}");
            return ExitCode::from(1);
        }
    };

    let mut app_state = match state::AppState::init_from_globals(&global_args) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("fatal: initialization failed: {e:#}");
            return ExitCode::from(1);
        }
    };

    let result = match &cli_args.command {
        Commands::Identity { command } => {
            cmd_identity::execute(command, &global_args, &mut app_state)
        }
        Commands::Config { command } => cmd_config::execute(command, &global_args, &mut app_state),
        Commands::Init { yes } => cmd_init::execute(*yes, &global_args, &mut app_state),
        Commands::Mesh { command } => {
            rt.block_on(cmd_mesh::execute(command, &global_args, &mut app_state))
        }
        Commands::Feed { command } => cmd_feed::execute(command, &global_args, &mut app_state),
        Commands::Model { command } => cmd_model::execute(command, &global_args, &mut app_state),
        Commands::Train { command } => cmd_train::execute(command, &global_args, &mut app_state),
        Commands::Bounty { command } => cmd_bounty::execute(command, &global_args, &mut app_state),
        Commands::Token { command } => cmd_token::execute(command, &global_args, &mut app_state),
        Commands::Security { command } => {
            cmd_security::execute(command, &global_args, &mut app_state)
        }
        Commands::Lifecycle { command } => {
            cmd_lifecycle::execute(command, &global_args, &mut app_state)
        }
        Commands::Reputation { command } => {
            cmd_reputation::execute(command, &global_args, &mut app_state)
        }
        Commands::Inference { command } => {
            cmd_inference::execute(command, &global_args, &mut app_state)
        }
        Commands::Knowledge { command } => {
            cmd_knowledge::execute(command, &global_args, &mut app_state)
        }
        Commands::Lineage { command } => {
            cmd_lineage::execute(command, &global_args, &mut app_state)
        }
        Commands::Verify { command } => cmd_verify::execute(command, &global_args, &mut app_state),
        Commands::Discover { command } => {
            cmd_discover::execute(command, &global_args, &mut app_state)
        }
        Commands::Turboquant { command } => cmd_turboquant::execute(command, &global_args),
        Commands::Dashboard => rt.block_on(cmd_dashboard::execute(&global_args, &mut app_state)),
        Commands::Serve { port } => {
            rt.block_on(cmd_serve::execute(*port, &global_args, &mut app_state))
        }
        Commands::Version => {
            println!("agnetd v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => match e.downcast::<error::CliError>() {
            Ok(cli_err) => {
                eprintln!("{}", console::style(format!("✗ {}", cli_err.message())).red().bold());
                cli_err.exit_code()
            }
            Err(other_err) => {
                eprintln!("{}", console::style(format!("✗ {other_err:#}")).red().bold());
                ExitCode::from(1)
            }
        },
    }
}

fn init_logging(verbose: bool) {
    let level = if verbose { "debug" } else { "info" };
    let env_filter = tracing_subscriber::EnvFilter::try_new(level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    tracing_subscriber::fmt().with_env_filter(env_filter).with_target(false).init();
}
