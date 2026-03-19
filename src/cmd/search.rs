use anyhow::{bail, Result};
use clap::Args;

use crate::api::OnDemandDataRequest;
use crate::display::{self, OutputFormat};

use super::GlobalOpts;

#[derive(Args)]
pub struct SearchArgs {
    /// Data source: x, reddit
    #[arg(value_parser = parse_source)]
    pub source: String,

    /// Keywords to search for (up to 5, comma-separated)
    #[arg(short, long, value_delimiter = ',')]
    pub keywords: Vec<String>,

    /// Usernames to filter by (up to 5, comma-separated; X only)
    #[arg(short, long, value_delimiter = ',')]
    pub usernames: Vec<String>,

    /// Start date (YYYY-MM-DD or ISO 8601). Defaults to 24h ago
    #[arg(long)]
    pub from: Option<String>,

    /// End date (YYYY-MM-DD or ISO 8601). Defaults to now
    #[arg(long)]
    pub to: Option<String>,

    /// Maximum results (1-1000)
    #[arg(short, long, default_value = "100")]
    pub limit: i64,

    /// Keyword match mode: any (OR) or all (AND)
    #[arg(long, default_value = "any", value_parser = parse_keyword_mode)]
    pub mode: String,

    /// Search by URL instead of keywords (X or YouTube URLs)
    #[arg(long)]
    pub url: Option<String>,
}

fn parse_source(s: &str) -> Result<String, String> {
    match s.to_lowercase().as_str() {
        "x" | "twitter" => Ok("X".to_string()),
        "reddit" => Ok("REDDIT".to_string()),
        _ => Err(format!("invalid source '{s}': expected x, twitter, or reddit")),
    }
}

fn parse_keyword_mode(s: &str) -> Result<String, String> {
    match s.to_lowercase().as_str() {
        "any" | "or" => Ok("any".to_string()),
        "all" | "and" => Ok("all".to_string()),
        _ => Err(format!("invalid mode '{s}': expected any or all")),
    }
}

pub async fn run(cli: &GlobalOpts, args: SearchArgs) -> Result<()> {
    // Validate
    if args.keywords.is_empty() && args.usernames.is_empty() && args.url.is_none() {
        bail!("at least one of --keywords, --usernames, or --url is required");
    }
    if args.keywords.len() > 5 {
        bail!("maximum 5 keywords allowed");
    }
    if args.usernames.len() > 5 {
        bail!("maximum 5 usernames allowed");
    }
    if !args.usernames.is_empty() && args.source == "REDDIT" {
        bail!("--usernames is only supported for X/Twitter");
    }
    if args.limit < 1 || args.limit > 1000 {
        bail!("--limit must be between 1 and 1000");
    }

    let keyword_mode = if args.keywords.is_empty() { None } else { Some(args.mode) };

    let req = OnDemandDataRequest {
        source: args.source,
        keywords: args.keywords,
        usernames: args
            .usernames
            .iter()
            .map(|u| u.trim_start_matches('@').to_string())
            .collect(),
        start_date: args.from,
        end_date: args.to,
        limit: Some(args.limit),
        keyword_mode,
        url: args.url,
    };

    let client = cli.make_client()?;

    if cli.dry_run {
        let dry = client.on_demand_data_dry_run(&req)?;
        return display::print_dry_run(&dry);
    }

    let resp = client.on_demand_data(&req).await?;

    if let Some(status) = &resp.status {
        if status != "success" {
            bail!("API returned status: {status}");
        }
    }

    if let Some(meta) = &resp.meta {
        display::print_meta(meta);
    }

    let format = OutputFormat::from_str_opt(&cli.output)?;
    let data = resp.data.unwrap_or_default();
    display::print_posts(&data, format)
}
