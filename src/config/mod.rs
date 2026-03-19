use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl Config {
    pub fn path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("could not determine config directory")?
            .join("dataverse");
        Ok(dir.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        match fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents)
                .with_context(|| format!("failed to parse {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(anyhow::anyhow!("failed to read {}: {e}", path.display())),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, &contents)?;

        // Set 0600 permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    /// Resolve API key from: flag > env > config file
    pub fn resolve_api_key(flag: &Option<String>) -> Result<String> {
        if let Some(key) = flag {
            return Ok(key.clone());
        }
        if let Ok(key) = std::env::var("MC_API") {
            return Ok(key);
        }
        if let Ok(key) = std::env::var("MACROCOSMOS_API_KEY") {
            return Ok(key);
        }
        let config = Self::load()?;
        config
            .api_key
            .context("no API key configured. Run `dv auth` or set MC_API env var")
    }

    pub fn mask_key(key: &str) -> String {
        let chars: Vec<char> = key.chars().collect();
        if chars.len() <= 8 {
            return "****".to_string();
        }
        let prefix: String = chars[..4].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        format!("{prefix}...{suffix}")
    }
}
