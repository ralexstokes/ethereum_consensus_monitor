use crate::chain::{Coordinate, FinalityData};
use crate::eth2_api_client::{APIClientError, Eth2APIClient};
use crate::fork_choice::ProtoArray;
use eth2::types::Slot;
use reqwest::Client;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

const CONSENSUS_HEAD_SYNC_TIME_MILLIS: u64 = 150;
const CONSENSUS_HEAD_ATTEMPTS_PER_FETCH: u64 = 3;

fn hash_of(some_string: &str) -> u64 {
    let mut s = DefaultHasher::new();
    some_string.hash(&mut s);
    s.finish()
}

pub enum Status {
    Unreachable,
    Syncing,
    Healthy,
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

enum ConsensusType {
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

enum ExecutionType {
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

pub type Node = Arc<RwLock<NodeInner>>;

/// NodeInner represents an Ethereum node
pub struct NodeInner {
    // Reference to a consensus node
    api_client: Eth2APIClient,
    pub status: Status,
    node_type: Option<ConsensusType>,
    pub version: Option<String>,
    pub id: Option<u64>,
    // Indicate an attached execution client
    execution_node_type: Option<ExecutionType>,

    // last known head for this node
    pub head: Option<Coordinate>,
    head_delay_ms: u64,
}

impl fmt::Display for NodeInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let endpoint = self.api_client.get_endpoint();
        write!(f, "node at {} is {}", endpoint, self.status)?;
        write!(f, " with type ")?;
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

pub fn new_node(endpoint: &str, http: Client) -> Node {
    let api_client = Eth2APIClient::new(http, endpoint);
    let inner = NodeInner {
        api_client,
        status: Status::Unreachable,
        node_type: None,
        version: None,
        id: None,
        execution_node_type: None,
        head: None,
        head_delay_ms: 0,
    };
    Arc::new(RwLock::new(inner))
}

impl NodeInner {
    pub fn supports_fork_choice(&self) -> bool {
        matches!(self.node_type, Some(ConsensusType::Lighthouse))
    }

    async fn wait_for_head_delay(&self) {
        // allow some amount of time for synchronization
        // in the event this call is racing block propagation
        // during the current slot...
        sleep(Duration::from_millis(self.head_delay_ms)).await;
    }

    pub async fn fetch_fork_choice(&self) -> Result<ProtoArray, NodeError> {
        self.api_client
            .get_lighthouse_fork_choice()
            .await
            .map_err(|e| e.into())
    }

    pub async fn fetch_finality_data(&self, slot: Slot) -> Result<FinalityData, NodeError> {
        // TODO: fix synchronization here...
        self.wait_for_head_delay().await;
        self.api_client
            .get_finality_checkpoints(slot)
            .await
            .map(|checkpoints| checkpoints.into())
            .map_err(|e| e.into())
    }

    async fn fetch_head(&mut self) -> Result<Coordinate, NodeError> {
        self.wait_for_head_delay().await;
        let result = self.api_client.get_latest_header().await?;
        let (root, latest_header) = result;
        let slot = latest_header.message.slot;
        let head = Coordinate { slot, root };
        self.head = Some(head);
        Ok(head)
    }

    pub async fn fetch_head_with_hint(&mut self, slot_hint: Slot) -> Result<Coordinate, NodeError> {
        let mut head = self.fetch_head().await?;
        for _ in 0..CONSENSUS_HEAD_ATTEMPTS_PER_FETCH {
            if head.slot != slot_hint {
                // if the head is behind what the caller expects,
                // increase the `head_delay_ms` and try again...
                self.head_delay_ms += CONSENSUS_HEAD_SYNC_TIME_MILLIS;
                head = self.fetch_head().await?;
            } else {
                self.head_delay_ms = self.head_delay_ms.saturating_sub(10);
                break;
            }
        }
        Ok(head)
    }

    pub async fn refresh_status(&mut self) -> Result<(), NodeError> {
        let sync_status = self.api_client.get_sync_status().await?;
        self.status = if sync_status.is_syncing {
            Status::Syncing
        } else {
            Status::Healthy
        };
        Ok(())
    }

    pub async fn connect(&mut self) -> Result<(), NodeError> {
        let version = self.api_client.get_node_version().await?;
        self.node_type = infer_node_type(&version);
        self.version = Some(version);

        self.refresh_status().await?;

        let identity = self.api_client.get_identity_data().await?;
        let peer_id = identity.peer_id;
        self.id = Some(hash_of(&peer_id));

        self.fetch_head().await?;
        if self.supports_fork_choice() {
            self.fetch_fork_choice().await?;
        }
        Ok(())
    }
}
