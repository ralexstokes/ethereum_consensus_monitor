use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ConsensusChainConfig {
    pub seconds_per_slot: u64,
    pub slots_per_epoch: u64,
    pub genesis_time: u64,
}

#[derive(Deserialize, Debug)]
pub struct WeakSubjectivityConfig {
    pub provider_endpoint: String,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub network: String,
    pub etherscan_api_key: String,
    pub consensus_chain: ConsensusChainConfig,
    pub endpoints: Vec<String>,
    pub weak_subjectivity: WeakSubjectivityConfig,
}
