//! Configuration management for NGA CLI.

use anyhow::{Context, Result};
use rnga::NGAClient;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

pub const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub auth: Option<AuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthConfig {
    pub token: String,
    pub uid: String,
}

pub fn config_dir() -> Result<PathBuf> {
    if let Ok(dir) = env::var("RNGA_CONFIG_DIR") {
        return Ok(PathBuf::from(dir));
    }

    if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg_config).join("rnga"));
    }

    let home = env::var("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".config").join("rnga"))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE_NAME))
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&path).context("Failed to read config file")?;
    toml::from_str(&content).context("Failed to parse config file")
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create config directory")?;
    }

    let content = toml::to_string_pretty(config).context("Failed to serialize config")?;
    fs::write(&path, content).context("Failed to write config file")?;
    Ok(())
}

pub fn build_client() -> Result<NGAClient> {
    let config = load_config()?;
    let mut builder = NGAClient::builder();

    if let Some(auth) = config.auth {
        builder = builder.auth(&auth.token, &auth.uid);
    }

    builder.build().context("Failed to build NGA client")
}

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

pub fn auth_status() -> AuthStatus {
    match load_config() {
        Ok(config) => AuthStatus {
            authenticated: config.auth.is_some(),
            uid: config.auth.as_ref().map(|auth| auth.uid.clone()),
        },
        Err(_) => AuthStatus {
            authenticated: false,
            uid: None,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub uid: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = env::var(key).ok();
            env::set_var(key, value);
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => env::set_var(self.key, value),
                None => env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn test_config_dir_from_xdg() {
        let _lock = env_lock().lock().unwrap();
        let _guard = EnvVarGuard::set("XDG_CONFIG_HOME", "/tmp/rnga-test-config");
        env::remove_var("RNGA_CONFIG_DIR");

        let dir = config_dir().unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/rnga-test-config/rnga"));
    }

    #[test]
    fn test_config_path_uses_config_dir() {
        let _lock = env_lock().lock().unwrap();
        let _guard = EnvVarGuard::set("RNGA_CONFIG_DIR", "/tmp/rnga-custom");
        env::remove_var("XDG_CONFIG_HOME");

        let path = config_path().unwrap();
        assert_eq!(path, Path::new("/tmp/rnga-custom/config.toml"));
    }

    #[test]
    fn test_save_and_load_config() {
        let _lock = env_lock().lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("rnga-config-test-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let _guard = EnvVarGuard::set("RNGA_CONFIG_DIR", temp_dir.to_str().unwrap());

        let config = Config {
            auth: Some(AuthConfig {
                token: "token".into(),
                uid: "123".into(),
            }),
        };

        save_config(&config).unwrap();
        let loaded = load_config().unwrap();
        assert_eq!(loaded, config);

        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_auth_status_without_config_auth() {
        let _lock = env_lock().lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("rnga-auth-status-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let _guard = EnvVarGuard::set("RNGA_CONFIG_DIR", temp_dir.to_str().unwrap());

        let status = auth_status();
        assert!(!status.authenticated);
        assert!(status.uid.is_none());

        fs::remove_dir_all(temp_dir).ok();
    }
}
