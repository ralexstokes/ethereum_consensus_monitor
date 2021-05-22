use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct Spec {
    pub network_name: String,
    pub seconds_per_slot: u64,
    pub genesis_time: u64,
    pub slots_per_epoch: u64,
}

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

impl Config {
    pub fn get_spec(&self) -> Spec {
        Spec {
            network_name: self.network.clone(),
            seconds_per_slot: self.consensus_chain.seconds_per_slot,
            genesis_time: self.consensus_chain.genesis_time,
            slots_per_epoch: self.consensus_chain.slots_per_epoch,
        }
    }
}
