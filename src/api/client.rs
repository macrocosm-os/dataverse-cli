use anyhow::{bail, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::time::Duration;

use super::types::*;

const DEFAULT_BASE_URL: &str = "https://constellation.api.cloud.macrocosmos.ai";

const SN13_SERVICE: &str = "sn13.v1.Sn13Service";
const GRAVITY_SERVICE: &str = "gravity.v1.GravityService";

pub struct ApiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(api_key: String, base_url: Option<String>, timeout_secs: u64) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-client-id",
            HeaderValue::from_static("dataverse-rust-cli"),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_str(&format!("dataverse-cli/{}", env!("CARGO_PKG_VERSION")))
                .expect("valid header value"),
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            http,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            api_key,
        })
    }

    fn url(&self, service: &str, method: &str) -> String {
        format!("{}/{}/{}", self.base_url, service, method)
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    pub fn dry_run(
        &self,
        service: &str,
        method: &str,
        body: &serde_json::Value,
    ) -> DryRunOutput {
        let mut headers = std::collections::HashMap::new();
        headers.insert(
            "authorization".to_string(),
            format!("Bearer {}", crate::config::Config::mask_key(&self.api_key)),
        );
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("x-client-id".to_string(), "dataverse-rust-cli".to_string());

        DryRunOutput {
            method: "POST".to_string(),
            url: self.url(service, method),
            headers,
            body: body.clone(),
        }
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        service: &str,
        method: &str,
        body: &impl serde::Serialize,
    ) -> Result<T> {
        let url = self.url(service, method);
        let resp = self
            .http
            .post(&url)
            .header(AUTHORIZATION, self.auth_header())
            .json(body)
            .send()
            .await
            .with_context(|| format!("request to {url} failed"))?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            match status.as_u16() {
                401 => bail!("authentication failed: check your API key. {body_text}"),
                464 => {
                    // Macrocosmos custom status: upstream SN13 miner network issue
                    let detail = if body_text.is_empty() {
                        "SN13 miner network temporarily unavailable. Try again in a few seconds.".to_string()
                    } else {
                        body_text
                    };
                    bail!("service unavailable (464): {detail}");
                }
                500 | 502 | 503 | 504 => {
                    let msg = if body_text.is_empty() {
                        "server error".to_string()
                    } else {
                        body_text
                    };
                    bail!("service temporarily unavailable ({status}): {msg}\n  Tip: the SN13 miner network may be busy. Retry in a few seconds.");
                }
                _ => bail!("API error {status}: {body_text}"),
            }
        }

        resp.json::<T>()
            .await
            .with_context(|| format!("failed to parse response from {url}"))
    }

    // ─── SN13 ───────────────────────────────────────────────────────

    pub async fn on_demand_data(
        &self,
        req: &OnDemandDataRequest,
    ) -> Result<OnDemandDataResponse> {
        self.post(SN13_SERVICE, "OnDemandData", req).await
    }

    pub fn on_demand_data_dry_run(&self, req: &OnDemandDataRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(SN13_SERVICE, "OnDemandData", &body))
    }

    // ─── Gravity ────────────────────────────────────────────────────

    pub async fn create_gravity_task(
        &self,
        req: &CreateGravityTaskRequest,
    ) -> Result<CreateGravityTaskResponse> {
        self.post(GRAVITY_SERVICE, "CreateGravityTask", req).await
    }

    pub fn create_gravity_task_dry_run(
        &self,
        req: &CreateGravityTaskRequest,
    ) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(GRAVITY_SERVICE, "CreateGravityTask", &body))
    }

    pub async fn get_gravity_tasks(
        &self,
        req: &GetGravityTasksRequest,
    ) -> Result<GetGravityTasksResponse> {
        self.post(GRAVITY_SERVICE, "GetGravityTasks", req).await
    }

    pub fn get_gravity_tasks_dry_run(
        &self,
        req: &GetGravityTasksRequest,
    ) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(GRAVITY_SERVICE, "GetGravityTasks", &body))
    }

    pub async fn build_dataset(
        &self,
        req: &BuildDatasetRequest,
    ) -> Result<BuildDatasetResponse> {
        self.post(GRAVITY_SERVICE, "BuildDataset", req).await
    }

    pub fn build_dataset_dry_run(&self, req: &BuildDatasetRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(GRAVITY_SERVICE, "BuildDataset", &body))
    }

    pub async fn get_dataset(&self, req: &GetDatasetRequest) -> Result<GetDatasetResponse> {
        self.post(GRAVITY_SERVICE, "GetDataset", req).await
    }

    pub fn get_dataset_dry_run(&self, req: &GetDatasetRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(GRAVITY_SERVICE, "GetDataset", &body))
    }

    pub async fn cancel_gravity_task(&self, task_id: &str) -> Result<CancelResponse> {
        let req = CancelRequest {
            gravity_task_id: Some(task_id.to_string()),
            dataset_id: None,
        };
        self.post(GRAVITY_SERVICE, "CancelGravityTask", &req).await
    }

    pub async fn cancel_dataset(&self, dataset_id: &str) -> Result<CancelResponse> {
        let req = CancelRequest {
            gravity_task_id: None,
            dataset_id: Some(dataset_id.to_string()),
        };
        self.post(GRAVITY_SERVICE, "CancelDataset", &req).await
    }
}
