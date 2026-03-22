//! Native gRPC client for SN13 and Gravity APIs.

use anyhow::{Context, Result};
use std::time::Duration;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, ClientTlsConfig};

use super::types::*;

pub const DEFAULT_BASE_URL: &str = "https://constellation.api.cloud.macrocosmos.ai";

// ─── Generated protobuf modules ─────────────────────────────────────

pub mod sn13_proto {
    tonic::include_proto!("sn13.v1");
}

pub mod gravity_proto {
    tonic::include_proto!("gravity.v1");
}

// ─── Struct → JSON conversion ───────────────────────────────────────

fn struct_to_json(s: &prost_types::Struct) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> = s
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), prost_value_to_json(v)))
        .collect();
    serde_json::Value::Object(map)
}

fn prost_value_to_json(v: &prost_types::Value) -> serde_json::Value {
    match &v.kind {
        Some(prost_types::value::Kind::NullValue(_)) => serde_json::Value::Null,
        Some(prost_types::value::Kind::NumberValue(n)) => {
            if *n == (*n as i64) as f64 && n.is_finite() {
                serde_json::Value::Number(serde_json::Number::from(*n as i64))
            } else {
                serde_json::Number::from_f64(*n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
        }
        Some(prost_types::value::Kind::StringValue(s)) => serde_json::Value::String(s.clone()),
        Some(prost_types::value::Kind::BoolValue(b)) => serde_json::Value::Bool(*b),
        Some(prost_types::value::Kind::StructValue(s)) => struct_to_json(s),
        Some(prost_types::value::Kind::ListValue(l)) => {
            serde_json::Value::Array(l.values.iter().map(prost_value_to_json).collect())
        }
        None => serde_json::Value::Null,
    }
}

// ─── Timestamp helpers ──────────────────────────────────────────────

fn timestamp_to_string(ts: &prost_types::Timestamp) -> String {
    // Convert to RFC 3339 style: YYYY-MM-DDTHH:MM:SSZ
    let secs = ts.seconds;
    // Use chrono-free approach: seconds since epoch -> date string
    // We'll format as ISO 8601
    let dt = std::time::UNIX_EPOCH + Duration::from_secs(secs as u64);
    let datetime: std::time::SystemTime = dt;
    // Format using the humantime crate approach — just produce the timestamp
    // Actually, let's just produce a simple format
    humantime_format(datetime)
}

fn humantime_format(t: std::time::SystemTime) -> String {
    let dur = t
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();

    // Calculate date components
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Days since epoch to Y-M-D (simplified Gregorian)
    let (year, month, day) = days_to_ymd(days as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn days_to_ymd(mut days: i64) -> (i64, i64, i64) {
    // Algorithm from https://howardhinnant.github.io/date_algorithms.html
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u64; // day of era
    let yoe =
        (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as i64, d as i64)
}

/// Parse an ISO 8601 date/datetime string into a prost_types::Timestamp.
fn parse_datetime_to_timestamp(s: &str) -> Option<prost_types::Timestamp> {
    // Handle YYYY-MM-DD format
    if s.len() == 10 {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() == 3 {
            let year: i64 = parts[0].parse().ok()?;
            let month: u32 = parts[1].parse().ok()?;
            let day: u32 = parts[2].parse().ok()?;
            let days = ymd_to_days(year, month, day);
            return Some(prost_types::Timestamp {
                seconds: days * 86400,
                nanos: 0,
            });
        }
    }
    // Handle ISO 8601 with T separator
    if let Some(t_pos) = s.find('T') {
        let date_part = &s[..t_pos];
        let time_part = s[t_pos + 1..].trim_end_matches('Z');
        let parts: Vec<&str> = date_part.split('-').collect();
        if parts.len() == 3 {
            let year: i64 = parts[0].parse().ok()?;
            let month: u32 = parts[1].parse().ok()?;
            let day: u32 = parts[2].parse().ok()?;
            let days = ymd_to_days(year, month, day);

            let time_parts: Vec<&str> = time_part.split(':').collect();
            let hours: i64 = time_parts.first()?.parse().ok()?;
            let minutes: i64 = time_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            let seconds: i64 = time_parts
                .get(2)
                .and_then(|s| s.split('.').next()?.parse().ok())
                .unwrap_or(0);

            return Some(prost_types::Timestamp {
                seconds: days * 86400 + hours * 3600 + minutes * 60 + seconds,
                nanos: 0,
            });
        }
    }
    None
}

fn ymd_to_days(year: i64, month: u32, day: u32) -> i64 {
    // Inverse of days_to_ymd
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 {
        month as i64 + 9
    } else {
        month as i64 - 3
    };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let doy = (153 * m as u64 + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}

// ─── Auth interceptor ───────────────────────────────────────────────

#[derive(Clone)]
struct AuthInterceptor {
    api_key: String,
}

impl tonic::service::Interceptor for AuthInterceptor {
    fn call(
        &mut self,
        mut req: tonic::Request<()>,
    ) -> std::result::Result<tonic::Request<()>, tonic::Status> {
        let val: MetadataValue<_> = format!("Bearer {}", self.api_key)
            .parse()
            .map_err(|_| tonic::Status::internal("invalid api key"))?;
        req.metadata_mut().insert("authorization", val);

        if let Ok(v) = "dataverse-rust-cli".parse() {
            req.metadata_mut().insert("x-client-id", v);
        }

        Ok(req)
    }
}

// ─── Error mapping ──────────────────────────────────────────────────

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
    gravity: gravity_proto::gravity_service_client::GravityServiceClient<InterceptedChannel>,
    api_key: String,
    base_url: String,
}

impl ApiClient {
    pub fn new(api_key: String, base_url: Option<String>, timeout_secs: u64) -> Result<Self> {
        let url = base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        // We can't connect synchronously, so we create a lazy channel.
        // tonic Channel::from_shared + connect_lazy avoids blocking here.
        let tls = ClientTlsConfig::new().with_native_roots();

        let channel = Channel::from_shared(url.clone())
            .context("invalid endpoint URL")?
            .tls_config(tls)
            .context("TLS config failed")?
            .timeout(Duration::from_secs(timeout_secs))
            .connect_lazy();

        let interceptor = AuthInterceptor {
            api_key: api_key.clone(),
        };

        let sn13 = sn13_proto::sn13_service_client::Sn13ServiceClient::with_interceptor(
            channel.clone(),
            interceptor.clone(),
        );

        let gravity =
            gravity_proto::gravity_service_client::GravityServiceClient::with_interceptor(
                channel,
                interceptor,
            );

        Ok(Self {
            sn13,
            gravity,
            api_key,
            base_url: url,
        })
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
        headers.insert("x-client-id".to_string(), "dataverse-rust-cli".to_string());

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

        let response = self
            .sn13
            .clone()
            .on_demand_data(tonic::Request::new(grpc_req))
            .await
            .map_err(map_grpc_error)?;

        let inner = response.into_inner();

        let data: Vec<serde_json::Value> = inner.data.iter().map(struct_to_json).collect();
        let meta = inner.meta.as_ref().map(struct_to_json);

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
        Ok(self.dry_run("sn13.v1.Sn13Service", "OnDemandData", &body))
    }

    // ─── Gravity ─────────────────────────────────────────────────

    pub async fn create_gravity_task(
        &self,
        req: &CreateGravityTaskRequest,
    ) -> Result<CreateGravityTaskResponse> {
        let grpc_tasks: Vec<gravity_proto::GravityTask> = req
            .gravity_tasks
            .iter()
            .map(|t| gravity_proto::GravityTask {
                platform: t.platform.clone(),
                topic: t.topic.clone(),
                keyword: t.keyword.clone(),
                post_start_datetime: t
                    .post_start_datetime
                    .as_ref()
                    .and_then(|s| parse_datetime_to_timestamp(s)),
                post_end_datetime: t
                    .post_end_datetime
                    .as_ref()
                    .and_then(|s| parse_datetime_to_timestamp(s)),
            })
            .collect();

        let grpc_notifications: Vec<gravity_proto::NotificationRequest> = req
            .notification_requests
            .as_ref()
            .map(|nrs| {
                nrs.iter()
                    .map(|n| gravity_proto::NotificationRequest {
                        r#type: n.r#type.clone(),
                        address: n.address.clone(),
                        redirect_url: n.redirect_url.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let grpc_req = gravity_proto::CreateGravityTaskRequest {
            gravity_tasks: grpc_tasks,
            name: req.name.clone().unwrap_or_default(),
            notification_requests: grpc_notifications,
            gravity_task_id: None,
        };

        let response = self
            .gravity
            .clone()
            .create_gravity_task(tonic::Request::new(grpc_req))
            .await
            .map_err(map_grpc_error)?;

        let inner = response.into_inner();
        Ok(CreateGravityTaskResponse {
            gravity_task_id: Some(inner.gravity_task_id),
        })
    }

    pub fn create_gravity_task_dry_run(
        &self,
        req: &CreateGravityTaskRequest,
    ) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(
            "gravity.v1.GravityService",
            "CreateGravityTask",
            &body,
        ))
    }

    pub async fn get_gravity_tasks(
        &self,
        req: &GetGravityTasksRequest,
    ) -> Result<GetGravityTasksResponse> {
        let grpc_req = gravity_proto::GetGravityTasksRequest {
            gravity_task_id: req.gravity_task_id.clone(),
            include_crawlers: req.include_crawlers,
        };

        let response = self
            .gravity
            .clone()
            .get_gravity_tasks(tonic::Request::new(grpc_req))
            .await
            .map_err(map_grpc_error)?;

        let inner = response.into_inner();

        let states: Vec<GravityTaskState> = inner
            .gravity_task_states
            .iter()
            .map(|s| {
                let crawler_workflows: Option<Vec<serde_json::Value>> =
                    if s.crawler_workflows.is_empty() {
                        None
                    } else {
                        Some(
                            s.crawler_workflows
                                .iter()
                                .map(|c| crawler_to_json(c))
                                .collect(),
                        )
                    };

                GravityTaskState {
                    gravity_task_id: if s.gravity_task_id.is_empty() {
                        None
                    } else {
                        Some(s.gravity_task_id.clone())
                    },
                    name: if s.name.is_empty() {
                        None
                    } else {
                        Some(s.name.clone())
                    },
                    status: if s.status.is_empty() {
                        None
                    } else {
                        Some(s.status.clone())
                    },
                    start_time: s.start_time.as_ref().map(timestamp_to_string),
                    crawler_ids: if s.crawler_ids.is_empty() {
                        None
                    } else {
                        Some(s.crawler_ids.clone())
                    },
                    crawler_workflows,
                }
            })
            .collect();

        Ok(GetGravityTasksResponse {
            gravity_task_states: Some(states),
        })
    }

    pub fn get_gravity_tasks_dry_run(
        &self,
        req: &GetGravityTasksRequest,
    ) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run(
            "gravity.v1.GravityService",
            "GetGravityTasks",
            &body,
        ))
    }

    pub async fn build_dataset(
        &self,
        req: &BuildDatasetRequest,
    ) -> Result<BuildDatasetResponse> {
        let grpc_notifications: Vec<gravity_proto::NotificationRequest> = req
            .notification_requests
            .as_ref()
            .map(|nrs| {
                nrs.iter()
                    .map(|n| gravity_proto::NotificationRequest {
                        r#type: n.r#type.clone(),
                        address: n.address.clone(),
                        redirect_url: n.redirect_url.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let grpc_req = gravity_proto::BuildDatasetRequest {
            crawler_id: req.crawler_id.clone(),
            notification_requests: grpc_notifications,
            max_rows: req.max_rows.unwrap_or(10000),
            is_periodic: None,
        };

        let response = self
            .gravity
            .clone()
            .build_dataset(tonic::Request::new(grpc_req))
            .await
            .map_err(map_grpc_error)?;

        let inner = response.into_inner();
        Ok(BuildDatasetResponse {
            dataset_id: Some(inner.dataset_id),
            dataset: inner.dataset.as_ref().map(dataset_to_json),
        })
    }

    pub fn build_dataset_dry_run(&self, req: &BuildDatasetRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run("gravity.v1.GravityService", "BuildDataset", &body))
    }

    pub async fn get_dataset(&self, req: &GetDatasetRequest) -> Result<GetDatasetResponse> {
        let grpc_req = gravity_proto::GetDatasetRequest {
            dataset_id: req.dataset_id.clone(),
        };

        let response = self
            .gravity
            .clone()
            .get_dataset(tonic::Request::new(grpc_req))
            .await
            .map_err(map_grpc_error)?;

        let inner = response.into_inner();

        Ok(GetDatasetResponse {
            dataset: inner.dataset.as_ref().map(proto_dataset_to_dataset_info),
        })
    }

    pub fn get_dataset_dry_run(&self, req: &GetDatasetRequest) -> Result<DryRunOutput> {
        let body = serde_json::to_value(req)?;
        Ok(self.dry_run("gravity.v1.GravityService", "GetDataset", &body))
    }

    pub async fn cancel_gravity_task(&self, task_id: &str) -> Result<CancelResponse> {
        let grpc_req = gravity_proto::CancelGravityTaskRequest {
            gravity_task_id: task_id.to_string(),
        };

        let response = self
            .gravity
            .clone()
            .cancel_gravity_task(tonic::Request::new(grpc_req))
            .await
            .map_err(map_grpc_error)?;

        let inner = response.into_inner();
        Ok(CancelResponse {
            message: Some(inner.message),
        })
    }

    pub async fn cancel_dataset(&self, dataset_id: &str) -> Result<CancelResponse> {
        let grpc_req = gravity_proto::CancelDatasetRequest {
            dataset_id: dataset_id.to_string(),
        };

        let response = self
            .gravity
            .clone()
            .cancel_dataset(tonic::Request::new(grpc_req))
            .await
            .map_err(map_grpc_error)?;

        let inner = response.into_inner();
        Ok(CancelResponse {
            message: Some(inner.message),
        })
    }
}

// ─── Proto → serde type converters ──────────────────────────────────

fn crawler_to_json(c: &gravity_proto::Crawler) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "crawlerId".to_string(),
        serde_json::Value::String(c.crawler_id.clone()),
    );
    if let Some(state) = &c.state {
        let mut state_map = serde_json::Map::new();
        state_map.insert(
            "status".to_string(),
            serde_json::Value::String(state.status.clone()),
        );
        state_map.insert(
            "bytesCollected".to_string(),
            serde_json::Value::Number(serde_json::Number::from(state.bytes_collected)),
        );
        state_map.insert(
            "recordsCollected".to_string(),
            serde_json::Value::Number(serde_json::Number::from(state.records_collected)),
        );
        map.insert("state".to_string(), serde_json::Value::Object(state_map));
    }
    if let Some(criteria) = &c.criteria {
        let mut criteria_map = serde_json::Map::new();
        criteria_map.insert(
            "platform".to_string(),
            serde_json::Value::String(criteria.platform.clone()),
        );
        if let Some(topic) = &criteria.topic {
            criteria_map.insert(
                "topic".to_string(),
                serde_json::Value::String(topic.clone()),
            );
        }
        if let Some(keyword) = &criteria.keyword {
            criteria_map.insert(
                "keyword".to_string(),
                serde_json::Value::String(keyword.clone()),
            );
        }
        map.insert(
            "criteria".to_string(),
            serde_json::Value::Object(criteria_map),
        );
    }
    if let Some(ts) = &c.start_time {
        map.insert(
            "startTime".to_string(),
            serde_json::Value::String(timestamp_to_string(ts)),
        );
    }
    serde_json::Value::Object(map)
}

fn dataset_to_json(d: &gravity_proto::Dataset) -> serde_json::Value {
    serde_json::to_value(proto_dataset_to_dataset_info(d)).unwrap_or(serde_json::Value::Null)
}

fn proto_dataset_to_dataset_info(d: &gravity_proto::Dataset) -> DatasetInfo {
    DatasetInfo {
        crawler_workflow_id: if d.crawler_workflow_id.is_empty() {
            None
        } else {
            Some(d.crawler_workflow_id.clone())
        },
        create_date: d.create_date.as_ref().map(timestamp_to_string),
        expire_date: d.expire_date.as_ref().map(timestamp_to_string),
        files: if d.files.is_empty() {
            None
        } else {
            Some(
                d.files
                    .iter()
                    .map(|f| DatasetFile {
                        file_name: if f.file_name.is_empty() {
                            None
                        } else {
                            Some(f.file_name.clone())
                        },
                        file_size_bytes: Some(f.file_size_bytes as i64),
                        num_rows: Some(f.num_rows as i64),
                        url: if f.url.is_empty() {
                            None
                        } else {
                            Some(f.url.clone())
                        },
                    })
                    .collect(),
            )
        },
        status: if d.status.is_empty() {
            None
        } else {
            Some(d.status.clone())
        },
        status_message: if d.status_message.is_empty() {
            None
        } else {
            Some(d.status_message.clone())
        },
        steps: if d.steps.is_empty() {
            None
        } else {
            Some(
                d.steps
                    .iter()
                    .map(|s| DatasetStep {
                        progress: Some(s.progress),
                        step: Some(serde_json::Value::Number(serde_json::Number::from(
                            s.step,
                        ))),
                        step_name: if s.step_name.is_empty() {
                            None
                        } else {
                            Some(s.step_name.clone())
                        },
                    })
                    .collect(),
            )
        },
        total_steps: Some(d.total_steps),
    }
}
