use anyhow::Result;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_refresh")]
    pub refresh_interval: u64,
    #[serde(default)]
    pub tls_insecure: bool,
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,
    pub sources: Vec<Source>,
    #[serde(default)]
    pub display: DisplayConfig,

}

fn default_port() -> u16 {
    std::env::var("ALERTVIEW_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080)
}

fn default_refresh() -> u64 {
    std::env::var("ALERTVIEW_REFRESH_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30)
}

fn default_cache_ttl() -> u64 {
    std::env::var("ALERTVIEW_CACHE_TTL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0) // 0 = disabled
}

impl Config {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let config = Config::load("config.example").expect("Failed to load config.example");
        assert_eq!(config.port, 8080);
        assert_eq!(config.refresh_interval, 30);
        assert!(!config.tls_insecure);
        assert!(config.sources.len() > 0);
    }

    #[test]
    fn test_source_defaults() {
        let source: Source = serde_yaml::from_str(r#"
            name: test
            type: alertmanager
            url: http://localhost:9093
        "#).expect("Failed to parse source");
        
        assert_eq!(source.name, "test");
        assert_eq!(source.source_type, SourceType::Alertmanager);
        assert_eq!(source.url, "http://localhost:9093");
        assert_eq!(source.timeout, 15); // default
        // RetryPolicy defaults
        assert_eq!(source.retry_policy.max_retries, 3); // default
        assert_eq!(source.retry_policy.initial_delay_ms, 1000); // default
        assert_eq!(source.retry_policy.max_delay_ms, 30000); // default
    }

    #[test]
    fn test_source_custom_values() {
        let source: Source = serde_yaml::from_str(r#"
            name: test
            type: grafana
            url: http://localhost:3000
            timeout: 30
            retry_policy:
              max_retries: 5
              initial_delay_ms: 2000
              max_delay_ms: 60000
        "#).expect("Failed to parse source");
        
        assert_eq!(source.timeout, 30);
        assert_eq!(source.retry_policy.max_retries, 5);
        assert_eq!(source.retry_policy.initial_delay_ms, 2000);
        assert_eq!(source.retry_policy.max_delay_ms, 60000);
    }

    #[test]
    fn test_display_config_defaults() {
        let display: DisplayConfig = serde_yaml::from_str("labels: [namespace, job]").expect("Failed to parse display");
        assert_eq!(display.labels, vec!["namespace", "job"]);
        assert_eq!(display.theme, None);
        assert_eq!(display.timezone, "local");
        assert!(!display.play_sounds);
    }

    #[test]
    fn test_display_config_custom() {
        let display: DisplayConfig = serde_yaml::from_str(r#"
            labels: [namespace, pod]
            theme: dark
            timezone: Europe/Paris
            play_sounds: true
        "#).expect("Failed to parse display");
        
        assert_eq!(display.labels, vec!["namespace", "pod"]);
        assert_eq!(display.theme, Some("dark".to_string()));
        assert_eq!(display.timezone, "Europe/Paris");
        assert!(display.play_sounds);
    }

    #[test]
    fn test_link_template_parsing() {
        let source: Source = serde_yaml::from_str(r#"
            name: test
            type: alertmanager
            url: http://localhost:9093
            link_template: "https://example.com/alerts?query={{.Labels.alertname}}"
        "#).expect("Failed to parse source with link_template");
        
        assert_eq!(source.link_template, Some("https://example.com/alerts?query={{.Labels.alertname}}".to_string()));
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub name: String,
    #[serde(rename = "type")]
    pub source_type: SourceType,
    pub url: String,
    pub dashboard_url: Option<String>,
    pub link_template: Option<String>,
    pub basic_auth: Option<BasicAuth>,
    pub bearer_token: Option<String>,
    #[serde(default = "default_source_timeout")]
    pub timeout: u64,
    #[serde(default)]
    pub retry_policy: RetryPolicy,
}

fn default_source_timeout() -> u64 {
    15
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetryPolicy {
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    #[serde(default = "default_retry_delay")]
    pub initial_delay_ms: u64,
    #[serde(default = "default_max_delay")]
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            initial_delay_ms: default_retry_delay(),
            max_delay_ms: default_max_delay(),
        }
    }
}

fn default_max_retries() -> usize {
    3
}

fn default_retry_delay() -> u64 {
    1000 // 1 second
}

fn default_max_delay() -> u64 {
    30000 // 30 seconds
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
    #[serde(default)]
    pub theme: Option<String>, // "dark", "light", or custom CSS URL
    #[serde(default = "default_timezone")]
    pub timezone: String, // "local", "UTC", or IANA timezone (e.g., "Europe/Paris")
    #[serde(default)]
    pub play_sounds: bool,
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

fn default_timezone() -> String {
    "local".to_string()
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Cannot read {:?}: {}", path, e))?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub async fn load_async(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| anyhow::anyhow!("Cannot read {:?}: {}", path, e))?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

// Type pour stocker la config avec possibilité de reload
pub type SharedConfig = Arc<RwLock<Config>>;
