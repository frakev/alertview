use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_refresh")]
    pub refresh_interval: u64,
    #[serde(default)]
    pub tls_insecure: bool,
    pub sources: Vec<Source>,
    #[serde(default)]
    pub display: DisplayConfig,
}

fn default_port() -> u16 {
    8080
}
fn default_refresh() -> u64 {
    30
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub name: String,
    #[serde(rename = "type")]
    pub source_type: SourceType,
    pub url: String,
    pub dashboard_url: Option<String>,
    pub basic_auth: Option<BasicAuth>,
    pub bearer_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Alertmanager,
    Grafana,
    Zabbix,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DisplayConfig {
    #[serde(default = "default_labels")]
    pub labels: Vec<String>,
}

fn default_labels() -> Vec<String> {
    vec![
        "namespace".to_string(),
        "job".to_string(),
        "instance".to_string(),
        "cluster".to_string(),
        "node".to_string(),
    ]
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Cannot read {:?}: {}", path, e))?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
