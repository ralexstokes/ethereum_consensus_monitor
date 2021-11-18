use crate::chain::Coordinate;
use crate::config::Config;
use crate::monitor::{MonitorEvent, State};
use crate::node::Status;
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::filters::ws::Message;
use warp::Filter;

#[derive(Serialize)]
struct NodeResponse {
    id: Option<u64>,
    head: Option<Coordinate>,
    version: Option<String>,
    execution_client: Option<String>,
    healthy: bool,
    syncing: bool,
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

pub struct APIServer {
    state: Arc<State>,
}

macro_rules! get {
    ($path:literal, $handler:ident, $state:ident) => {
        warp::get()
            .and(warp::path($path))
            .and(warp::path::end())
            .and(with_state($state.clone()))
            .and_then($handler)
    };
}

impl APIServer {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }

    pub async fn run(&self, addr: impl Into<SocketAddr>) {
        let state = self.state.clone();

        let network_config = get!("network-config", serve_network_config, state);
        let nodes = get!("nodes", get_nodes, state);
        // let chain = get!("chain", get_chain_data, state);
        // // let fork_choice = get!("fork-choice", get_fork_choice, state);
        // let participation = get!("participation", serve_participation_data, state);
        // let deposit_contract = get!("deposit-contract", serve_deposit_contract_data, state);
        // let weak_subjectivity = get!("weak-subjectivity", serve_weak_subjectivity_data, state);
        let connect = warp::path("connect")
            .and(with_state(state.clone()))
            .and(warp::ws())
            .map(|state: Arc<State>, ws: warp::ws::Ws| {
                let mut rx = state.events_tx.subscribe();
                ws.on_upgrade(|mut socket| async move {
                    loop {
                        tokio::select! {
                            result = rx.recv() => {
                                match result {
                                    Ok(event) => {
                                        match event {
                                            head @ MonitorEvent::NewHead { .. } => {
                                                match serde_json::to_string(&head) {
                                                    Ok(msg) => {
                                                        let msg = Message::text(msg);
                                                        match socket.send(msg).await {
                                                            Ok(_) => {}
                                                            Err(err) => log::warn!(
                                                                "error sending ws message to client: {:?}",
                                                                err
                                                            ),
                                                        }
                                                    }
                                                    Err(err) => {
                                                        log::warn!("error serializing head update: {:?}", err);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        log::warn!("error receiving update: {:?}", err);
                                    }
                                }
                            }
                            msg = socket.next() => {
                                match msg {
                                    Some(Ok(msg)) => {
                                        if msg.is_close() {
                                            log::debug!("ws client disconnecting");
                                            break;
                                        }
                                    }
                                    Some(Err(err)) => {
                                        log::warn!("error receiving ws message from client: {:?}", err);
                                        break;
                                    }
                                    None => break,
                                }
                            }
                        }
                    }
                })
            });

        let api = warp::get()
            .and(warp::path("api"))
            .and(warp::path("v1"))
            .and(
                network_config
                    .or(nodes)
                    .or(connect)
                    // .or(chain)
                    // .or(fork_choice)
                    // .or(participation)
                    // .or(deposit_contract)
                    // .or(weak_subjectivity),
            );

        let html_dir = state.config.monitor.output_dir.clone();
        let app = warp::get().and(warp::any()).and(warp::fs::dir(html_dir));

        let routes = api.or(app);

        warp::serve(routes).run(addr).await
    }
}

fn with_state(
    state: Arc<State>,
) -> impl Filter<Extract = (Arc<State>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn get_nodes(state: Arc<State>) -> Result<impl warp::Reply, warp::Rejection> {
    let nodes = state
        .nodes
        .iter()
        .map(|node| {
            let node = node.state.lock().expect("can read");
            NodeResponse {
                id: node.id,
                head: node.head,
                version: node.version.clone(),
                execution_client: node.execution_description.clone(),
                healthy: matches!(node.status, Status::Healthy | Status::Syncing),
                syncing: matches!(node.status, Status::Syncing),
            }
        })
        .collect::<Vec<_>>();
    Ok(warp::reply::json(&nodes))
}

async fn serve_network_config(state: Arc<State>) -> Result<impl warp::Reply, warp::Rejection> {
    let network_config: NetworkConfigResponse = (&state.config).into();
    Ok(warp::reply::json(&network_config))
}

// async fn get_chain_data(state: Arc<State>) -> Result<impl warp::Reply, warp::Rejection> {
//     // let status = state.chain.get_status();
//     let status: FinalityData = Default::default();
//     Ok(warp::reply::json(&status))
// }

// async fn get_fork_choice(state: Arc<State>) -> Result<impl warp::Reply, warp::Rejection> {
//     let state = state.lock().expect("can read state");
//     let tree = state.fork_choice.tree.read().expect("has data");
//     Ok(warp::reply::json(&*tree))
// }

// async fn serve_participation_data(_state: Arc<State>) -> Result<impl warp::Reply, warp::Rejection> {
//     let response: Vec<String> = vec![];
//     Ok(warp::reply::json(&response))
// }

// async fn serve_deposit_contract_data(
//     _state: Arc<State>,
// ) -> Result<impl warp::Reply, warp::Rejection> {
//     Ok(warp::reply::json(&"todo"))
// }

// async fn serve_weak_subjectivity_data(
//     _state: Arc<State>,
// ) -> Result<impl warp::Reply, warp::Rejection> {
//     Ok(warp::reply::json(&"todo"))
// }
