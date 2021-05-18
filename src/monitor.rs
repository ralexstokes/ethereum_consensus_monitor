use crate::api_server::ApiServer;
use crate::config::Config;
use crate::node::{Node, Status};
use crate::timer::Timer;
use futures::future;
use reqwest::Client;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task;

pub struct Monitor {
    config: Config,
    output_dir: PathBuf,
    port: u16,
}

pub type Nodes = Arc<Vec<RwLock<Node>>>;

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
    async fn connect_to_nodes(&self) -> Nodes {
        let client = Client::new();
        let connections = self.config.endpoints.iter().map(|endpoint| {
            let client = client.clone();
            async move {
                let mut node = Node::new(endpoint, client);
                if let Err(err) = node.connect().await {
                    log::warn!("{}", err);
                }
                RwLock::new(node)
            }
        });
        let nodes = future::join_all(connections).await;
        Arc::new(nodes)
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
        let timer_task = task::spawn(async move {
            if timer.is_before_genesis() {
                log::warn!("before genesis, blocking head monitor until then...");
            }
            loop {
                let slot = timer.tick_slot().await;

                let fetches = nodes.iter().map(|node| async move {
                    let mut node = node.write().await;
                    if let Status::Syncing = node.status {
                        if let Err(e) = node.refresh_status().await {
                            log::warn!("{}", e);
                            node.status = Status::Unreachable;
                        }
                    }
                    let result = node.fetch_head_with_hint(slot).await;
                    if let Err(e) = result {
                        log::warn!("{}", e);
                        node.status = Status::Unreachable;
                    }
                });
                future::join_all(fetches).await;
            }
        });

        let port = self.port;
        let api_server = ApiServer::new(&self.output_dir, nodes_handle);
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
