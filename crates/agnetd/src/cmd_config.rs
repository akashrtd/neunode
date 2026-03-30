use anyhow::Result;

use crate::cli::{Cli, ConfigCommands};
use crate::config::CliConfig;
use crate::output::OutputWriter;

pub fn execute(cmd: &ConfigCommands, cli: &Cli, config: &mut CliConfig) -> Result<()> {
    let writer = OutputWriter::new(cli.output);
    match cmd {
        ConfigCommands::Set { key, value } => config_set(key, value, config, &writer),
        ConfigCommands::Get { key } => config_get(key, config, &writer),
        ConfigCommands::List => config_list(config, &writer),
        ConfigCommands::Path => config_path(config, &writer),
    }
}

fn config_set(key: &str, value: &str, config: &mut CliConfig, writer: &OutputWriter) -> Result<()> {
    config.set(key, value)?;
    config.save()?;
    writer.write_status(&format!("Set {key} = {value}"));
    Ok(())
}

fn config_get(key: &str, config: &CliConfig, writer: &OutputWriter) -> Result<()> {
    match config.get(key) {
        Some(value) => writer.write_value(key, &value),
        None => {
            writer.write_error(&format!("unknown config key: {key}"));
            anyhow::bail!("unknown config key: {key}");
        }
    }
    Ok(())
}

fn config_list(config: &CliConfig, writer: &OutputWriter) -> Result<()> {
    let all = config.list_all();
    let pairs: Vec<(&str, &str)> = all.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    writer.write_key_value_pairs(&pairs);
    Ok(())
}

fn config_path(config: &CliConfig, writer: &OutputWriter) -> Result<()> {
    writer.write_value("Config path", &config.config_path.to_string_lossy());
    Ok(())
}
