use crate::chain::Coordinate;
use crate::config::Config;
use crate::fork_choice::ForkChoice;
use crate::node::Node;
use futures::future;
use serde::Serialize;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use warp;
use warp::Filter;

#[derive(Serialize)]
struct NodeResponse {
    id: Option<u64>,
    head: Option<Coordinate>,
}

#[derive(Serialize, Clone)]
struct NetworkConfigResponse {
    network_name: String,
    seconds_per_slot: u64,
    genesis_time: u64,
    slots_per_epoch: u64,
}

impl From<&Config> for NetworkConfigResponse {
    fn from(config: &Config) -> Self {
        Self {
            network_name: config.network.name.clone(),
            seconds_per_slot: config.consensus_chain.seconds_per_slot,
            genesis_time: config.consensus_chain.genesis_time,
            slots_per_epoch: config.consensus_chain.slots_per_epoch,
        }
    }
}

pub struct ApiServer {
    web_dir: PathBuf,
    nodes: Vec<Node>,
    fork_choice_handle: ForkChoice,
    network_config: NetworkConfigResponse,
}

impl ApiServer {
    pub fn new<P>(
        web_dir: P,
        nodes: Vec<Node>,
        fork_choice_handle: ForkChoice,
        config: &Config,
    ) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            web_dir: web_dir.as_ref().to_path_buf(),
            nodes,
            fork_choice_handle,
            network_config: config.into(),
        }
    }

    pub async fn run(&self, addr: impl Into<SocketAddr>) {
        let network_config = Arc::new(self.network_config.clone());
        let network_config = warp::get()
            .and(warp::path("network-config"))
            .and(warp::path::end())
            .and(warp::any().map(move || network_config.clone()))
            .and_then(serve_network_config);

        let nodes_handle = self.nodes.clone();
        let nodes = warp::get()
            .and(warp::path("nodes"))
            .and(warp::path::end())
            .and(warp::any().map(move || nodes_handle.clone()))
            .and_then(get_nodes);

        let fork_choice_handle = self.fork_choice_handle.clone();
        let fork_choice = warp::get()
            .and(warp::path("fork-choice"))
            .and(warp::path::end())
            .and(warp::any().map(move || fork_choice_handle.clone()))
            .and_then(get_fork_choice);

        let participation = warp::get()
            .and(warp::path("participation"))
            .and(warp::path::end())
            .and_then(serve_participation_data);

        let deposit_contract = warp::get()
            .and(warp::path("deposit-contract"))
            .and(warp::path::end())
            .and_then(serve_deposit_contract_data);

        let weak_subjectivity = warp::get()
            .and(warp::path("weak-subjectivity"))
            .and(warp::path::end())
            .and_then(serve_weak_subjectivity_data);

        let html_dir = self.web_dir.clone();
        let app = warp::get().and(warp::any()).and(warp::fs::dir(html_dir));

        let routes = network_config
            .or(nodes)
            .or(fork_choice)
            .or(participation)
            .or(deposit_contract)
            .or(weak_subjectivity)
            .or(app);

        warp::serve(routes).run(addr).await
    }
}

async fn get_nodes(nodes: Vec<Node>) -> Result<impl warp::Reply, warp::Rejection> {
    let reads = nodes.iter().map(|node| async move {
        let node = node.read().await;
        NodeResponse {
            id: node.id,
            head: node.head,
        }
    });
    let nodes = future::join_all(reads).await;
    Ok(warp::reply::json(&nodes))
}

async fn serve_network_config(
    network_config: Arc<NetworkConfigResponse>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(warp::reply::json(&*network_config))
}

async fn get_fork_choice(fork_choice: ForkChoice) -> Result<impl warp::Reply, warp::Rejection> {
    let tree = fork_choice.tree.read().expect("has data");
    Ok(warp::reply::json(&*tree))
}

async fn serve_participation_data() -> Result<impl warp::Reply, warp::Rejection> {
    let response: Vec<String> = vec![];
    Ok(warp::reply::json(&response))
}

async fn serve_deposit_contract_data() -> Result<impl warp::Reply, warp::Rejection> {
    Ok(warp::reply::json(&"todo"))
}

async fn serve_weak_subjectivity_data() -> Result<impl warp::Reply, warp::Rejection> {
    Ok(warp::reply::json(&"todo"))
}
