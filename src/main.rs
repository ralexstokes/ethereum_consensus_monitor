use clap::{AppSettings, Clap};
use std::fs;
use std::path::PathBuf;

#[derive(Clap)]
#[clap(setting = AppSettings::ColoredHelp)]
struct Options {
    #[clap(long, default_value = "config.toml")]
    config_path: PathBuf,
    #[clap(long)]
    output_dir: Option<PathBuf>,
    #[clap(long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    pretty_env_logger::init();

    let options: Options = Options::parse();

    let config = fs::read_to_string(options.config_path)?;

    let mut monitor = eth_monitor::from_config(&config);
    if let Some(output_dir) = options.output_dir {
        monitor.with_output_dir(output_dir);
    }
    if let Some(port) = options.port {
        monitor.with_port(port);
    }

    monitor.run().await;

    Ok(())
}
