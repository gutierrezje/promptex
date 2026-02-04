use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub github: GitHubConfig,

    #[serde(default)]
    pub defaults: DefaultsConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GitHubConfig {
    pub token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_shallow_clone")]
    pub shallow_clone: bool,

    #[serde(default = "default_pr_limit")]
    pub pr_limit: usize,
}

fn default_shallow_clone() -> bool {
    true
}

fn default_pr_limit() -> usize {
    50
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            shallow_clone: default_shallow_clone(),
            pr_limit: default_pr_limit(),
        }
    }
}

impl Config {
    /// Load configuration from ~/.issuance/config.toml
    /// Creates default config if file doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)
            .context("Failed to read config file")?;

        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;

        Ok(config)
    }

    /// Get path to config file (~/.issuance/config.toml)
    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .context("Could not determine home directory")?;

        let config_dir = home.join(".issuance");
        Ok(config_dir.join("config.toml"))
    }

    /// Get path to issuance directory (~/.issuance)
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .context("Could not determine home directory")?;

        Ok(home.join(".issuance"))
    }

    /// Create default config file if it doesn't exist
    pub fn init() -> Result<()> {
        let config_dir = Self::config_dir()?;
        let config_path = Self::config_path()?;

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .context("Failed to create config directory")?;
        }

        // Create default config if it doesn't exist
        if !config_path.exists() {
            let default_config = Self::default();
            let content = toml::to_string_pretty(&default_config)
                .context("Failed to serialize default config")?;

            fs::write(&config_path, content)
                .context("Failed to write default config")?;
        }

        Ok(())
    }
}
