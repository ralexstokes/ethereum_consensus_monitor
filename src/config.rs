use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Clone)]
pub struct Spec {
    pub network_name: String,
    pub seconds_per_slot: u64,
    pub genesis_time: u64,
    pub slots_per_epoch: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    pub name: String,
    pub etherscan_api_key: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MonitorConfig {
    pub output_dir: PathBuf,
    pub port: u16,
    pub endpoints: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConsensusChainConfig {
    pub seconds_per_slot: u64,
    pub slots_per_epoch: u64,
    pub genesis_time: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WeakSubjectivityConfig {
    pub provider_endpoint: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub monitor: MonitorConfig,
    pub network: NetworkConfig,
    pub consensus_chain: ConsensusChainConfig,
    pub weak_subjectivity: WeakSubjectivityConfig,
}
