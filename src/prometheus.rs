use reqwest::Client;
use serde::Deserialize;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::time::SystemTime;
use thiserror::Error;
use tokio::time::Duration;

#[derive(Debug, Error)]
pub enum PrometheusError {
    #[error("Network error: {0}")]
    Network(reqwest::Error),
    #[error("Malformed json response: {0}")]
    MalformedResponse(reqwest::Error),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryResultStatus {
    Success,
    Error,
}

#[derive(Debug, Clone, Deserialize)]
struct QueryResult {
    status: QueryResultStatus,
    data: QueryResultData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
enum QueryResultDataType {
    Matrix,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueryResultData {
    result_type: QueryResultDataType,
    result: Vec<QueryResultDataResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct QueryResultDataResult {
    metric: HashMap<String, String>,
    values: Vec<QueryResultDataResultValue>,
}

#[derive(Debug, Clone, Deserialize)]
struct QueryResultDataResultValue(u64, String);

async fn query_prometheus(
    client: &Client,
    base_url: &str,
    query: &str,
    start: u64,
    end: u64,
    step: usize,
) -> Result<QueryResult, PrometheusError> {
    client
        .get(base_url)
        .query(&[
            ("query", query),
            ("start", &format!("{}", start)),
            ("end", &format!("{}", end)),
            ("step", &format!("{}", step)),
        ])
        .send()
        .await
        .map_err(|err| PrometheusError::Network(err))?
        .json()
        .await
        .map_err(|err| PrometheusError::MalformedResponse(err))
}

#[derive(Debug, Clone)]
pub struct StallDetector {
    client: Client,
    base_url: String,
}

impl StallDetector {
    pub fn new(prometheus_url: impl std::fmt::Display) -> Self {
        StallDetector {
            client: Client::new(),
            base_url: format!("{}/api/v1/query_range", prometheus_url),
        }
    }

    /// Get a list of tasmota devices for which a specified sensor has stalled
    pub async fn get_stalled(
        &self,
        metrics: &str,
        duration: Duration,
    ) -> Result<impl Iterator<Item = String>, PrometheusError> {
        let end_time = SystemTime::now();
        let start_time = end_time - duration;

        let end_time = end_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let start_time = start_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let data = query_prometheus(
            &self.client,
            &self.base_url,
            metrics,
            start_time,
            end_time,
            min(
                60usize,
                max(2usize, (end_time as usize - start_time as usize) / 240),
            ),
        )
        .await?
        .data
        .result;

        Ok(data.into_iter().filter_map(|mut query_result| {
            let mut values = query_result.values.into_iter();
            let first = values.next()?.1;

            if values.all(|value| value.1 == first) {
                query_result.metric.remove("tasmota_id")
            } else {
                None
            }
        }))
    }
}
