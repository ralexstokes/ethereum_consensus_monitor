use anyhow::{Context, Result};
use clap::{AppSettings, Clap};
use ethereum_consensus_monitor::Monitor;
use std::fs;
use std::path::PathBuf;

#[derive(Clap)]
#[clap(setting = AppSettings::ColoredHelp)]
struct Options {
    #[clap(long, default_value = "config.toml")]
    config_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let options: Options = Options::parse();

    let config = fs::read_to_string(&options.config_path)
        .with_context(|| format!("failed to read config from {:?}", options.config_path))?;

    let monitor = Monitor::from_config(&config);
    monitor.run().await;

    Ok(())
}
