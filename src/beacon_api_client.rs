use crate::node::ConsensusType;
use crate::{chain::Coordinate, fork_choice::ProtoArray};
use base64::{self, DecodeError};
use eth2::types::{
    BlockHeaderAndSignature, BlockHeaderData, ErrorMessage, FinalityCheckpointsData,
    GenericResponse, Hash256, IdentityData, Slot, SyncingData, VersionData,
};
use eventsource_client as sse;
use futures::{Stream, TryStreamExt};
use reqwest::{Client, Error as HTTPError};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{self, Error as JSONError};
use std::fmt::Write;
use std::time::Duration;
use thiserror::Error;

const ACCEPT_HEADER: &'static str = "Accept";
const ACCEPT_HEADER_VALUE: &'static str = "text/event-stream";

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
                    log::warn!(
                        "could not deserialize as json: `{}` (length {})",
                        text,
                        text.len()
                    );
                    Err(err.into())
                }
                Err(err) => Err(err.into()),
            },
        },
    }
}

#[derive(Clone, Debug)]
pub struct BeaconAPIClient {
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
    #[error("error with eventsource sse: {0}")]
    EventSourceError(String),
    #[error("error decoding base64 data: {0}")]
    Base64Error(#[from] DecodeError),
}

type APIResult<T> = Result<T, APIClientError>;

impl BeaconAPIClient {
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

    pub fn stream_head(
        &self,
        consensus_type: ConsensusType,
    ) -> impl Stream<Item = APIResult<APIResult<Coordinate>>> {
        let url = self.endpoint_for("events?topics=head");
        let sse_client = sse::Client::for_url(&url)
            .expect("can parse url")
            .reconnect(
                sse::ReconnectOptions::reconnect(true)
                    .retry_initial(false)
                    .delay(Duration::from_secs(5))
                    .backoff_factor(2)
                    .delay_max(Duration::from_secs(60))
                    .build(),
            )
            .header(ACCEPT_HEADER, ACCEPT_HEADER_VALUE)
            .expect("can add header")
            .build();
        parse_head_events(sse_client, consensus_type)
    }
}

fn parse_head_events(
    client: sse::Client<sse::HttpsConnector>,
    consensus_type: ConsensusType,
) -> impl Stream<Item = APIResult<APIResult<Coordinate>>> {
    client
        .stream()
        .map_ok(move |event| {
            let event_type = event.event_type.trim();
            assert!(event_type == "head");
            let data = event.field("data");
            if let Some(data) = data {
                let json: serde_json::Value = serde_json::from_slice(data).unwrap();
                match json {
                    serde_json::Value::Object(data) => {
                        let root = data.get("block").ok_or_else(|| {
                            APIClientError::APIError("missing block root from head API".to_string())
                        })?;
                        let root: Hash256 = match root {
                            serde_json::Value::String(data) => match consensus_type {
                                ConsensusType::Prysm => base64::decode(&data)
                                    .map(|root| Hash256::from_slice(&root))
                                    .map_err(|p| p.into()),
                                _ => (&data[2..]).parse::<Hash256>().map_err(|err| {
                                    let mut buffer = String::new();
                                    let _ = write!(&mut buffer, "{:?}", err);
                                    APIClientError::EventSourceError(buffer)
                                }),
                            },
                            _ => Err(APIClientError::APIError(
                                "wrong type for field in head event API".to_string(),
                            )),
                        }?;
                        let slot = data.get("slot").ok_or_else(|| {
                            APIClientError::APIError("missing slot from head event API".to_string())
                        })?;
                        let slot = match slot {
                            serde_json::Value::String(data) => {
                                data.parse::<Slot>().map_err(|err| {
                                    let mut buffer = String::new();
                                    let _ = write!(&mut buffer, "{:?}", err);
                                    APIClientError::EventSourceError(buffer)
                                })
                            }
                            _ => Err(APIClientError::APIError(
                                "wrong type for field in head event API".to_string(),
                            )),
                        }?;
                        Ok(Coordinate { slot, root })
                    }
                    _ => {
                        return Err(APIClientError::APIError(
                            "json from head stream did not match expected format".to_string(),
                        ))
                    }
                }
            } else {
                Err(APIClientError::APIError(
                    "expected API response is malformed".to_string(),
                ))
            }
        })
        .map_err(|err| {
            let mut buffer = String::new();
            let _ = write!(&mut buffer, "{:?}", err);
            APIClientError::EventSourceError(buffer)
        })
}
