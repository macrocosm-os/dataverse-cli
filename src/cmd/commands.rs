use serde::Serialize;

/// Machine-readable command catalog for LLM/agent consumption.
/// Modeled after CoinGecko CLI's `cg commands` pattern.

#[derive(Serialize)]
pub struct CommandCatalog {
    pub version: String,
    pub api_base_url: String,
    pub commands: Vec<CommandInfo>,
}

#[derive(Serialize)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    pub flags: Vec<FlagInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
    pub output_formats: Vec<String>,
    pub requires_auth: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_method: Option<String>,
}

#[derive(Serialize)]
pub struct FlagInfo {
    pub name: String,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    pub description: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub r#enum: Vec<String>,
}

pub fn build_catalog() -> CommandCatalog {
    CommandCatalog {
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_base_url: "https://constellation.api.cloud.macrocosmos.ai".to_string(),
        commands: vec![
            CommandInfo {
                name: "search".to_string(),
                description: "Search social media posts from X/Twitter or Reddit in real-time via the Bittensor SN13 decentralized data network".to_string(),
                flags: vec![
                    FlagInfo {
                        name: "source".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Data source platform".to_string(),
                        required: true,
                        r#enum: vec!["x".into(), "twitter".into(), "reddit".into()],
                    },
                    FlagInfo {
                        name: "--keywords / -k".to_string(),
                        r#type: "string[]".to_string(),
                        default: None,
                        description: "Keywords to search for, comma-separated (up to 5). For Reddit, first item should be subreddit like r/MachineLearning".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                    FlagInfo {
                        name: "--usernames / -u".to_string(),
                        r#type: "string[]".to_string(),
                        default: None,
                        description: "Usernames to filter by, comma-separated (up to 5, X only). @ prefix optional".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                    FlagInfo {
                        name: "--from".to_string(),
                        r#type: "string".to_string(),
                        default: Some("24h ago".into()),
                        description: "Start date (YYYY-MM-DD or ISO 8601)".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                    FlagInfo {
                        name: "--to".to_string(),
                        r#type: "string".to_string(),
                        default: Some("now".into()),
                        description: "End date (YYYY-MM-DD or ISO 8601)".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                    FlagInfo {
                        name: "--limit / -l".to_string(),
                        r#type: "int".to_string(),
                        default: Some("100".into()),
                        description: "Maximum results (1-1000)".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                    FlagInfo {
                        name: "--mode".to_string(),
                        r#type: "string".to_string(),
                        default: Some("any".into()),
                        description: "Keyword match mode".to_string(),
                        required: false,
                        r#enum: vec!["any".into(), "all".into()],
                    },
                    FlagInfo {
                        name: "--url".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Search by URL instead of keywords (X or YouTube URLs)".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                ],
                examples: vec![
                    "dv search x -k bittensor -l 10".into(),
                    "dv search x -k bitcoin,ethereum --from 2026-01-01 -l 50".into(),
                    "dv search x -u @elonmusk -l 20".into(),
                    "dv search x -k bittensor,tao --mode all -l 10".into(),
                    "dv search reddit -k r/MachineLearning -l 25".into(),
                    "dv -o json search x -k ai -l 100".into(),
                    "dv -o csv search x -k crypto -l 500 > data.csv".into(),
                ],
                output_formats: vec!["table".into(), "json".into(), "csv".into()],
                requires_auth: true,
                api_service: Some("sn13.v1.Sn13Service".into()),
                api_method: Some("OnDemandData".into()),
            },
            CommandInfo {
                name: "gravity create".to_string(),
                description: "Create a Gravity large-scale data collection task on the Bittensor SN13 network. Miners continuously collect social data matching your criteria for up to 7 days. Use `dv gravity status --crawlers` to monitor progress.".to_string(),
                flags: vec![
                    FlagInfo {
                        name: "--platform / -p".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Platform to collect from".to_string(),
                        required: true,
                        r#enum: vec!["x".into(), "twitter".into(), "reddit".into()],
                    },
                    FlagInfo {
                        name: "--topic / -t".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Topic to track. X: must start with # or $. Reddit: must start with r/".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                    FlagInfo {
                        name: "--keyword / -k".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Additional keyword filter within the topic".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                    FlagInfo {
                        name: "--name / -n".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Task name".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                    FlagInfo {
                        name: "--email".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Notification email when task completes".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                ],
                examples: vec![
                    "dv gravity create -p x -t '#bittensor' -n 'TAO tracker'".into(),
                    "dv gravity create -p reddit -t 'r/MachineLearning' -k 'transformer'".into(),
                    "dv gravity create -p x -t '$BTC' --email me@example.com".into(),
                ],
                output_formats: vec!["table".into()],
                requires_auth: true,
                api_service: Some("gravity.v1.GravityService".into()),
                api_method: Some("CreateGravityTask".into()),
            },
            CommandInfo {
                name: "gravity status".to_string(),
                description: "List all Gravity tasks or get detailed status of a specific task. IMPORTANT: Always use `dv gravity status --crawlers` (no task_id) to list ALL tasks with record counts and sizes. Pass a task_id only when you already know the exact ID. Always include --crawlers to see Records and Size columns.".to_string(),
                flags: vec![
                    FlagInfo {
                        name: "task_id".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Gravity task ID. OMIT this to list ALL tasks for the user. Only pass a specific ID when you already know it.".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                    FlagInfo {
                        name: "--crawlers".to_string(),
                        r#type: "bool".to_string(),
                        default: Some("false".into()),
                        description: "Include crawler details (records collected, bytes collected). ALWAYS use this flag — without it, Records and Size columns show as empty.".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                ],
                examples: vec![
                    "dv gravity status --crawlers".into(),
                    "dv gravity status multicrawler-abc123 --crawlers".into(),
                    "dv -o json gravity status --crawlers".into(),
                ],
                output_formats: vec!["table".into(), "json".into(), "csv".into()],
                requires_auth: true,
                api_service: Some("gravity.v1.GravityService".into()),
                api_method: Some("GetGravityTasks".into()),
            },
            CommandInfo {
                name: "gravity build".to_string(),
                description: "Build a downloadable dataset from a crawler. WARNING: this stops the crawler and deregisters it from the network".to_string(),
                flags: vec![
                    FlagInfo {
                        name: "crawler_id".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Crawler ID to build dataset from".to_string(),
                        required: true,
                        r#enum: vec![],
                    },
                    FlagInfo {
                        name: "--max-rows".to_string(),
                        r#type: "int".to_string(),
                        default: Some("10000".into()),
                        description: "Maximum rows in the dataset".to_string(),
                        required: false,
                        r#enum: vec![],
                    },
                ],
                examples: vec![
                    "dv gravity build crawler-0-multicrawler-abc123".into(),
                    "dv gravity build crawler-0-multicrawler-abc123 --max-rows 50000".into(),
                ],
                output_formats: vec!["table".into()],
                requires_auth: true,
                api_service: Some("gravity.v1.GravityService".into()),
                api_method: Some("BuildDataset".into()),
            },
            CommandInfo {
                name: "gravity dataset".to_string(),
                description: "Get dataset build status, progress steps, and download links for completed datasets (Parquet format)".to_string(),
                flags: vec![
                    FlagInfo {
                        name: "dataset_id".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Dataset ID".to_string(),
                        required: true,
                        r#enum: vec![],
                    },
                ],
                examples: vec![
                    "dv gravity dataset dataset-abc123".into(),
                    "dv -o json gravity dataset dataset-abc123".into(),
                ],
                output_formats: vec!["table".into(), "json".into()],
                requires_auth: true,
                api_service: Some("gravity.v1.GravityService".into()),
                api_method: Some("GetDataset".into()),
            },
            CommandInfo {
                name: "gravity cancel".to_string(),
                description: "Cancel a running Gravity data collection task".to_string(),
                flags: vec![
                    FlagInfo {
                        name: "task_id".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Gravity task ID to cancel".to_string(),
                        required: true,
                        r#enum: vec![],
                    },
                ],
                examples: vec!["dv gravity cancel multicrawler-abc123".into()],
                output_formats: vec!["table".into()],
                requires_auth: true,
                api_service: Some("gravity.v1.GravityService".into()),
                api_method: Some("CancelGravityTask".into()),
            },
            CommandInfo {
                name: "gravity cancel-dataset".to_string(),
                description: "Cancel a dataset build in progress".to_string(),
                flags: vec![
                    FlagInfo {
                        name: "dataset_id".to_string(),
                        r#type: "string".to_string(),
                        default: None,
                        description: "Dataset ID to cancel".to_string(),
                        required: true,
                        r#enum: vec![],
                    },
                ],
                examples: vec!["dv gravity cancel-dataset dataset-abc123".into()],
                output_formats: vec!["table".into()],
                requires_auth: true,
                api_service: Some("gravity.v1.GravityService".into()),
                api_method: Some("CancelDataset".into()),
            },
            CommandInfo {
                name: "auth".to_string(),
                description: "Interactively configure and validate your Macrocosmos API key. Get a free key at https://app.macrocosmos.ai/account?tab=api-keys".to_string(),
                flags: vec![],
                examples: vec!["dv auth".into()],
                output_formats: vec!["table".into()],
                requires_auth: false,
                api_service: None,
                api_method: None,
            },
            CommandInfo {
                name: "status".to_string(),
                description: "Check configured API key source and test connection to the SN13 network".to_string(),
                flags: vec![],
                examples: vec!["dv status".into()],
                output_formats: vec!["table".into()],
                requires_auth: true,
                api_service: None,
                api_method: None,
            },
        ],
    }
}

pub fn run_commands() {
    let catalog = build_catalog();
    let json = serde_json::to_string_pretty(&catalog).expect("serialize catalog");
    println!("{json}");
}
