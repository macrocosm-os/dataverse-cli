pub mod banner;

use anyhow::Result;
use colored::Colorize;
use tabled::{
    settings::{object::Columns, Style, Width},
    Table, Tabled,
};

use crate::api::{DatasetInfo, GravityTaskState};

#[derive(Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}

impl OutputFormat {
    pub fn from_str_opt(s: &str) -> Result<Self> {
        match s {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            _ => anyhow::bail!("invalid output format: {s} (expected: table, json, csv)"),
        }
    }
}

// ─── Social Data Posts ──────────────────────────────────────────────

#[derive(Tabled)]
struct PostRow {
    #[tabled(rename = "Date")]
    date: String,
    #[tabled(rename = "Author")]
    author: String,
    #[tabled(rename = "Text")]
    text: String,
    #[tabled(rename = "Likes")]
    likes: String,
    #[tabled(rename = "Reposts")]
    reposts: String,
    #[tabled(rename = "Replies")]
    replies: String,
    #[tabled(rename = "Views")]
    views: String,
}

fn truncate(s: &str, max: usize) -> String {
    let mut chars = s.chars();
    let collected: String = chars.by_ref().take(max.saturating_sub(3)).collect();
    if chars.next().is_some() {
        format!("{collected}...")
    } else {
        s.to_string()
    }
}

/// Extract a string from a JSON value using a dot-separated path (e.g. "user.username")
fn extract_str(val: &serde_json::Value, path: &str) -> String {
    let mut current = val;
    for key in path.split('.') {
        match current.get(key) {
            Some(v) => current = v,
            None => return "-".to_string(),
        }
    }
    current.as_str().map(|s| s.to_string()).unwrap_or_else(|| {
        // Handle numbers and booleans as strings
        if current.is_null() { "-".to_string() } else { current.to_string() }
    })
}

/// Extract a number from a JSON value using a dot-separated path
fn extract_num(val: &serde_json::Value, path: &str) -> String {
    let mut current = val;
    for key in path.split('.') {
        match current.get(key) {
            Some(v) => current = v,
            None => return "-".to_string(),
        }
    }
    current
        .as_i64()
        .or_else(|| current.as_f64().map(|f| f as i64))
        .map(format_count)
        .unwrap_or_else(|| "-".to_string())
}

fn format_count(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

pub fn print_posts(data: &[serde_json::Value], format: OutputFormat) -> Result<()> {
    if data.is_empty() {
        eprintln!("{}", "no results found".yellow());
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(data)?);
        }
        OutputFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(std::io::stdout());
            wtr.write_record(["date", "author", "text", "likes", "reposts", "replies", "views"])?;
            for post in data {
                let text = extract_str(post, "text")
                    .replace('\n', " ")
                    .replace('\r', "");
                wtr.write_record([
                    &extract_str(post, "datetime"),
                    &extract_str(post, "user.username"),
                    &text,
                    &extract_num(post, "tweet.like_count"),
                    &extract_num(post, "tweet.retweet_count"),
                    &extract_num(post, "tweet.reply_count"),
                    &extract_num(post, "tweet.view_count"),
                ])?;
            }
            wtr.flush()?;
        }
        OutputFormat::Table => {
            let rows: Vec<PostRow> = data
                .iter()
                .map(|post| {
                    let raw_text = extract_str(post, "text")
                        .replace('\n', " ")
                        .replace('\r', "");
                    PostRow {
                        date: extract_str(post, "datetime")
                            .chars()
                            .take(16)
                            .collect(),
                        author: truncate(&extract_str(post, "user.username"), 20),
                        text: truncate(&raw_text, 60),
                        likes: extract_num(post, "tweet.like_count"),
                        reposts: extract_num(post, "tweet.retweet_count"),
                        replies: extract_num(post, "tweet.reply_count"),
                        views: extract_num(post, "tweet.view_count"),
                    }
                })
                .collect();

            let count = rows.len();
            let mut table = Table::new(rows);
            table
                .with(Style::rounded())
                .modify(Columns::new(2..3), Width::wrap(60));

            println!("{table}");
            eprintln!(
                "\n{} {}",
                count.to_string().bold(),
                if count == 1 { "result" } else { "results" }
            );
        }
    }
    Ok(())
}

// ─── Gravity Tasks ──────────────────────────────────────────────────

#[derive(Tabled)]
struct TaskRow {
    #[tabled(rename = "Task ID")]
    id: String,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Started")]
    started: String,
    #[tabled(rename = "Crawlers")]
    crawlers: String,
    #[tabled(rename = "Records")]
    records: String,
    #[tabled(rename = "Size")]
    size: String,
}

/// Extract records_collected and bytes_collected from crawler_workflows
fn extract_crawler_stats(workflows: &Option<Vec<serde_json::Value>>) -> (i64, i64) {
    let mut total_records: i64 = 0;
    let mut total_bytes: i64 = 0;
    if let Some(wfs) = workflows {
        for wf in wfs {
            if let Some(state) = wf.get("state") {
                if let Some(r) = state.get("recordsCollected").and_then(|v| {
                    v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                }) {
                    total_records += r;
                }
                if let Some(b) = state.get("bytesCollected").and_then(|v| {
                    v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                }) {
                    total_bytes += b;
                }
            }
        }
    }
    (total_records, total_bytes)
}

pub fn print_gravity_tasks(tasks: &[GravityTaskState], format: OutputFormat) -> Result<()> {
    if tasks.is_empty() {
        eprintln!("{}", "no gravity tasks found".yellow());
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(tasks)?);
        }
        OutputFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(std::io::stdout());
            wtr.write_record(["task_id", "name", "status", "started", "crawlers"])?;
            for t in tasks {
                wtr.write_record([
                    t.gravity_task_id.as_deref().unwrap_or("-"),
                    t.name.as_deref().unwrap_or("-"),
                    t.status.as_deref().unwrap_or("-"),
                    t.start_time.as_deref().unwrap_or("-"),
                    &t.crawler_ids
                        .as_ref()
                        .map(|c| c.len().to_string())
                        .unwrap_or_else(|| "0".to_string()),
                ])?;
            }
            wtr.flush()?;
        }
        OutputFormat::Table => {
            let rows: Vec<TaskRow> = tasks
                .iter()
                .map(|t| {
                    let status_raw = t.status.as_deref().unwrap_or("Unknown");
                    let status = match status_raw {
                        "Completed" => status_raw.green().to_string(),
                        "Running" | "Submitted" => status_raw.cyan().to_string(),
                        "Failed" | "Cancelled" => status_raw.red().to_string(),
                        "Pending" => status_raw.yellow().to_string(),
                        _ => status_raw.to_string(),
                    };
                    let (records, bytes) = extract_crawler_stats(&t.crawler_workflows);
                    TaskRow {
                        id: truncate(
                            t.gravity_task_id.as_deref().unwrap_or("-"),
                            20,
                        ),
                        name: truncate(t.name.as_deref().unwrap_or("-"), 30),
                        status,
                        started: t
                            .start_time
                            .as_deref()
                            .unwrap_or("-")
                            .chars()
                            .take(16)
                            .collect(),
                        crawlers: t
                            .crawler_ids
                            .as_ref()
                            .map(|c| c.len().to_string())
                            .unwrap_or_else(|| "0".to_string()),
                        records: if records > 0 { format_count(records) } else { "-".to_string() },
                        size: if bytes > 0 { format_bytes(bytes) } else { "-".to_string() },
                    }
                })
                .collect();

            let mut table = Table::new(rows);
            table.with(Style::rounded());
            println!("{table}");
        }
    }
    Ok(())
}

// ─── Dataset ────────────────────────────────────────────────────────

pub fn print_dataset(dataset: &DatasetInfo, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(dataset)?);
        }
        OutputFormat::Csv | OutputFormat::Table => {
            println!(
                "{}  {}",
                "Status:".bold(),
                colorize_status(dataset.status.as_deref().unwrap_or("Unknown"))
            );
            if let Some(msg) = &dataset.status_message {
                if !msg.is_empty() {
                    println!("{}  {msg}", "Message:".bold());
                }
            }
            if let Some(created) = &dataset.create_date {
                println!("{}  {created}", "Created:".bold());
            }
            if let Some(expires) = &dataset.expire_date {
                println!("{}  {expires}", "Expires:".bold());
            }

            // Progress
            if let Some(steps) = &dataset.steps {
                let total = dataset.total_steps.unwrap_or(10) as usize;
                let completed = steps
                    .iter()
                    .filter(|s| s.progress.unwrap_or(0.0) >= 1.0)
                    .count();
                let pct = if total > 0 {
                    (completed as f64 / total as f64 * 100.0) as u64
                } else {
                    0
                };
                println!(
                    "{}  {completed}/{total} steps ({pct}%)",
                    "Progress:".bold()
                );

                for step in steps {
                    let name = step.step_name.as_deref().unwrap_or("?");
                    let prog = step.progress.unwrap_or(0.0);
                    let icon = if prog >= 1.0 { "+" } else if prog > 0.0 { "~" } else { " " };
                    println!("  [{icon}] {name} ({:.0}%)", prog * 100.0);
                }
            }

            // Files
            if let Some(files) = &dataset.files {
                if !files.is_empty() {
                    println!("\n{}:", "Files".bold());
                    for f in files {
                        let name = f.file_name.as_deref().unwrap_or("?");
                        let size = f
                            .file_size_bytes
                            .map(|b| format_bytes(b))
                            .unwrap_or_else(|| "-".to_string());
                        let rows = f
                            .num_rows
                            .map(|r| format!("{r} rows"))
                            .unwrap_or_default();
                        println!("  {name}  ({size}, {rows})");
                        if let Some(url) = &f.url {
                            println!("    {url}");
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn colorize_status(s: &str) -> String {
    match s {
        "Completed" => s.green().to_string(),
        "Running" | "Submitted" | "Processing" => s.cyan().to_string(),
        "Failed" | "Cancelled" => s.red().to_string(),
        "Pending" => s.yellow().to_string(),
        _ => s.to_string(),
    }
}

fn format_bytes(bytes: i64) -> String {
    if bytes < 0 {
        return "unknown".to_string();
    }
    let b = bytes as f64;
    if b >= 1_073_741_824.0 {
        format!("{:.1} GB", b / 1_073_741_824.0)
    } else if b >= 1_048_576.0 {
        format!("{:.1} MB", b / 1_048_576.0)
    } else if b >= 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

// ─── Dry Run ────────────────────────────────────────────────────────

pub fn print_dry_run(dry_run: &crate::api::DryRunOutput) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(dry_run)?);
    Ok(())
}

// ─── Meta ───────────────────────────────────────────────────────────

pub fn print_meta(meta: &serde_json::Value) {
    if let Some(obj) = meta.as_object() {
        let parts: Vec<String> = obj
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        if !parts.is_empty() {
            eprintln!("{} {}", "meta:".dimmed(), parts.join(", ").dimmed());
        }
    }
}
