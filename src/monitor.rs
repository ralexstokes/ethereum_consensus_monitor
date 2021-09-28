use crate::api_server::ApiServer;
use crate::beacon_api_client::BeaconAPIClient;
use crate::chain::{Chain, Coordinate};
use crate::config::Config;
use crate::fork_choice::ForkChoice;
use crate::node::{Node, Status};
use crate::timer::Timer;
use eth2::lighthouse_vc::http_client;
use futures::{future, Stream, TryStreamExt};
use reqwest::{Client, ClientBuilder};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::{self, JoinHandle};
use tokio::time::sleep;

const LOCALHOST: [u8; 4] = [127, 0, 0, 1];
const TEN_MINUTES_AS_SECONDS: u64 = 600;
const NODE_CONNECT_ATTEMPTS: usize = 128;

pub struct Monitor {
    timer: Timer,
    state: Arc<State>,
}

#[derive(Default)]
pub struct State {
    pub config: Config,
    pub nodes: Vec<Arc<Node>>,
    // fork_choice: ForkChoice,
    pub chain: Chain,
}

fn build_node(endpoint: &String, http_client: Client) -> Arc<Node> {
    Arc::new(Node::new(endpoint, http_client))
}

fn build_nodes<'a>(
    endpoints: impl Iterator<Item = &'a String>,
    http_client: &Client,
) -> Vec<Arc<Node>> {
    endpoints
        .map(|endpoint| build_node(endpoint, http_client.clone()))
        .collect()
}

async fn connect_to_node(node: &Arc<Node>) {
    for _ in 0..NODE_CONNECT_ATTEMPTS {
        while let Err(err) = node.connect().await {
            log::warn!("{}", err);
            sleep(Duration::from_secs(TEN_MINUTES_AS_SECONDS)).await;
        }
        break;
    }
}

async fn stream_head_updates(node: &Arc<Node>) {
    let client = &node.api_client;

    let mut stream = Box::pin(client.stream_head());
    while let Ok(Some(head)) = stream.try_next().await {
        match head {
            Ok(head) => node.update_head(head),
            Err(err) => {
                log::warn!("error streaming head for node: {}", err);
                continue;
            }
        }
    }
}

impl Monitor {
    pub fn from_config(config: &str) -> Self {
        let config: Config = toml::from_str(config).expect("config is well-formatted TOML");
        let timer_config = &config.consensus_chain;
        let timer = Timer::new(
            timer_config.genesis_time,
            timer_config.seconds_per_slot,
            timer_config.slots_per_epoch,
        );

        let http_client = ClientBuilder::new()
            // .connect_timeout(Duration::from_millis(1000))
            // .timeout(Duration::from_millis(1000))
            .build()
            .expect("no errors with http client setup");
        let nodes = build_nodes(config.monitor.endpoints.iter(), &http_client);
        let state = State {
            config,
            nodes,
            ..Default::default()
        };
        Self {
            timer,
            state: Arc::new(state),
        }
    }

    // async fn connect_to_nodes(&self) -> Vec<Node> {
    //     let connections = self.config.monitor.endpoints.iter().map(|endpoint| {
    //         let client = client.clone();
    //         async move {
    //             let node_ref = new_node(endpoint, client);
    //             {
    //                 let mut node = node_ref.write().await;
    //                 if let Err(err) = node.connect().await {
    //                     log::warn!("{}", err);
    //                 }
    //             }
    //             node_ref
    //         }
    //     });
    //     future::join_all(connections).await
    // }

    pub async fn run(&self) {
        let timer = &self.timer;

        let state = self.state.clone();
        let nodes_task = self
            .state
            .nodes
            .iter()
            .map(|node| {
                let node = node.clone();
                task::spawn(async move {
                    connect_to_node(&node).await;
                    stream_head_updates(&node).await;
                })
            })
            .collect::<Vec<JoinHandle<_>>>();

        // let fork_choice_provider = find_fork_choice_provider(&nodes).await;
        // let fork_choice_head = if let Some(ref node) = fork_choice_provider {
        //     let node = node.read().await;
        //     node.head.expect("has head")
        // } else {
        //     Coordinate::default()
        // };
        // let fork_choice = ForkChoice::new(
        //     fork_choice_head,
        //     self.config.consensus_chain.slots_per_epoch,
        // );
        // let fork_choice_handle = fork_choice.clone();
        // let chain = Chain::default();
        // let chain_handle = chain.clone();

        // let timer_task = task::spawn(async move {
        //     if timer.is_before_genesis() {
        //         log::warn!("before genesis, blocking monitor until then...");
        //     }
        //     loop {
        //         let (slot, epoch) = timer.tick_slot().await;
        //         log::trace!("epoch: {}, slot: {}", epoch, slot);

        //         let fetches = nodes.iter().map(|node| async move {
        //             let mut node = node.write().await;
        //             let result = match node.status {
        //                 Status::Healthy => node.fetch_head_with_hint(slot).await.map(|_| ()),
        //                 Status::Syncing => node.refresh_status().await,
        //                 Status::Unreachable => node.refresh_status().await,
        //             };
        //             if let Err(e) = result {
        //                 log::warn!("{}", e);
        //                 node.status = Status::Unreachable;
        //             }
        //         });
        //         future::join_all(fetches).await;

        //         if let Some(ref fork_choice_provider) = fork_choice_provider {
        //             let mut node = fork_choice_provider.write().await;
        //             let result = node.fetch_fork_choice().await;
        //             let mut fork_choice = fork_choice_handle.clone();
        //             match result {
        //                 Ok(proto_array) => {
        //                     let _ = task::spawn_blocking(move || {
        //                         fork_choice.update(proto_array);
        //                     });
        //                 }
        //                 Err(e) => {
        //                     log::warn!("{}", e);
        //                     node.status = Status::Unreachable;
        //                 }
        //             }

        //             let result = node.fetch_finality_data(slot).await;
        //             match result {
        //                 Ok(finality_data) => chain_handle.set_status(finality_data),
        //                 Err(e) => {
        //                     log::warn!("{}", e);
        //                     node.status = Status::Unreachable;
        //                 }
        //             }
        //         }
        //     }
        // });

        let api_server = ApiServer::new(self.state.clone());
        let port = self.state.config.monitor.port;
        let server_task = task::spawn(async move {
            api_server.run((LOCALHOST, port)).await;
        });

        // let tasks = vec![nodes_task];
        future::join_all(nodes_task).await;
    }
}

// async fn find_fork_choice_provider(nodes: &[Node]) -> Option<Node> {
//     for node_ref in nodes {
//         let node = node_ref.read().await;
//         if node.supports_fork_choice() {
//             return Some(node_ref.clone());
//         }
//     }
//     None
// }
