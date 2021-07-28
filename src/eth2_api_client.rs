use crate::fork_choice::ProtoArray;
use eth2::types::{
    BlockHeaderAndSignature, BlockHeaderData, ErrorMessage, FinalityCheckpointsData,
    GenericResponse, Hash256, IdentityData, Slot, SyncingData, VersionData,
};
use reqwest::{Client, Error as HTTPError};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{self, Error as JSONError};
use thiserror::Error;

async fn do_get<T>(client: &Client, endpoint: &str) -> Result<T, APIClientError>
where
    T: Serialize + DeserializeOwned,
{
    let response = client.get(endpoint).send().await?;
    let body = response.bytes().await?;
    let result = serde_json::from_slice::<GenericResponse<T>>(&body).map(|resp| resp.data);
    match result {
        Ok(result) => Ok(result),
        Err(err) => match serde_json::from_slice::<ErrorMessage>(&body) {
            Ok(error) => Err(APIClientError::APIError(error.message)),
            Err(_) => match std::str::from_utf8(&body) {
                Ok(text) => {
                    log::warn!("could not deserialize as json: {}", text);
                    Err(err.into())
                }
                Err(err) => Err(err.into()),
            },
        },
    }
}

pub struct Eth2APIClient {
    http: Client,
    endpoint: String,
}

const ENDPOINT_PREFIX: &str = "/eth/v1/";

#[derive(Error, Debug)]
pub enum APIClientError {
    #[error("API error: {0}")]
    APIError(String),
    #[error("http error: {0}")]
    HTTPClient(#[from] HTTPError),
    #[error("json error: {0}")]
    SerdeError(#[from] JSONError),
    #[error("string decoding error: {0}")]
    StringError(#[from] std::str::Utf8Error),
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
        do_get(&self.http, &endpoint)
            .await
            .map(|data: VersionData| data.version)
    }

    pub async fn get_latest_header(&self) -> APIResult<(Hash256, BlockHeaderAndSignature)> {
        let endpoint = self.endpoint_for("beacon/headers/head");
        do_get(&self.http, &endpoint)
            .await
            .map(|data: BlockHeaderData| (data.root, data.header))
    }

    pub async fn get_sync_status(&self) -> APIResult<SyncingData> {
        let endpoint = self.endpoint_for("node/syncing");
        do_get(&self.http, &endpoint).await
    }

    pub async fn get_identity_data(&self) -> APIResult<IdentityData> {
        let endpoint = self.endpoint_for("node/identity");
        do_get(&self.http, &endpoint).await
    }

    pub async fn get_finality_checkpoints(&self, slot: Slot) -> APIResult<FinalityCheckpointsData> {
        let endpoint_query =
            String::from("beacon/states/") + &slot.to_string() + "/finality_checkpoints";
        let endpoint = self.endpoint_for(&endpoint_query);
        do_get(&self.http, &endpoint).await
    }

    pub async fn get_lighthouse_fork_choice(&self) -> APIResult<ProtoArray> {
        let endpoint = String::from(self.get_endpoint()) + "/lighthouse/proto_array";
        do_get(&self.http, &endpoint).await
    }
}
