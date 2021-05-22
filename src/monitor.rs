use crate::api_server::ApiServer;
use crate::chain::Coordinate;
use crate::config::Config;
use crate::fork_choice::ForkChoice;
use crate::node::{new_node, Node, Status};
use crate::timer::Timer;
use futures::future;
use reqwest::Client;
use std::path::Path;
use std::path::PathBuf;
use tokio::task;

pub struct Monitor {
    config: Config,
    output_dir: PathBuf,
    port: u16,
}

impl Monitor {
    pub fn with_output_dir<P>(&mut self, output_dir: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        self.output_dir = output_dir.as_ref().to_path_buf();
        self
    }

    pub fn with_port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }
}

impl Monitor {
    async fn connect_to_nodes(&self) -> Vec<Node> {
        let client = Client::new();
        let connections = self.config.endpoints.iter().map(|endpoint| {
            let client = client.clone();
            async move {
                let node_ref = new_node(endpoint, client);
                {
                    let mut node = node_ref.write().await;
                    if let Err(err) = node.connect().await {
                        log::warn!("{}", err);
                    }
                }
                node_ref
            }
        });
        future::join_all(connections).await
    }

    fn build_timer(&self) -> Timer {
        let timer_config = &self.config.consensus_chain;

        Timer::new(
            timer_config.genesis_time,
            timer_config.seconds_per_slot,
            timer_config.slots_per_epoch,
        )
    }

    pub async fn run(&self) {
        let timer = self.build_timer();

        let nodes = self.connect_to_nodes().await;
        let nodes_handle = nodes.clone();

        let fork_choice_provider = find_fork_choice_provider(&nodes).await;
        let fork_choice_head = if let Some(ref node) = fork_choice_provider {
            let node = node.read().await;
            node.head.expect("has head")
        } else {
            Coordinate::default()
        };
        let fork_choice = ForkChoice::new(
            fork_choice_head,
            self.config.consensus_chain.slots_per_epoch,
        );
        let fork_choice_handle = fork_choice.clone();

        let timer_task = task::spawn(async move {
            if timer.is_before_genesis() {
                log::warn!("before genesis, blocking head monitor until then...");
            }
            loop {
                let slot = timer.tick_slot().await;

                let fetches = nodes.iter().map(|node| async move {
                    let mut node = node.write().await;
                    let result = match node.status {
                        Status::Healthy => node.fetch_head_with_hint(slot).await.map(|_| ()),
                        Status::Syncing => node.refresh_status().await,
                        Status::Unreachable => node.refresh_status().await,
                    };
                    if let Err(e) = result {
                        log::warn!("{}", e);
                        node.status = Status::Unreachable;
                    }
                });
                future::join_all(fetches).await;

                if let Some(ref fork_choice_provider) = fork_choice_provider {
                    let mut node = fork_choice_provider.write().await;
                    let result = node.fetch_fork_choice().await;
                    let mut fork_choice = fork_choice.clone();
                    match result {
                        Ok(proto_array) => {
                            let _ = task::spawn_blocking(move || {
                                fork_choice.update(proto_array);
                            });
                        }
                        Err(e) => {
                            log::warn!("{}", e);
                            node.status = Status::Unreachable;
                        }
                    }
                }
            }
        });

        let port = self.port;
        let api_server = ApiServer::new(
            &self.output_dir,
            nodes_handle,
            fork_choice_handle,
            self.config.get_spec(),
        );
        let server_task = task::spawn(async move {
            api_server.run(([127, 0, 0, 1], port)).await;
        });

        let tasks = vec![timer_task, server_task];
        future::join_all(tasks).await;
    }
}

pub fn from_config(config: &str) -> Monitor {
    let config = toml::from_str(config).expect("config is well-formatted TOML");
    Monitor {
        config,
        output_dir: PathBuf::from("public"),
        port: 3030,
    }
}

async fn find_fork_choice_provider(nodes: &Vec<Node>) -> Option<Node> {
    for node_ref in nodes {
        let node = node_ref.read().await;
        if node.supports_fork_choice() {
            return Some(node_ref.clone());
        }
    }
    None
}
