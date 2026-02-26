use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
pub struct Config {
    pub vault_path: Option<PathBuf>,
    pub max_interval: Option<u32>,
    pub default_ease: Option<f64>,
    pub link_weight: Option<f64>,
    pub load_balance: Option<bool>,
    pub exclude_dirs: Option<Vec<String>>,
}

impl Config {
    pub fn max_interval(&self) -> u32 {
        self.max_interval.unwrap_or(90)
    }

    pub fn default_ease(&self) -> f64 {
        self.default_ease.unwrap_or(2.5)
    }

    pub fn link_weight(&self) -> f64 {
        self.link_weight.unwrap_or(0.1)
    }

    pub fn load_balance(&self) -> bool {
        self.load_balance.unwrap_or(true)
    }

    pub fn exclude_dirs(&self) -> Vec<String> {
        self.exclude_dirs
            .clone()
            .unwrap_or_else(|| vec![".git".into(), ".obsidian".into(), ".trash".into()])
    }
}

pub fn load_config() -> Result<Config> {
    let config_path = dirs::config_dir()
        .map(|d| d.join("sprout").join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("~/.config/sprout/config.toml"));

    if !config_path.exists() {
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(&config_path)?;
    let config: Config = basic_toml::from_str(&content)?;
    Ok(config)
}

pub fn resolve_vault(
    cli_vault: Option<&PathBuf>,
    config: &Config,
) -> Result<PathBuf> {
    // 1. CLI flag
    if let Some(vault) = cli_vault {
        return Ok(std::fs::canonicalize(vault)?);
    }

    // 2. SPROUT_VAULT env
    if let Ok(env_vault) = std::env::var("SPROUT_VAULT") {
        let path = PathBuf::from(env_vault);
        return Ok(std::fs::canonicalize(path)?);
    }

    // 3. Config file
    if let Some(ref vault_path) = config.vault_path {
        return Ok(std::fs::canonicalize(vault_path)?);
    }

    // 4. Current working directory
    Ok(std::fs::canonicalize(std::env::current_dir()?)?)
}
