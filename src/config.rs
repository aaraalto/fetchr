use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub keys: Keys,
    #[serde(default)]
    pub defaults: Defaults,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Keys {
    pub gemini: Option<String>,
    pub serper: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Defaults {
    pub limit: usize,
    pub output_dir: String,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            limit: 5,
            output_dir: "./downloads".to_string(),
        }
    }
}

fn config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not find config directory")?
        .join("fetchr");
    Ok(config_dir.join("config.toml"))
}

pub fn load() -> Result<Config> {
    let path = config_path()?;

    let mut config = if path.exists() {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {:?}", path))?;
        toml::from_str(&content).with_context(|| "Failed to parse config file")?
    } else {
        Config::default()
    };

    // Override with environment variables if set
    if config.keys.gemini.is_none() {
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            config.keys.gemini = Some(key);
        }
    }
    if config.keys.serper.is_none() {
        if let Ok(key) = std::env::var("SERPER_API_KEY") {
            config.keys.serper = Some(key);
        }
    }

    Ok(config)
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory {:?}", parent))?;
    }

    let content = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;

    fs::write(&path, &content)
        .with_context(|| format!("Failed to write config to {:?}", path))?;

    // Set restrictive permissions on config file (contains API keys)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, permissions)
            .with_context(|| "Failed to set config file permissions")?;
    }

    Ok(())
}

pub fn set_key(provider: &str, key: &str) -> Result<()> {
    let mut config = load()?;

    match provider.to_lowercase().as_str() {
        "gemini" => config.keys.gemini = Some(key.to_string()),
        "serper" => config.keys.serper = Some(key.to_string()),
        _ => anyhow::bail!("Unknown provider: {}. Use 'gemini' or 'serper'.", provider),
    }

    save(&config)?;
    Ok(())
}

/// Replace home directory with ~ for cleaner display
fn shorten_path(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(relative) = path.strip_prefix(&home) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}

pub fn show() -> Result<()> {
    let path = config_path()?;
    println!("Config file: {}", shorten_path(&path));

    let config = load()?;

    println!("\n[keys]");
    println!(
        "gemini = {}",
        config.keys.gemini.as_ref().map(|_| "***").unwrap_or("(not set)")
    );
    println!(
        "serper = {}",
        config.keys.serper.as_ref().map(|_| "***").unwrap_or("(not set)")
    );

    println!("\n[defaults]");
    println!("limit = {}", config.defaults.limit);
    println!("output_dir = {}", config.defaults.output_dir);

    Ok(())
}
