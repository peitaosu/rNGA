//! Configuration management for NGA CLI.

use anyhow::{Context, Result};
use rnga::NGAClient;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// CLI configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    /// Authentication credentials.
    pub auth: Option<AuthConfig>,
}

/// Authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Access token.
    pub token: String,
    /// User ID.
    pub uid: String,
}

/// Get the configuration file path.
pub fn config_path() -> Result<PathBuf> {
    let exe_path = env::current_exe().context("Could not determine executable path")?;
    let exe_dir = exe_path
        .parent()
        .context("Could not determine executable directory")?;

    Ok(exe_dir.join("rnga.toml"))
}

/// Load configuration from file.
pub fn load_config() -> Result<Config> {
    let path = config_path()?;

    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&path).context("Failed to read config file")?;

    toml::from_str(&content).context("Failed to parse config file")
}

/// Save configuration to file.
pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    let content = toml::to_string_pretty(config).context("Failed to serialize config")?;

    fs::write(&path, content).context("Failed to write config file")?;

    Ok(())
}

/// Build an NGA client from the current configuration.
pub fn build_client() -> Result<NGAClient> {
    let config = load_config()?;

    let mut builder = NGAClient::builder();

    if let Some(auth) = config.auth {
        builder = builder.auth(&auth.token, &auth.uid);
    }

    builder.build().context("Failed to build NGA client")
}

/// Build an NGA client that requires authentication.
pub fn build_authed_client() -> Result<NGAClient> {
    let config = load_config()?;

    let auth = config
        .auth
        .context("Authentication required. Run 'rnga auth login' first.")?;

    NGAClient::builder()
        .auth(&auth.token, &auth.uid)
        .build()
        .context("Failed to build NGA client")
}
