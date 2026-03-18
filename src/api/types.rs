use serde::{Deserialize, Serialize};

// ─── SN13 OnDemandData ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OnDemandDataRequest {
    pub source: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub usernames: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OnDemandDataResponse {
    pub status: Option<String>,
    pub data: Option<Vec<serde_json::Value>>,
    pub meta: Option<serde_json::Value>,
}

// ─── Gravity ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GravityTask {
    pub platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_start_datetime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_end_datetime: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGravityTaskRequest {
    pub gravity_tasks: Vec<GravityTask>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_requests: Option<Vec<NotificationRequest>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRequest {
    pub r#type: String,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGravityTaskResponse {
    pub gravity_task_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetGravityTasksRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gravity_task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_crawlers: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetGravityTasksResponse {
    pub gravity_task_states: Option<Vec<GravityTaskState>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GravityTaskState {
    pub gravity_task_id: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub start_time: Option<String>,
    pub crawler_ids: Option<Vec<String>>,
    pub crawler_workflows: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildDatasetRequest {
    pub crawler_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_requests: Option<Vec<NotificationRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_rows: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildDatasetResponse {
    pub dataset_id: Option<String>,
    #[allow(dead_code)]
    pub dataset: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDatasetRequest {
    pub dataset_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDatasetResponse {
    pub dataset: Option<DatasetInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetInfo {
    pub crawler_workflow_id: Option<String>,
    pub create_date: Option<String>,
    pub expire_date: Option<String>,
    pub files: Option<Vec<DatasetFile>>,
    pub status: Option<String>,
    pub status_message: Option<String>,
    pub steps: Option<Vec<DatasetStep>>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub total_steps: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetFile {
    pub file_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub file_size_bytes: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub num_rows: Option<i64>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetStep {
    pub progress: Option<f64>,
    pub step: Option<serde_json::Value>,
    pub step_name: Option<String>,
}

/// Deserialize a value that may be either an integer or a string containing an integer
fn deserialize_optional_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct OptionalI64Visitor;

    impl<'de> de::Visitor<'de> for OptionalI64Visitor {
        type Value = Option<i64>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an integer, a string containing an integer, or null")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v))
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v as i64))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            if v.is_empty() {
                return Ok(None);
            }
            v.parse::<i64>().map(Some).map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(OptionalI64Visitor)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelRequest {
    pub gravity_task_id: Option<String>,
    pub dataset_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CancelResponse {
    pub message: Option<String>,
}

// ─── Dry Run ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DryRunOutput {
    pub method: String,
    pub url: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: serde_json::Value,
}
