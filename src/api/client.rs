//! Native gRPC client for SN13 and Gravity APIs.

use anyhow::{Context, Result};
use std::time::Duration;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, ClientTlsConfig};

use super::types::*;

pub const DEFAULT_BASE_URL: &str = "https://constellation.api.cloud.macrocosmos.ai";
const SN13_SERVICE: &str = "sn13.v1.Sn13Service";
const GRAVITY_SERVICE: &str = "gravity.v1.GravityService";
const CLIENT_ID: &str = "dataverse-rust-cli";

// ─── Generated protobuf modules ─────────────────────────────────────

pub mod sn13_proto {
    tonic::include_proto!("sn13.v1");
}

// ─── Struct → JSON conversion ───────────────────────────────────────

fn struct_to_json(s: prost_types::Struct) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> = s
        .fields
        .into_iter()
        .map(|(k, v)| (k, prost_value_to_json(v)))
        .collect();
    serde_json::Value::Object(map)
}

fn prost_value_to_json(v: prost_types::Value) -> serde_json::Value {
    match v.kind {
        Some(prost_types::value::Kind::NullValue(_)) => serde_json::Value::Null,
        Some(prost_types::value::Kind::NumberValue(n)) => {
            if n == (n as i64) as f64 && n.is_finite() {
                serde_json::Value::Number(serde_json::Number::from(n as i64))
            } else {
                serde_json::Number::from_f64(n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
        }
        Some(prost_types::value::Kind::StringValue(s)) => serde_json::Value::String(s),
        Some(prost_types::value::Kind::BoolValue(b)) => serde_json::Value::Bool(b),
        Some(prost_types::value::Kind::StructValue(s)) => struct_to_json(s),
        Some(prost_types::value::Kind::ListValue(l)) => {
            serde_json::Value::Array(l.values.into_iter().map(prost_value_to_json).collect())
        }
        None => serde_json::Value::Null,
    }
}


// ─── Auth interceptor ───────────────────────────────────────────────

#[derive(Clone)]
struct AuthInterceptor {
    auth_header: MetadataValue<tonic::metadata::Ascii>,
    client_id: MetadataValue<tonic::metadata::Ascii>,
}

impl AuthInterceptor {
    fn new(api_key: &str) -> Result<Self> {
        let auth_header = format!("Bearer {api_key}")
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid API key for header"))?;
        let client_id = CLIENT_ID
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid client ID"))?;
        Ok(Self { auth_header, client_id })
    }
}

impl tonic::service::Interceptor for AuthInterceptor {
    fn call(
        &mut self,
        mut req: tonic::Request<()>,
    ) -> std::result::Result<tonic::Request<()>, tonic::Status> {
        req.metadata_mut().insert("authorization", self.auth_header.clone());
        req.metadata_mut().insert("x-client-id", self.client_id.clone());
        // Match Python SDK metadata — server may require these
        if let Ok(v) = format!("dataverse-cli/{}", env!("CARGO_PKG_VERSION")).parse() {
            req.metadata_mut().insert("x-client-version", v);
        }
        if let Ok(v) = "dataverse-cli".parse() {
            req.metadata_mut().insert("x-source", v);
        }
        Ok(req)
    }
}

// ─── Error mapping ──────────────────────────────────────────────────

/// The ALB/server sometimes answers a gRPC request with a plain JSON error
/// body (e.g. auth failures come back as HTTP 500 + JSON). tonic then fails
/// with "invalid compression flag: 123" — 123 is '{', the first byte of the
/// JSON. This detects that case so we can fetch the real error message.
fn is_json_over_grpc_error(status: &tonic::Status) -> bool {
    status.message().contains("invalid compression flag")
}

fn map_grpc_error(status: tonic::Status) -> anyhow::Error {
    match status.code() {
        tonic::Code::Unauthenticated => {
            anyhow::anyhow!(
                "authentication failed: check your API key. {}",
                status.message()
            )
        }
        tonic::Code::Unavailable => {
            anyhow::anyhow!(
                "service temporarily unavailable: {}\n  Tip: the SN13 miner network may be busy. Retry in a few seconds.",
                status.message()
            )
        }
        tonic::Code::Internal => {
            anyhow::anyhow!(
                "service temporarily unavailable (internal): {}\n  Tip: the SN13 miner network may be busy. Retry in a few seconds.",
                status.message()
            )
        }
        _ => anyhow::anyhow!("gRPC error ({}): {}", status.code(), status.message()),
    }
}

// ─── Type aliases for intercepted clients ───────────────────────────

type InterceptedChannel =
    tonic::service::interceptor::InterceptedService<Channel, AuthInterceptor>;

// ─── ApiClient ──────────────────────────────────────────────────────

pub struct ApiClient {
    sn13: sn13_proto::sn13_service_client::Sn13ServiceClient<InterceptedChannel>,
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl ApiClient {
    pub fn new(api_key: String, base_url: Option<String>, timeout_secs: u64) -> Result<Self> {
        let url = base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        // SN13: native gRPC (reliable, bypasses ALB transcoding)
        let tls = ClientTlsConfig::new().with_native_roots();
        let channel = Channel::from_shared(url.clone())
            .context("invalid endpoint URL")?
            .tls_config(tls)
            .context("TLS config failed")?
            .timeout(Duration::from_secs(timeout_secs))
            .connect_lazy();

        let interceptor = AuthInterceptor::new(&api_key)?;
        let sn13 = sn13_proto::sn13_service_client::Sn13ServiceClient::with_interceptor(
            channel,
            interceptor,
        )
        .send_compressed(tonic::codec::CompressionEncoding::Gzip)
        .accept_compressed(tonic::codec::CompressionEncoding::Gzip);

        // Gravity: HTTP/JSON (their gRPC endpoint is broken — sends JSON over binary channel)
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::CONTENT_TYPE, reqwest::header::HeaderValue::from_static("application/json"));
        headers.insert("x-client-id", reqwest::header::HeaderValue::from_static(CLIENT_ID));
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {api_key}"))
                .context("invalid API key for header")?,
        );
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            sn13,
            http,
            api_key,
            base_url: url,
        })
    }

    /// HTTP POST for Gravity endpoints (their gRPC is broken server-side).
    async fn gravity_post<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        body: &impl serde::Serialize,
    ) -> Result<T> {
        let url = format!("{}/{}/{}", self.base_url, GRAVITY_SERVICE, method);
        let resp = self.http.post(&url).json(body).send().await
            .with_context(|| format!("request to {url} failed"))?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            match status.as_u16() {
                401 => anyhow::bail!("authentication failed: check your API key. {body_text}"),
                // Auth failures often surface as 500 with an error body
                _ if body_text.contains("Expired token") => anyhow::bail!(
                    "authentication failed: your API key has expired.\n  Get a new key at https://app.macrocosmos.ai/account?tab=api-keys and run `dv auth`."
                ),
                _ if body_text.contains("API Key") || body_text.contains("api_key") => anyhow::bail!(
                    "authentication failed: {body_text}\n  Check your key at https://app.macrocosmos.ai/account?tab=api-keys or run `dv auth`."
                ),
                500 | 502 | 503 | 504 => {
                    let msg = if body_text.is_empty() { "server error".to_string() } else { body_text };
                    anyhow::bail!("service temporarily unavailable ({status}): {msg}\n  Tip: the SN13 miner network may be busy. Retry in a few seconds.");
                }
                _ => anyhow::bail!("API error {status}: {body_text}"),
            }
        }

        resp.json::<T>().await
            .with_context(|| format!("failed to parse response from {url}"))
    }

    /// Replay a failed SN13 request over the HTTP/JSON transcoding endpoint
    /// to recover the real error message the gRPC channel couldn't decode.
    /// Returns None if the replay unexpectedly succeeds or yields no message.
    async fn fetch_json_error(&self, method: &str, body: &impl serde::Serialize) -> Option<anyhow::Error> {
        let url = format!("{}/{}/{}", self.base_url, SN13_SERVICE, method);
        let resp = self.http.post(&url).json(body).send().await.ok()?;
        if resp.status().is_success() {
            return None;
        }
        let v: serde_json::Value = resp.json().await.ok()?;
        let msg = v.get("message")?.as_str()?.to_string();
        if msg.contains("Expired token") {
            Some(anyhow::anyhow!(
                "authentication failed: your API key has expired.\n  Get a new key at https://app.macrocosmos.ai/account?tab=api-keys and run `dv auth`."
            ))
        } else if msg.contains("API Key") || msg.contains("api_key") || msg.contains("authentication failed") {
            Some(anyhow::anyhow!(
                "authentication failed: {msg}\n  Check your key at https://app.macrocosmos.ai/account?tab=api-keys or run `dv auth`."
            ))
        } else {
            Some(anyhow::anyhow!("API error: {msg}"))
        }
    }

    // ─── Dry-run helpers ─────────────────────────────────────────

    fn dry_run(
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
        headers.insert(
            "content-type".to_string(),
            "application/grpc".to_string(),
        );
        headers.insert("x-client-id".to_string(), CLIENT_ID.to_string());

        DryRunOutput {
            method: "gRPC".to_string(),
            url: format!("{}/{}/{}", self.base_url, service, method),
            headers,
            body: body.clone(),
        }
    }

    // ─── SN13 ────────────────────────────────────────────────────

    pub async fn on_demand_data(
        &self,
        req: &OnDemandDataRequest,
    ) -> Result<OnDemandDataResponse> {
        let grpc_req = sn13_proto::OnDemandDataRequest {
            source: req.source.clone(),
            usernames: req.usernames.clone(),
            keywords: req.keywords.clone(),
            start_date: req.start_date.clone(),
            end_date: req.end_date.clone(),
            limit: req.limit,
            keyword_mode: req.keyword_mode.clone(),
            url: req.url.clone(),
        };

        let response = match self
            .sn13
            .clone()
            .on_demand_data(tonic::Request::new(grpc_req))
            .await
        {
            Ok(resp) => resp,
            Err(status) => {
                if is_json_over_grpc_error(&status) {
                    if let Some(err) = self.fetch_json_error("OnDemandData", req).await {
                        return Err(err);
                    }
                }
                return Err(map_grpc_error(status));
            }
        };

        let inner = response.into_inner();

        let data: Vec<serde_json::Value> = inner.data.into_iter().map(struct_to_json).collect();
        let meta = inner.meta.map(struct_to_json);

        // The gRPC server may return an empty status string on success;
        // normalize to "success" so downstream checks work.
        let status = if inner.status.is_empty() {
            "success".to_string()
        } else {
            inner.status
        };

        Ok(OnDemandDataResponse {
            status: Some(status),
            data: Some(data),
            meta,
        })
    }

    pub fn on_demand_data_dry_run(&self, req: &OnDemandDataRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(SN13_SERVICE, "OnDemandData", &body))
    }

    // ─── Gravity (HTTP/JSON — their gRPC endpoint is broken) ────

    pub async fn create_gravity_task(
        &self,
        req: &CreateGravityTaskRequest,
    ) -> Result<CreateGravityTaskResponse> {
        self.gravity_post("CreateGravityTask", req).await
    }

    pub fn create_gravity_task_dry_run(&self, req: &CreateGravityTaskRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(GRAVITY_SERVICE, "CreateGravityTask", &body))
    }

    pub async fn get_gravity_tasks(&self, req: &GetGravityTasksRequest) -> Result<GetGravityTasksResponse> {
        self.gravity_post("GetGravityTasks", req).await
    }

    pub fn get_gravity_tasks_dry_run(&self, req: &GetGravityTasksRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(GRAVITY_SERVICE, "GetGravityTasks", &body))
    }

    pub async fn build_dataset(&self, req: &BuildDatasetRequest) -> Result<BuildDatasetResponse> {
        self.gravity_post("BuildDataset", req).await
    }

    pub fn build_dataset_dry_run(&self, req: &BuildDatasetRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(GRAVITY_SERVICE, "BuildDataset", &body))
    }

    pub async fn get_dataset(&self, req: &GetDatasetRequest) -> Result<GetDatasetResponse> {
        self.gravity_post("GetDataset", req).await
    }

    pub fn get_dataset_dry_run(&self, req: &GetDatasetRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(GRAVITY_SERVICE, "GetDataset", &body))
    }

    pub async fn cancel_gravity_task(&self, task_id: &str) -> Result<CancelResponse> {
        let req = CancelRequest { gravity_task_id: Some(task_id.to_string()), dataset_id: None };
        self.gravity_post("CancelGravityTask", &req).await
    }

    pub async fn cancel_dataset(&self, dataset_id: &str) -> Result<CancelResponse> {
        let req = CancelRequest { gravity_task_id: None, dataset_id: Some(dataset_id.to_string()) };
        self.gravity_post("CancelDataset", &req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_value(kind: prost_types::value::Kind) -> prost_types::Value {
        prost_types::Value { kind: Some(kind) }
    }

    #[test]
    fn prost_numbers_convert_to_integers_when_whole() {
        use prost_types::value::Kind;
        assert_eq!(prost_value_to_json(make_value(Kind::NumberValue(42.0))), serde_json::json!(42));
        assert_eq!(prost_value_to_json(make_value(Kind::NumberValue(1.5))), serde_json::json!(1.5));
    }

    #[test]
    fn prost_struct_converts_to_json_object() {
        use prost_types::value::Kind;
        let s = prost_types::Struct {
            fields: [
                ("name".to_string(), make_value(Kind::StringValue("dv".to_string()))),
                ("ok".to_string(), make_value(Kind::BoolValue(true))),
            ]
            .into_iter()
            .collect(),
        };
        assert_eq!(struct_to_json(s), serde_json::json!({"name": "dv", "ok": true}));
    }

    #[test]
    fn json_over_grpc_error_detected_by_message() {
        let status = tonic::Status::internal(
            "protocol error: received message with invalid compression flag: 123",
        );
        assert!(is_json_over_grpc_error(&status));
        assert!(!is_json_over_grpc_error(&tonic::Status::internal("boom")));
    }
}

