use crate::api_server::APIServer;
use crate::chain::{Chain, Coordinate};
use crate::config::Config;
use crate::node::{Node, Status};
use crate::timer::Timer;
use futures::{future, TryStreamExt};
use reqwest::{Client, ClientBuilder};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::{self, Sender};
use tokio::task::{self, JoinHandle};
use tokio::time::sleep;

const LOCALHOST: [u8; 4] = [0, 0, 0, 0];
const TEN_MINUTES_AS_SECONDS: u64 = 600;
const NODE_CONNECT_ATTEMPTS: usize = 128;

#[derive(Debug, Serialize, Clone)]
pub enum MonitorEvent {
    #[serde(rename = "new_head")]
    NewHead {
        id: u64,
        head: Coordinate,
        syncing: bool,
    },
}

pub struct Monitor {
    _timer: Timer,
    state: Arc<State>,
}

pub struct State {
    pub config: Config,
    pub nodes: Vec<Arc<Node>>,
    pub chain: Chain,
    pub events_tx: Sender<MonitorEvent>,
}

fn build_node(
    endpoint: &str,
    execution_description: Option<&String>,
    http_client: Client,
) -> Arc<Node> {
    Arc::new(Node::new(endpoint, execution_description, http_client))
}

fn build_nodes<'a>(
    endpoints: impl Iterator<Item = (&'a String, Option<&'a String>)>,
    http_client: &Client,
) -> Vec<Arc<Node>> {
    endpoints
        .map(|(endpoint, execution_description)| {
            build_node(endpoint, execution_description, http_client.clone())
        })
        .collect()
}

async fn connect_to_node(node: &Arc<Node>) {
    for _ in 0..NODE_CONNECT_ATTEMPTS {
        while let Err(err) = node.connect().await {
            log::warn!("{}", err);
            sleep(Duration::from_secs(TEN_MINUTES_AS_SECONDS)).await;
        }
    }
}

async fn stream_head_updates(node: &Arc<Node>, channel: Sender<MonitorEvent>) {
    let client = &node.api_client;
    let consensus_type = {
        let state = node.state.lock().expect("can read state");
        let consensus_type = state.node_type.clone();

        if consensus_type.is_none() {
            return;
        }

        consensus_type.unwrap()
    };

    let mut stream = Box::pin(client.stream_head(consensus_type));
    while let Ok(Some(head)) = stream.try_next().await {
        match head {
            Ok(head) => {
                node.update_head(head);
                let event =
                    node.state
                        .lock()
                        .expect("can read state")
                        .id
                        .map(|id| MonitorEvent::NewHead {
                            id,
                            head,
                            syncing: false,
                        });
                if event.is_none() {
                    continue;
                }
                let event = event.unwrap();
                let syncing = match node.fetch_status().await {
                    Ok(status) => {
                        matches!(status, Status::Syncing)
                    }
                    Err(err) => {
                        log::warn!("could not fetch node status: {}", err);
                        false
                    }
                };
                let event = match event {
                    MonitorEvent::NewHead { id, head, .. } => {
                        MonitorEvent::NewHead { id, head, syncing }
                    }
                };
                if let Ok(subscriber_count) = channel.send(event) {
                    log::debug!(
                        "sent head updates to {} connected clients",
                        subscriber_count
                    );
                }
                // ignore errors as they only signal lack of subscribers
            }
            Err(err) => {
                log::warn!("error streaming head for node: {}", err);
                continue;
            }
        }
    }
}

// async fn find_fork_choice_provider(nodes: &[Arc<Node>]) -> Option<&Arc<Node>> {
//     nodes.iter().find(|node| node.supports_fork_choice())
// }

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
        let nodes = build_nodes(
            config
                .monitor
                .endpoints
                .iter()
                .map(|endpoint| (&endpoint.consensus, endpoint.execution.as_ref())),
            &http_client,
        );
        let node_count = nodes.len();
        let head_event_buffer_size = 4 * node_count;
        let (events_tx, _) = broadcast::channel(head_event_buffer_size);
        let state = State {
            config,
            nodes,
            chain: Default::default(),
            events_tx,
        };
        Self {
            _timer: timer,
            state: Arc::new(state),
        }
    }

    pub async fn run(&self) {
        let mut tasks = self
            .state
            .nodes
            .iter()
            .map(|node| {
                let node = node.clone();
                let channel = self.state.events_tx.clone();
                task::spawn(async move {
                    connect_to_node(&node).await;
                    stream_head_updates(&node, channel).await;
                })
            })
            .collect::<Vec<JoinHandle<_>>>();

        let api_server = APIServer::new(self.state.clone());
        let port = self.state.config.monitor.port;
        let server_task = task::spawn(async move {
            api_server.run((LOCALHOST, port)).await;
        });

        tasks.push(server_task);
        future::join_all(tasks).await;
    }
}

// tmp scratch
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
