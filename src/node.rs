use crate::eth2_api_client::{APIClientError, Eth2APIClient};
use eth2::types::{Hash256, Slot};
use reqwest::Client;
use serde::Serialize;
use std::fmt;
use thiserror::Error;
use tokio::time::{sleep, Duration};

const CONSENSUS_HEAD_SYNC_TIME_MILLIS: u64 = 150;
const CONSENSUS_HEAD_ATTEMPTS_PER_FETCH: u64 = 3;

#[derive(Copy, Clone, Debug, Serialize)]
pub struct Head {
    pub slot: Slot,
    pub root: Hash256,
}

impl fmt::Display for Head {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.slot, self.root)
    }
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

#[derive(Error, Debug)]
#[error("{0}")]
pub enum NodeError {
    APIError(#[from] APIClientError),
}

/// Node represents an Ethereum node
pub struct Node {
    // Reference to a consensus node
    api_client: Eth2APIClient,
    status: Status,
    node_type: Option<ConsensusType>,

    // last known head for this node
    pub head: Option<Head>,
    head_delay_ms: u64,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let endpoint = self.api_client.get_endpoint();
        write!(f, "node at {} is {}", endpoint, self.status)?;
        write!(f, " with type ")?;
        if let Some(ref node_type) = self.node_type {
            write!(f, "{}", node_type)?
        } else {
            write!(f, "unknown")?
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

impl Node {
    pub fn new(endpoint: &str, http: Client) -> Self {
        let api_client = Eth2APIClient::new(http, endpoint);
        Node {
            api_client,
            status: Status::Unreachable,
            node_type: None,
            head: None,
            head_delay_ms: 0,
        }
    }

    async fn fetch_head(&mut self) -> Result<Head, NodeError> {
        // allow some amount of time for synchronization
        // in the event this call is racing block propagation
        // during the current slot...
        sleep(Duration::from_millis(self.head_delay_ms)).await;

        let result = self.api_client.get_latest_header().await?;
        let (root, latest_header) = result;
        let slot = latest_header.message.slot;
        let head = Head { slot, root };
        self.head = Some(head);
        Ok(head)
    }

    pub async fn fetch_head_with_hint(&mut self, slot_hint: Slot) -> Result<Head, NodeError> {
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

    pub async fn connect(&mut self) -> Result<(), NodeError> {
        let version = self.api_client.get_node_version().await?;

        let sync_status = self.api_client.get_sync_status().await?;

        self.status = if sync_status.is_syncing {
            Status::Syncing
        } else {
            Status::Healthy
        };
        self.node_type = infer_node_type(&version);
        self.fetch_head().await?;
        Ok(())
    }
}
