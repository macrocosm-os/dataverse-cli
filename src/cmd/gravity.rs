use anyhow::{bail, Result};
use clap::{Args, Subcommand};

use crate::api::*;
use crate::config::Config;
use crate::display::{self, OutputFormat};

use super::GlobalOpts;

#[derive(Subcommand)]
pub enum GravityCommands {
    /// Create a new data collection task
    Create(CreateArgs),

    /// List gravity tasks or get status of a specific task
    Status(StatusArgs),

    /// Build a dataset from a crawler
    Build(BuildArgs),

    /// Get dataset status and download links
    Dataset(DatasetArgs),

    /// Cancel a gravity task
    Cancel(CancelArgs),

    /// Cancel a dataset build
    CancelDataset(CancelDatasetArgs),
}

#[derive(Args)]
pub struct CreateArgs {
    /// Platform: x, reddit
    #[arg(short, long, value_parser = parse_platform)]
    pub platform: String,

    /// Topic to track (X: #hashtag or $cashtag; Reddit: r/subreddit)
    #[arg(short, long)]
    pub topic: Option<String>,

    /// Additional keyword filter
    #[arg(short, long)]
    pub keyword: Option<String>,

    /// Task name
    #[arg(short, long)]
    pub name: Option<String>,

    /// Notification email
    #[arg(long)]
    pub email: Option<String>,

    /// Start datetime for posts (ISO 8601)
    #[arg(long)]
    pub from: Option<String>,

    /// End datetime for posts (ISO 8601)
    #[arg(long)]
    pub to: Option<String>,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Gravity task ID (omit to list all tasks)
    #[arg()]
    pub task_id: Option<String>,

    /// Include crawler details
    #[arg(long)]
    pub crawlers: bool,
}

#[derive(Args)]
pub struct BuildArgs {
    /// Crawler ID to build dataset from
    #[arg()]
    pub crawler_id: String,

    /// Maximum rows in dataset
    #[arg(long, default_value = "10000")]
    pub max_rows: i64,

    /// Notification email
    #[arg(long)]
    pub email: Option<String>,
}

#[derive(Args)]
pub struct DatasetArgs {
    /// Dataset ID
    #[arg()]
    pub dataset_id: String,
}

#[derive(Args)]
pub struct CancelArgs {
    /// Gravity task ID to cancel
    #[arg()]
    pub task_id: String,
}

#[derive(Args)]
pub struct CancelDatasetArgs {
    /// Dataset ID to cancel
    #[arg()]
    pub dataset_id: String,
}

fn parse_platform(s: &str) -> Result<String, String> {
    match s.to_lowercase().as_str() {
        "x" | "twitter" => Ok("x".to_string()),
        "reddit" => Ok("reddit".to_string()),
        _ => Err(format!("invalid platform '{s}': expected x, twitter, or reddit")),
    }
}

pub async fn run(cli: &GlobalOpts, cmd: GravityCommands) -> Result<()> {
    let format = OutputFormat::from_str_opt(&cli.output)?;

    match cmd {
        GravityCommands::Create(args) => create(cli, args, format).await,
        GravityCommands::Status(args) => status(cli, args, format).await,
        GravityCommands::Build(args) => build(cli, args).await,
        GravityCommands::Dataset(args) => dataset(cli, args, format).await,
        GravityCommands::Cancel(args) => cancel_task(cli, args).await,
        GravityCommands::CancelDataset(args) => cancel_dataset(cli, args).await,
    }
}

async fn create(cli: &GlobalOpts, args: CreateArgs, _format: OutputFormat) -> Result<()> {
    // Validate topic prefixes
    if let Some(topic) = &args.topic {
        if args.platform == "x" && !topic.starts_with('#') && !topic.starts_with('$') {
            bail!("X topics must start with # (hashtag) or $ (cashtag)");
        }
        if args.platform == "reddit" && !topic.starts_with("r/") {
            bail!("Reddit topics must start with r/");
        }
    }

    let task = GravityTask {
        platform: args.platform,
        topic: args.topic,
        keyword: args.keyword,
        post_start_datetime: args.from,
        post_end_datetime: args.to,
    };

    let notifications = args.email.map(|email| {
        vec![NotificationRequest {
            r#type: "email".to_string(),
            address: email,
            redirect_url: None,
        }]
    });

    let req = CreateGravityTaskRequest {
        gravity_tasks: vec![task],
        name: args.name,
        notification_requests: notifications,
    };

    let api_key = Config::resolve_api_key(&cli.api_key)?;
    let client = ApiClient::new(api_key, cli.base_url.clone(), cli.timeout)?;

    if cli.dry_run {
        let dry = client.create_gravity_task_dry_run(&req)?;
        return display::print_dry_run(&dry);
    }

    let resp = client.create_gravity_task(&req).await?;
    let task_id = resp
        .gravity_task_id
        .unwrap_or_else(|| "unknown".to_string());

    println!("{}", colored::Colorize::green("Gravity task created"));
    println!("  Task ID: {task_id}");
    println!("\n  Check status: dv gravity status {task_id}");
    Ok(())
}

async fn status(cli: &GlobalOpts, args: StatusArgs, format: OutputFormat) -> Result<()> {
    let req = GetGravityTasksRequest {
        gravity_task_id: args.task_id,
        include_crawlers: if args.crawlers { Some(true) } else { None },
    };

    let api_key = Config::resolve_api_key(&cli.api_key)?;
    let client = ApiClient::new(api_key, cli.base_url.clone(), cli.timeout)?;

    if cli.dry_run {
        let dry = client.get_gravity_tasks_dry_run(&req)?;
        return display::print_dry_run(&dry);
    }

    let resp = client.get_gravity_tasks(&req).await?;
    let tasks = resp.gravity_task_states.unwrap_or_default();
    display::print_gravity_tasks(&tasks, format)
}

async fn build(cli: &GlobalOpts, args: BuildArgs) -> Result<()> {
    let notifications = args.email.map(|email| {
        vec![NotificationRequest {
            r#type: "email".to_string(),
            address: email,
            redirect_url: None,
        }]
    });

    let req = BuildDatasetRequest {
        crawler_id: args.crawler_id,
        notification_requests: notifications,
        max_rows: Some(args.max_rows),
    };

    let api_key = Config::resolve_api_key(&cli.api_key)?;
    let client = ApiClient::new(api_key, cli.base_url.clone(), cli.timeout)?;

    if cli.dry_run {
        let dry = client.build_dataset_dry_run(&req)?;
        return display::print_dry_run(&dry);
    }

    let resp = client.build_dataset(&req).await?;
    let dataset_id = resp
        .dataset_id
        .unwrap_or_else(|| "unknown".to_string());

    println!("{}", colored::Colorize::green("Dataset build started"));
    println!("  Dataset ID: {dataset_id}");
    println!("\n  Check status: dv gravity dataset {dataset_id}");
    Ok(())
}

async fn dataset(cli: &GlobalOpts, args: DatasetArgs, format: OutputFormat) -> Result<()> {
    let req = GetDatasetRequest {
        dataset_id: args.dataset_id,
    };

    let api_key = Config::resolve_api_key(&cli.api_key)?;
    let client = ApiClient::new(api_key, cli.base_url.clone(), cli.timeout)?;

    if cli.dry_run {
        let dry = client.get_dataset_dry_run(&req)?;
        return display::print_dry_run(&dry);
    }

    let resp = client.get_dataset(&req).await?;
    match resp.dataset {
        Some(ds) => display::print_dataset(&ds, format),
        None => {
            eprintln!("{}", colored::Colorize::yellow("dataset not found"));
            Ok(())
        }
    }
}

async fn cancel_task(cli: &GlobalOpts, args: CancelArgs) -> Result<()> {
    let api_key = Config::resolve_api_key(&cli.api_key)?;
    let client = ApiClient::new(api_key, cli.base_url.clone(), cli.timeout)?;

    let resp = client.cancel_gravity_task(&args.task_id).await?;
    let msg = resp.message.unwrap_or_else(|| "cancelled".to_string());
    println!("{}: {msg}", colored::Colorize::green("Task cancelled"));
    Ok(())
}

async fn cancel_dataset(cli: &GlobalOpts, args: CancelDatasetArgs) -> Result<()> {
    let api_key = Config::resolve_api_key(&cli.api_key)?;
    let client = ApiClient::new(api_key, cli.base_url.clone(), cli.timeout)?;

    let resp = client.cancel_dataset(&args.dataset_id).await?;
    let msg = resp.message.unwrap_or_else(|| "cancelled".to_string());
    println!("{}: {msg}", colored::Colorize::green("Dataset cancelled"));
    Ok(())
}
