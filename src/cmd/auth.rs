use anyhow::Result;
use colored::Colorize;
use dialoguer::Password;

use crate::api::ApiClient;
use crate::config::Config;

use super::GlobalOpts;

pub async fn run_auth() -> Result<()> {
    println!("{}", "Dataverse CLI - API Key Setup".bold());
    println!();
    println!("Get your free API key at: {}", "https://app.macrocosmos.ai/account?tab=api-keys".cyan());
    println!();

    let key: String = Password::new()
        .with_prompt("API key")
        .interact()?;

    if key.trim().is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    // Test the key
    print!("Validating key... ");
    let client = ApiClient::new(key.clone(), None, 30)?;
    let req = crate::api::OnDemandDataRequest {
        source: "X".to_string(),
        keywords: vec!["test".to_string()],
        usernames: vec![],
        start_date: None,
        end_date: None,
        limit: Some(1),
        keyword_mode: None,
        url: None,
    };

    match client.on_demand_data(&req).await {
        Ok(_) => println!("{}", "valid".green()),
        Err(e) => {
            let msg = format!("{e}");
            if msg.contains("authentication failed") {
                println!("{}", "invalid".red());
                anyhow::bail!("API key is invalid (401). Check your key at https://app.macrocosmos.ai");
            } else {
                // Network/service error — key might still be valid
                println!("{}", "warning".yellow());
                eprintln!(
                    "  Could not verify key (network issue): {e}\n  Saving anyway — the SN13 network may be temporarily unavailable."
                );
            }
        }
    }

    // Save
    let mut config = Config::load()?;
    config.api_key = Some(key);
    config.save()?;

    let path = Config::path()?;
    println!("\n{} Saved to {}", "Done!".green().bold(), path.display());

    // Suggest env var alternative
    println!("\n{}", "Tip:".dimmed());
    println!(
        "{}",
        "  You can also set the MC_API environment variable instead.".dimmed()
    );

    Ok(())
}

pub async fn run_status(cli: &GlobalOpts) -> Result<()> {
    let key = Config::resolve_api_key(&cli.api_key)?;
    let masked = Config::mask_key(&key);

    println!("{}  {masked}", "API Key:".bold());

    // Source
    if cli.api_key.is_some() {
        println!("{}  --api-key flag", "Source:".bold());
    } else if std::env::var("MC_API").is_ok() {
        println!("{}  MC_API env var", "Source:".bold());
    } else if std::env::var("MACROCOSMOS_API_KEY").is_ok() {
        println!("{}  MACROCOSMOS_API_KEY env var", "Source:".bold());
    } else {
        let path = Config::path()?;
        println!("{}  {}", "Source:".bold(), path.display());
    }

    // Test connection
    print!("\n{} ", "Testing connection...".dimmed());
    let client = ApiClient::new(key, cli.base_url.clone(), 30)?;
    let req = crate::api::OnDemandDataRequest {
        source: "X".to_string(),
        keywords: vec!["test".to_string()],
        usernames: vec![],
        start_date: None,
        end_date: None,
        limit: Some(1),
        keyword_mode: None,
        url: None,
    };

    match client.on_demand_data(&req).await {
        Ok(_) => println!("{}", "connected".green().bold()),
        Err(e) => {
            println!("{}", "failed".red().bold());
            eprintln!("  {e}");
        }
    }

    Ok(())
}
