use anyhow::{Context, Result};
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
    #[serde(default = "default_log_format")]
    pub log_format: String,
    #[serde(default = "default_config_watch_method")]
    pub config_watch_method: String, // "inotify" or "polling"
    #[serde(default = "default_config_poll_interval")]
    pub config_poll_interval: u64, // seconds, only used with polling method
}

fn default_config_watch_method() -> String {
    std::env::var("ALERTVIEW_CONFIG_WATCH_METHOD")
        .ok()
        .unwrap_or_else(|| "polling".to_string())
}

fn default_config_poll_interval() -> u64 {
    std::env::var("ALERTVIEW_CONFIG_POLL_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10) // 10 seconds by default
}

fn default_log_format() -> String {
    std::env::var("ALERTVIEW_LOG_FORMAT")
        .ok()
        .unwrap_or_else(|| "text".to_string())
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



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let config = Config::load("config.example").expect("Failed to load config.example");
        assert_eq!(config.port, 8080);
        assert_eq!(config.refresh_interval, 30);
        assert!(!config.tls_insecure);
        assert!(!config.sources.is_empty());
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
        assert_eq!(display.labels, ["namespace", "job"]);
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
        
        assert_eq!(display.labels, ["namespace", "pod"]);
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

    #[test]
    fn test_severity_label_default() {
        let source: Source = serde_yaml::from_str(r#"
            name: test
            type: alertmanager
            url: http://localhost:9093
        "#).expect("Failed to parse source");

        assert_eq!(source.severity_label, "severity");
    }

    #[test]
    fn test_severity_label_custom() {
        let source: Source = serde_yaml::from_str(r#"
            name: test
            type: alertmanager
            url: http://localhost:9093
            severity_label: Severity
        "#).expect("Failed to parse source with severity_label");

        assert_eq!(source.severity_label, "Severity");
    }

    #[test]
    fn test_log_format_default() {
        let config: Config = serde_yaml::from_str("sources: []").expect("Failed to parse config");
        assert_eq!(config.log_format, "text");
    }

    #[test]
    fn test_log_format_json() {
        let config: Config = serde_yaml::from_str("log_format: json\nsources: []").expect("Failed to parse config");
        assert_eq!(config.log_format, "json");
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub name: String,
    #[serde(rename = "type")]
    pub source_type: SourceType,
    pub url: String,
    pub dashboard_url: Option<String>,
    /// Template for the ↗ button ("open in the source"). Falls back to the
    /// alert's own generator URL, then to `dashboard_url`.
    pub link_template: Option<String>,
    /// Template making the whole alert clickable. Overrides
    /// `display.alert_link_template` for this source.
    pub alert_link_template: Option<String>,
    /// Overrides `display.source_link` for this source.
    pub source_link: Option<bool>,
    /// Name of the label used to classify severity (Alertmanager/Grafana only).
    /// Lookup is case-insensitive. Defaults to "severity".
    #[serde(default = "default_severity_label")]
    pub severity_label: String,
    pub basic_auth: Option<BasicAuth>,
    pub bearer_token: Option<String>,
    #[serde(default = "default_source_timeout")]
    pub timeout: u64,
    #[serde(default)]
    pub retry_policy: RetryPolicy,
}

impl Source {
    /// Validate the source configuration
    pub fn validate(&self) -> Result<()> {
        // Validate URL is not empty
        if self.url.is_empty() {
            anyhow::bail!("URL cannot be empty");
        }
        
        // Validate timeout is reasonable
        if self.timeout == 0 {
            anyhow::bail!("timeout cannot be 0");
        }
        
        // Validate retry policy
        if self.retry_policy.initial_delay_ms == 0 {
            anyhow::bail!("initial_delay_ms cannot be 0");
        }
        
        if self.retry_policy.max_delay_ms < self.retry_policy.initial_delay_ms {
            anyhow::bail!("max_delay_ms must be >= initial_delay_ms");
        }
        
        Ok(())
    }
}

fn default_source_timeout() -> u64 {
    15
}

fn default_severity_label() -> String {
    "severity".to_string()
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

#[derive(Debug, Clone, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_labels")]
    pub labels: Vec<String>,
    /// "auto" (follow the OS), "dark" or "light". A URL is still accepted for
    /// backwards compatibility and treated as `custom_css`.
    #[serde(default)]
    pub theme: Option<String>,
    /// URL of an extra stylesheet layered on top of the theme.
    #[serde(default)]
    pub custom_css: Option<String>,
    #[serde(default = "default_timezone")]
    pub timezone: String, // "local", "UTC", or IANA timezone (e.g., "Europe/Paris")
    #[serde(default)]
    pub play_sounds: bool,
    #[serde(default)]
    pub group_by: Vec<String>, // Labels to group alerts by (e.g., ["namespace", "job"])
    /// Severity levels from most to least severe. Any severity not listed here
    /// (including ones a source invents) sorts after every listed level.
    #[serde(default = "default_severity_order")]
    pub severity_order: Vec<String>,
    /// Labels shown in front of the alert name, joined by `prefix_separator`,
    /// in both normal and TV mode. Only the ones the alert carries are shown,
    /// and they are dropped from the trailing label chips so nothing appears
    /// twice. Shown even if absent from `labels`.
    #[serde(default = "default_prefix_labels")]
    pub prefix_labels: Vec<String>,
    #[serde(default = "default_prefix_separator")]
    pub prefix_separator: String,
    /// Start in TV mode when this browser has no stored preference and the URL
    /// says nothing. An explicit choice (the TV button, or `?tv=`) still wins.
    #[serde(default)]
    pub tv_mode_default: bool,
    /// Template making the whole alert clickable, for sources that do not
    /// declare their own. No template means the alert is not clickable.
    #[serde(default)]
    pub alert_link_template: Option<String>,
    /// Show the alert name (the `alertname` label). When false the summary
    /// annotation takes its place, and alerts without a summary keep their name
    /// rather than showing nothing.
    #[serde(default = "default_true")]
    pub show_alert_name: bool,
    /// Show the label chips next to each alert.
    #[serde(default = "default_true")]
    pub show_labels: bool,
    /// Icon marking critical alerts. Empty string disables it.
    #[serde(default = "default_critical_icon")]
    pub critical_icon: String,
    /// Show the ↗ "open in the source" button.
    #[serde(default = "default_true")]
    pub source_link: bool,
    /// Open links in a new tab. False keeps them in the same tab (kiosk).
    #[serde(default = "default_true")]
    pub link_new_tab: bool,
}

// `display:` may be omitted entirely, in which case serde builds the struct
// through `Default` and never sees the per-field defaults above — so `Default`
// has to produce the same values.
impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            labels: default_labels(),
            theme: None,
            custom_css: None,
            timezone: default_timezone(),
            play_sounds: false,
            group_by: Vec::new(),
            severity_order: default_severity_order(),
            prefix_labels: default_prefix_labels(),
            prefix_separator: default_prefix_separator(),
            tv_mode_default: false,
            alert_link_template: None,
            show_alert_name: true,
            show_labels: true,
            critical_icon: default_critical_icon(),
            source_link: true,
            link_new_tab: true,
        }
    }
}

fn default_prefix_labels() -> Vec<String> {
    vec!["hostname".to_string()]
}

fn default_critical_icon() -> String {
    "🔥".to_string()
}

fn default_true() -> bool {
    true
}

fn default_prefix_separator() -> String {
    " / ".to_string()
}

fn default_severity_order() -> Vec<String> {
    ["critical", "error", "high", "warning", "info", "none"]
        .iter()
        .map(|s| s.to_string())
        .collect()
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
        config.validate()?;
        Ok(config)
    }

    pub async fn load_async(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| anyhow::anyhow!("Cannot read {:?}: {}", path, e))?;
        let config: Config = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate port range
        if self.port == 0 {
            anyhow::bail!("Port cannot be 0");
        }
        
        // Validate refresh interval
        if self.refresh_interval == 0 {
            anyhow::bail!("refresh_interval cannot be 0");
        }
        
        // Validate each source
        let mut seen = std::collections::HashSet::new();
        for (i, source) in self.sources.iter().enumerate() {
            source.validate().with_context(|| format!("Invalid configuration for source at index {}", i))?;
            // Names key the alert cache, the announced-fingerprint state and the
            // source filter chips: two sources sharing one would shadow each other.
            if !seen.insert(source.name.to_lowercase()) {
                anyhow::bail!("Duplicate source name {:?} (names must be unique)", source.name);
            }
        }
        
        Ok(())
    }
}

// Type to store config with reload capability
pub type SharedConfig = Arc<RwLock<Config>>;
