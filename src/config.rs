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

#[cfg(test)]
pub fn parse_config(content: &str) -> anyhow::Result<Config> {
    let config: Config = basic_toml::from_str(content)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.max_interval(), 90);
        assert!((config.default_ease() - 2.5).abs() < f64::EPSILON);
        assert!((config.link_weight() - 0.1).abs() < f64::EPSILON);
        assert!(config.load_balance());
        assert_eq!(
            config.exclude_dirs(),
            vec![".git".to_string(), ".obsidian".to_string(), ".trash".to_string()]
        );
    }

    #[test]
    fn test_config_custom_values() {
        let config = Config {
            vault_path: Some(PathBuf::from("/notes")),
            max_interval: Some(180),
            default_ease: Some(3.0),
            link_weight: Some(0.2),
            load_balance: Some(false),
            exclude_dirs: Some(vec!["node_modules".into()]),
        };
        assert_eq!(config.max_interval(), 180);
        assert!((config.default_ease() - 3.0).abs() < f64::EPSILON);
        assert!((config.link_weight() - 0.2).abs() < f64::EPSILON);
        assert!(!config.load_balance());
        assert_eq!(config.exclude_dirs(), vec!["node_modules".to_string()]);
    }

    #[test]
    fn test_parse_empty_toml() {
        let config = parse_config("").unwrap();
        assert_eq!(config.max_interval(), 90);
        assert!(config.vault_path.is_none());
    }

    #[test]
    fn test_parse_full_toml() {
        let toml = r#"
vault_path = "/home/user/notes"
max_interval = 120
default_ease = 2.8
link_weight = 0.15
load_balance = false
exclude_dirs = [".git", "archive"]
"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(config.vault_path, Some(PathBuf::from("/home/user/notes")));
        assert_eq!(config.max_interval(), 120);
        assert!((config.default_ease() - 2.8).abs() < 0.001);
        assert!((config.link_weight() - 0.15).abs() < 0.001);
        assert!(!config.load_balance());
        assert_eq!(
            config.exclude_dirs(),
            vec![".git".to_string(), "archive".to_string()]
        );
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml = r#"max_interval = 60"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(config.max_interval(), 60);
        // Other fields use defaults
        assert!((config.default_ease() - 2.5).abs() < f64::EPSILON);
        assert!(config.load_balance());
    }

    #[test]
    fn test_resolve_vault_cli_flag() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = Config::default();
        let vault = resolve_vault(Some(&dir.path().to_path_buf()), &config).unwrap();
        assert_eq!(vault, std::fs::canonicalize(dir.path()).unwrap());
    }

    #[test]
    fn test_resolve_vault_falls_back_to_cwd() {
        let config = Config::default();
        // No CLI flag, no env, no config → cwd
        let vault = resolve_vault(None, &config).unwrap();
        assert_eq!(vault, std::fs::canonicalize(std::env::current_dir().unwrap()).unwrap());
    }
}
