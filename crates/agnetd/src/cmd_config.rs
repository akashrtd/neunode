use anyhow::Result;

use crate::cli::{Cli, ConfigCommands};
use crate::output::OutputWriter;
use crate::state::AppState;

pub fn execute(cmd: &ConfigCommands, cli: &Cli, state: &mut AppState) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        ConfigCommands::Set { key, value } => config_set(key, value, state, &writer),
        ConfigCommands::Get { key } => config_get(key, state, &writer),
        ConfigCommands::List => config_list(state, &writer),
        ConfigCommands::Path => config_path(state, &writer),
    }
}

fn config_set(key: &str, value: &str, state: &mut AppState, writer: &OutputWriter) -> Result<()> {
    state.config.set(key, value)?;
    state.save_config()?;
    writer.write_status(&format!("Set {key} = {value}"));
    Ok(())
}

fn config_get(key: &str, state: &AppState, writer: &OutputWriter) -> Result<()> {
    match state.config.get(key) {
        Some(value) => writer.write_value(key, &value),
        None => {
            writer.write_error(&format!("unknown config key: {key}"));
            anyhow::bail!("unknown config key: {key}");
        }
    }
    Ok(())
}

fn config_list(state: &AppState, writer: &OutputWriter) -> Result<()> {
    let all = state.config.list_all();
    let pairs: Vec<(&str, &str)> = all.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    writer.write_key_value_pairs(&pairs);
    Ok(())
}

fn config_path(state: &AppState, writer: &OutputWriter) -> Result<()> {
    writer.write_value("Config path", &state.config.config_path.to_string_lossy());
    Ok(())
}
