use crate::beacon_api_client::{APIClientError, BeaconAPIClient};
use crate::chain::Coordinate;
use reqwest::Client;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use thiserror::Error;

fn hash_of(some_string: &str) -> u64 {
    let mut s = DefaultHasher::new();
    some_string.hash(&mut s);
    s.finish()
}

fn infer_node_type(version: &str) -> Option<ConsensusType> {
    if version.to_lowercase().contains("prysm") {
        return Some(ConsensusType::Prysm);
    }
    if version.to_lowercase().contains("lighthouse") {
        return Some(ConsensusType::Lighthouse);
    }
    if version.to_lowercase().contains("teku") {
        return Some(ConsensusType::Teku);
    }
    if version.to_lowercase().contains("nimbus") {
        return Some(ConsensusType::Nimbus);
    }
    if version.to_lowercase().contains("lodestar") {
        return Some(ConsensusType::Lodestar);
    }
    None
}

#[derive(Debug, Clone)]
pub enum Status {
    Unreachable,
    Syncing,
    Healthy,
}

impl Default for Status {
    fn default() -> Self {
        Self::Unreachable
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Status::Unreachable => write!(f, "unreachable"),
            Status::Syncing => write!(f, "syncing"),
            Status::Healthy => write!(f, "healthy"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConsensusType {
    Prysm,
    Lighthouse,
    Teku,
    Nimbus,
    Lodestar,
}

impl fmt::Display for ConsensusType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConsensusType::Prysm => write!(f, "Prysm"),
            ConsensusType::Lighthouse => write!(f, "Lighthouse"),
            ConsensusType::Teku => write!(f, "Teku"),
            ConsensusType::Nimbus => write!(f, "Nimbus"),
            ConsensusType::Lodestar => write!(f, "Lodestar"),
        }
    }
}

#[derive(Debug)]
pub enum ExecutionType {
    Geth,
    Nethermind,
    Besu,
    Erigon,
}

impl fmt::Display for ExecutionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExecutionType::Geth => write!(f, "Geth"),
            ExecutionType::Nethermind => write!(f, "Nethermind"),
            ExecutionType::Besu => write!(f, "Besu"),
            ExecutionType::Erigon => write!(f, "Erigon"),
        }
    }
}

#[derive(Error, Debug)]
#[error("{0}")]
pub enum NodeError {
    APIError(#[from] APIClientError),
}

#[derive(Default, Debug)]
pub struct NodeState {
    pub id: Option<u64>,

    pub status: Status,
    pub node_type: Option<ConsensusType>,
    // NOTE: temp for interop
    pub execution_description: Option<String>,
    pub version: Option<String>,
    // Indicate an attached execution client
    pub execution_node_type: Option<ExecutionType>,

    // last known head for this node
    pub head: Option<Coordinate>,
}

/// Node represents an Ethereum node
#[derive(Debug)]
pub struct Node {
    pub endpoint: String,
    pub api_client: BeaconAPIClient,
    pub state: Mutex<NodeState>,
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "is {} with type ", self.status)?;
        if let Some(ref node_type) = self.node_type {
            write!(f, "{}", node_type)?
        } else {
            write!(f, "unknown")?
        }
        if let Some(ref node_type) = self.execution_node_type {
            write!(f, "and execution client {}", node_type)?
        }
        write!(f, " with head ")?;
        if let Some(ref head) = self.head {
            write!(f, "{}", head)
        } else {
            write!(f, "unknown")
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let endpoint = &self.endpoint;
        let state = self.state.lock().expect("can lock state");
        write!(f, "node at {} {} ", endpoint, state)
    }
}

impl Node {
    pub fn new(
        endpoint: &str,
        execution_description: Option<&String>,
        http_client: Client,
    ) -> Self {
        let mut state: NodeState = Default::default();
        state.execution_description = execution_description.map(|s| s.to_string());
        Self {
            endpoint: endpoint.to_string(),
            api_client: BeaconAPIClient::new(http_client, endpoint),
            state: Mutex::new(state),
        }
    }

    pub fn supports_fork_choice(&self) -> bool {
        let state = self.state.lock().expect("can read state");
        matches!(state.node_type, Some(ConsensusType::Lighthouse))
    }

    // pub async fn fetch_fork_choice(&self) -> Result<ProtoArray, NodeError> {
    //     self.api_client
    //         .get_lighthouse_fork_choice()
    //         .await
    //         .map_err(|e| e.into())
    // }

    // pub async fn fetch_finality_data(&self, slot: Slot) -> Result<FinalityData, NodeError> {
    //     // TODO: fix synchronization here...
    //     self.wait_for_head_delay().await;
    //     self.api_client
    //         .get_finality_checkpoints(slot)
    //         .await
    //         .map(|checkpoints| checkpoints.into())
    //         .map_err(|e| e.into())
    // }

    pub async fn fetch_status(&self) -> Result<Status, NodeError> {
        let sync_status = self.api_client.get_sync_status().await?;
        let mut inner = self.state.lock().expect("can lock state");
        let status = if sync_status.is_syncing {
            Status::Syncing
        } else {
            Status::Healthy
        };
        inner.status = status.clone();
        Ok(status)
    }

    pub async fn fetch_version(&self) -> Result<(), NodeError> {
        let version = self.api_client.get_node_version().await?;
        let mut inner = self.state.lock().expect("can lock state");
        inner.node_type = infer_node_type(&version);
        inner.version = Some(version);
        Ok(())
    }

    pub async fn fetch_identity(&self) -> Result<(), NodeError> {
        let identity = self.api_client.get_identity_data().await?;
        let mut inner = self.state.lock().expect("can lock state");
        let peer_id = identity.peer_id;
        inner.id = Some(hash_of(&peer_id));
        Ok(())
    }

    pub async fn connect(&self) -> Result<(), NodeError> {
        self.fetch_version().await?;
        self.fetch_status().await?;
        self.fetch_identity().await?;
        Ok(())
    }

    pub fn update_head(&self, head: Coordinate) {
        let mut inner = self.state.lock().expect("can lock state");
        inner.head = Some(head);
    }
}
