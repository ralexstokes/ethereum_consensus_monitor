use eth2::types::{
    BlockHeaderAndSignature, BlockHeaderData, GenericResponse, Hash256, SyncingData, VersionData,
};
use reqwest::{Client, Error as HTTPError};
use serde_json::Error as JSONError;
use thiserror::Error;

pub struct Eth2APIClient {
    http: Client,
    endpoint: String,
}

const ENDPOINT_PREFIX: &str = "/eth/v1/";

#[derive(Error, Debug)]
#[error("{0}")]
pub enum APIClientError {
    HTTPClient(#[from] HTTPError),
    SerdeError(#[from] JSONError),
}

type APIResult<T> = Result<T, APIClientError>;

impl Eth2APIClient {
    pub fn new(client: Client, endpoint: &str) -> Self {
        Self {
            http: client,
            endpoint: endpoint.to_string() + ENDPOINT_PREFIX,
        }
    }

    pub fn get_endpoint(&self) -> &str {
        self.endpoint.trim_end_matches(ENDPOINT_PREFIX)
    }

    fn endpoint_for(&self, suffix: &str) -> String {
        let mut result = self.endpoint.clone();
        result += suffix;
        result
    }

    pub async fn get_node_version(&self) -> APIResult<String> {
        let endpoint = self.endpoint_for("node/version");
        let response = self
            .http
            .get(endpoint)
            .send()
            .await?
            .json::<GenericResponse<VersionData>>()
            .await?;
        Ok(response.data.version)
    }

    pub async fn get_latest_header(&self) -> APIResult<(Hash256, BlockHeaderAndSignature)> {
        let endpoint = self.endpoint_for("beacon/headers/head");
        let response = self
            .http
            .get(endpoint)
            .send()
            .await?
            .json::<GenericResponse<BlockHeaderData>>()
            .await?;
        Ok((response.data.root, response.data.header))
    }

    pub async fn get_sync_status(&self) -> APIResult<SyncingData> {
        let endpoint = self.endpoint_for("node/syncing");
        let response = self
            .http
            .get(endpoint)
            .send()
            .await?
            .json::<GenericResponse<SyncingData>>()
            .await?;
        Ok(response.data)
    }
}
