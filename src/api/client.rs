//! HTTP/JSON client for the Macrocosmos SN13 and Gravity APIs.
//!
//! Both services are called via gRPC-Web JSON transcoding over HTTPS.
//! HTTP/2 is required — the Macrocosmos ALB returns 464 on HTTP/1.1,
//! so reqwest must be built with the `http2` feature.

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::time::Duration;

use super::types::*;

pub const DEFAULT_BASE_URL: &str = "https://constellation.api.cloud.macrocosmos.ai";
const SN13_SERVICE: &str = "sn13.v1.Sn13Service";
const GRAVITY_SERVICE: &str = "gravity.v1.GravityService";
const CLIENT_ID: &str = "dataverse-rust-cli";

// ─── Error classification ───────────────────────────────────────────

/// Map a non-success HTTP response to a user-facing error.
///
/// Auth failures often arrive as HTTP 500 with a JSON body describing the
/// key problem (e.g. `{"api_key_token": "Expired token."}`), so the body is
/// inspected before falling back to status-code-based classification.
fn classify_api_error(status: u16, body: &str) -> anyhow::Error {
    if body.contains("Expired token") {
        return anyhow::anyhow!(
            "authentication failed: your API key has expired.\n  Get a new key at https://app.macrocosmos.ai/account?tab=api-keys and run `dv auth`."
        );
    }
    if status == 401
        || body.contains("API Key")
        || body.contains("api_key")
        || body.contains("authentication failed")
    {
        let detail = if body.is_empty() { "check your API key" } else { body };
        return anyhow::anyhow!(
            "authentication failed: {detail}\n  Check your key at https://app.macrocosmos.ai/account?tab=api-keys or run `dv auth`."
        );
    }
    match status {
        // Macrocosmos ALB status: HTTP/1.1 used where HTTP/2 is required
        464 => anyhow::anyhow!(
            "service unavailable (464): the API requires HTTP/2. {body}"
        ),
        500 | 502 | 503 | 504 => {
            let msg = if body.is_empty() { "server error" } else { body };
            anyhow::anyhow!(
                "service temporarily unavailable ({status}): {msg}\n  Tip: the SN13 miner network may be busy. Retry in a few seconds."
            )
        }
        _ => anyhow::anyhow!("API error {status}: {body}"),
    }
}

// ─── ApiClient ──────────────────────────────────────────────────────

pub struct ApiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(api_key: String, base_url: Option<String>, timeout_secs: u64) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("x-client-id", HeaderValue::from_static(CLIENT_ID));
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static(concat!("dataverse-cli/", env!("CARGO_PKG_VERSION"))),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .context("invalid API key for header")?,
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
            .json(body)
            .send()
            .await
            .with_context(|| format!("request to {url} failed"))?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(classify_api_error(status.as_u16(), &body_text));
        }

        resp.json::<T>()
            .await
            .with_context(|| format!("failed to parse response from {url}"))
    }

    // ─── Dry-run helpers ─────────────────────────────────────────

    fn dry_run(&self, service: &str, method: &str, body: &serde_json::Value) -> DryRunOutput {
        let mut headers = std::collections::HashMap::new();
        headers.insert(
            "authorization".to_string(),
            format!("Bearer {}", crate::config::Config::mask_key(&self.api_key)),
        );
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("x-client-id".to_string(), CLIENT_ID.to_string());

        DryRunOutput {
            method: "POST".to_string(),
            url: self.url(service, method),
            headers,
            body: body.clone(),
        }
    }

    // ─── SN13 ────────────────────────────────────────────────────

    pub async fn on_demand_data(&self, req: &OnDemandDataRequest) -> Result<OnDemandDataResponse> {
        let mut resp: OnDemandDataResponse = self.post(SN13_SERVICE, "OnDemandData", req).await?;
        // The server may omit the status field on success; normalize so
        // downstream checks work.
        if resp.status.as_deref().unwrap_or("").is_empty() {
            resp.status = Some("success".to_string());
        }
        Ok(resp)
    }

    pub fn on_demand_data_dry_run(&self, req: &OnDemandDataRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(SN13_SERVICE, "OnDemandData", &body))
    }

    // ─── Gravity ─────────────────────────────────────────────────

    pub async fn create_gravity_task(
        &self,
        req: &CreateGravityTaskRequest,
    ) -> Result<CreateGravityTaskResponse> {
        self.post(GRAVITY_SERVICE, "CreateGravityTask", req).await
    }

    pub fn create_gravity_task_dry_run(&self, req: &CreateGravityTaskRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(GRAVITY_SERVICE, "CreateGravityTask", &body))
    }

    pub async fn get_gravity_tasks(&self, req: &GetGravityTasksRequest) -> Result<GetGravityTasksResponse> {
        self.post(GRAVITY_SERVICE, "GetGravityTasks", req).await
    }

    pub fn get_gravity_tasks_dry_run(&self, req: &GetGravityTasksRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(GRAVITY_SERVICE, "GetGravityTasks", &body))
    }

    pub async fn build_dataset(&self, req: &BuildDatasetRequest) -> Result<BuildDatasetResponse> {
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
        let req = CancelRequest { gravity_task_id: Some(task_id.to_string()), dataset_id: None };
        self.post(GRAVITY_SERVICE, "CancelGravityTask", &req).await
    }

    pub async fn cancel_dataset(&self, dataset_id: &str) -> Result<CancelResponse> {
        let req = CancelRequest { gravity_task_id: None, dataset_id: Some(dataset_id.to_string()) };
        self.post(GRAVITY_SERVICE, "CancelDataset", &req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_token_classified_as_auth_error() {
        let err = classify_api_error(500, r#"{"message": "Bad request: {\"api_key_token\":\"Expired token.\"}"}"#);
        let msg = err.to_string();
        assert!(msg.contains("expired"), "got: {msg}");
        assert!(msg.contains("dv auth"));
    }

    #[test]
    fn status_401_classified_as_auth_error() {
        let msg = classify_api_error(401, "").to_string();
        assert!(msg.contains("authentication failed"), "got: {msg}");
    }

    #[test]
    fn api_key_body_on_500_classified_as_auth_error() {
        let msg = classify_api_error(500, "Error on validating an API Key").to_string();
        assert!(msg.contains("authentication failed"), "got: {msg}");
    }

    #[test]
    fn plain_500_classified_as_transient() {
        let msg = classify_api_error(500, "upstream timeout").to_string();
        assert!(msg.contains("temporarily unavailable"), "got: {msg}");
        assert!(msg.contains("Retry"));
    }

    #[test]
    fn status_464_mentions_http2() {
        let msg = classify_api_error(464, "").to_string();
        assert!(msg.contains("HTTP/2"), "got: {msg}");
    }

    #[test]
    fn other_statuses_pass_through() {
        let msg = classify_api_error(429, "rate limited").to_string();
        assert!(msg.contains("429"));
        assert!(msg.contains("rate limited"));
    }
}
