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

/// Every environment variable AlertView reads. The `--help` text and the
/// documentation test both work from this list, so a variable cannot be added
/// without being documented, nor documented without existing — the old docs
/// invented four and omitted two.
pub const ENV_VARS: &[&str] = &[
    "ALERTVIEW_CONFIG",
    "ALERTVIEW_PORT",
    "ALERTVIEW_REFRESH_INTERVAL",
    "ALERTVIEW_CACHE_TTL",
    "ALERTVIEW_LOG_FORMAT",
    "ALERTVIEW_CONFIG_WATCH_METHOD",
    "ALERTVIEW_CONFIG_POLL_INTERVAL",
];

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
    /// Show the alert name (the `alertname` label). When false the first of
    /// `title_annotations` the alert carries takes its place, and alerts with
    /// none of them keep their name rather than showing nothing.
    #[serde(default = "default_true")]
    pub show_alert_name: bool,
    /// Annotations tried in order for the title when `show_alert_name` is false.
    /// A single string is accepted as a list of one.
    #[serde(
        default = "default_title_annotations",
        deserialize_with = "string_or_list"
    )]
    pub title_annotations: Vec<String>,
    /// Show the label chips next to each alert.
    #[serde(default = "default_true")]
    pub show_labels: bool,
    /// Icon marking critical alerts: a built-in name (`flame`, `bell-off`,
    /// `hourglass`) or any text, emoji included. Empty string disables it.
    #[serde(default = "default_critical_icon")]
    pub critical_icon: String,
    /// Icon shown instead of a status badge, per alert status. A status absent
    /// from the map (or mapped to "") shows nothing — which is what `firing`
    /// does by default, since it is the norm rather than the exception.
    #[serde(default = "default_status_icons")]
    pub status_icons: std::collections::HashMap<String, String>,
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
            title_annotations: default_title_annotations(),
            show_labels: true,
            critical_icon: default_critical_icon(),
            status_icons: default_status_icons(),
            source_link: true,
            link_new_tab: true,
        }
    }
}

fn default_prefix_labels() -> Vec<String> {
    vec!["hostname".to_string()]
}

// A built-in icon name rather than an emoji: an emoji needs a colour emoji
// font on whatever machine displays the dashboard, and a kiosk or a minimal
// Linux box has none — it draws an empty box instead. Known names are `flame`,
// `bell-off` and `hourglass`; anything else is shown as text, emoji included.
fn default_critical_icon() -> String {
    "flame".to_string()
}

fn default_status_icons() -> std::collections::HashMap<String, String> {
    [("silenced", "bell-off"), ("pending", "hourglass")]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn default_true() -> bool {
    true
}

fn default_title_annotations() -> Vec<String> {
    vec!["summary".to_string()]
}

fn string_or_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(s) => vec![s],
        OneOrMany::Many(v) => v,
    })
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

/// Keys the documentation used to recommend that never existed, and the three
/// that genuinely moved. A finite, hand-written table rather than fuzzy
/// matching: fuzzy matching needs a per-level list of field names, which is one
/// more thing to drift. The index in a path is normalised to `[]` before lookup.
const MIGRATION_HINTS: &[(&str, &str)] = &[
    ("sources[].cache_ttl", "caching is global: use the top-level `cache_ttl_seconds`"),
    ("sources[].tls_insecure", "TLS verification is global: use the top-level `tls_insecure`"),
    ("display.refresh_interval", "use the top-level `refresh_interval`"),
    ("display.sort", "ordering follows `display.severity_order`"),
    ("display.group_sort", "groups are ordered by their most severe alert; see `display.severity_order`"),
    ("display.filters", "not a config option — filter from the search box, the severity and source chips, or a `?q=` URL parameter"),
    ("display.compact_mode", "never existed; TV mode (`display.tv_mode_default`) gives the dense layout"),
    ("display.hide_header", "never existed; use `display.custom_css`"),
    ("display.hide_footer", "never existed; use `display.custom_css`"),
    ("display.severity_colors", "never existed; use `display.custom_css`"),
    ("sources[].api_key", "use `bearer_token`, or `basic_auth` for Alertmanager and Grafana"),
    ("sources[].username", "credentials go under `basic_auth: { username, password }`"),
    ("sources[].password", "credentials go under `basic_auth: { username, password }`"),
];

/// `serde_ignored` reports a sequence element as a dotted segment —
/// `sources.0.cache_ttl`. Both helpers below work from that shape; it was
/// verified against the crate rather than assumed.
///
/// For display: `sources.0.cache_ttl` -> `sources[0].cache_ttl`, which is how
/// one points at a YAML list in prose.
fn pretty_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for segment in path.split('.') {
        if segment.chars().all(|c| c.is_ascii_digit()) && !segment.is_empty() && !out.is_empty() {
            out.push('[');
            out.push_str(segment);
            out.push(']');
        } else {
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(segment);
        }
    }
    out
}

/// For lookup: `sources.0.cache_ttl` -> `sources[].cache_ttl`, so one hint
/// covers every element of a sequence.
fn normalise_indices(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for segment in path.split('.') {
        if segment.chars().all(|c| c.is_ascii_digit()) && !segment.is_empty() && !out.is_empty() {
            out.push_str("[]");
        } else {
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(segment);
        }
    }
    out
}

fn hint_for(path: &str) -> Option<&'static str> {
    let normalised = normalise_indices(path);
    MIGRATION_HINTS
        .iter()
        .find(|(key, _)| *key == normalised)
        .map(|(_, hint)| *hint)
}

impl Config {
    /// The single parse entry point. Unknown keys are captured rather than
    /// dropped: a key AlertView does not understand is almost always a key the
    /// user believed in, and silently ignoring it is how a dashboard ends up
    /// ignoring half of someone's settings without ever saying so.
    ///
    /// `serde_ignored` wraps the deserialiser instead of putting
    /// `deny_unknown_fields` on each struct: no attribute to forget on the next
    /// struct someone adds, and the paths come out complete — `sources[2].cache_ttl`
    /// rather than a bare `cache_ttl` with no idea which source it came from.
    pub fn from_yaml(origin: &str, yaml: &str) -> Result<Self> {
        let mut unknown: Vec<String> = Vec::new();
        let de = serde_yaml::Deserializer::from_str(yaml);
        let config: Config = serde_ignored::deserialize(de, |path| {
            unknown.push(path.to_string());
        })?;

        if !unknown.is_empty() {
            let mut message = format!(
                "{origin}: {} key(s) AlertView does not understand:",
                unknown.len()
            );
            for path in &unknown {
                let shown = pretty_path(path);
                match hint_for(path) {
                    Some(hint) => message.push_str(&format!("\n  • {shown} — {hint}")),
                    None => message.push_str(&format!("\n  • {shown}")),
                }
            }
            message.push_str(
                "\n\nEvery option is listed in config.example and in \
                 docs/configuration/config-file.md. Remove the key or correct it \
                 — it would have been ignored, which is worse than this error.",
            );
            anyhow::bail!(message);
        }

        config.validate()?;
        Ok(config)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Cannot read {:?}: {}", path, e))?;
        Self::from_yaml(&path.display().to_string(), &content)
    }

    pub async fn load_async(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| anyhow::anyhow!("Cannot read {:?}: {}", path, e))?;
        Self::from_yaml(&path.display().to_string(), &content)
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

        // These used to be free-form strings: a typo silently fell through to
        // the other branch (text logs, inotify) instead of saying so.
        if !matches!(self.log_format.as_str(), "text" | "json") {
            anyhow::bail!(
                "log_format must be \"text\" or \"json\", got {:?}",
                self.log_format
            );
        }
        if !matches!(self.config_watch_method.as_str(), "inotify" | "polling") {
            anyhow::bail!(
                "config_watch_method must be \"inotify\" or \"polling\", got {:?}",
                self.config_watch_method
            );
        }
        if self.config_poll_interval == 0 {
            anyhow::bail!("config_poll_interval cannot be 0");
        }

        // An empty order ranks every severity the same, which silently turns
        // off sorting, the group ordering and the filter chips' order.
        if self.display.severity_order.is_empty() {
            anyhow::bail!("display.severity_order cannot be empty");
        }

        if self.sources.is_empty() {
            tracing::warn!("No sources configured: the dashboard will stay empty");
        }

        // Validate each source
        let mut seen = std::collections::HashSet::new();
        for (i, source) in self.sources.iter().enumerate() {
            source
                .validate()
                .with_context(|| format!("Invalid configuration for source at index {}", i))?;
            // Names key the alert cache, the announced-fingerprint state and the
            // source filter chips: two sources sharing one would shadow each other.
            if !seen.insert(source.name.to_lowercase()) {
                anyhow::bail!(
                    "Duplicate source name {:?} (names must be unique)",
                    source.name
                );
            }
        }

        Ok(())
    }
}

// Type to store config with reload capability
pub type SharedConfig = Arc<RwLock<Config>>;

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
        let source: Source = serde_yaml::from_str(
            r#"
            name: test
            type: alertmanager
            url: http://localhost:9093
        "#,
        )
        .expect("Failed to parse source");

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
        let source: Source = serde_yaml::from_str(
            r#"
            name: test
            type: grafana
            url: http://localhost:3000
            timeout: 30
            retry_policy:
              max_retries: 5
              initial_delay_ms: 2000
              max_delay_ms: 60000
        "#,
        )
        .expect("Failed to parse source");

        assert_eq!(source.timeout, 30);
        assert_eq!(source.retry_policy.max_retries, 5);
        assert_eq!(source.retry_policy.initial_delay_ms, 2000);
        assert_eq!(source.retry_policy.max_delay_ms, 60000);
    }

    #[test]
    fn test_display_config_defaults() {
        let display: DisplayConfig =
            serde_yaml::from_str("labels: [namespace, job]").expect("Failed to parse display");
        assert_eq!(display.labels, ["namespace", "job"]);
        assert_eq!(display.theme, None);
        assert_eq!(display.timezone, "local");
        assert!(!display.play_sounds);
        assert_eq!(display.title_annotations, ["summary"]);
    }

    #[test]
    fn test_title_annotations() {
        let display: DisplayConfig =
            serde_yaml::from_str("title_annotations: [description, summary]")
                .expect("Failed to parse display");
        assert_eq!(display.title_annotations, ["description", "summary"]);

        let display: DisplayConfig = serde_yaml::from_str("title_annotations: description")
            .expect("Failed to parse display");
        assert_eq!(display.title_annotations, ["description"]);
    }

    #[test]
    fn test_display_config_custom() {
        let display: DisplayConfig = serde_yaml::from_str(
            r#"
            labels: [namespace, pod]
            theme: dark
            timezone: Europe/Paris
            play_sounds: true
        "#,
        )
        .expect("Failed to parse display");

        assert_eq!(display.labels, ["namespace", "pod"]);
        assert_eq!(display.theme, Some("dark".to_string()));
        assert_eq!(display.timezone, "Europe/Paris");
        assert!(display.play_sounds);
    }

    #[test]
    fn test_link_template_parsing() {
        let source: Source = serde_yaml::from_str(
            r#"
            name: test
            type: alertmanager
            url: http://localhost:9093
            link_template: "https://example.com/alerts?query={{.Labels.alertname}}"
        "#,
        )
        .expect("Failed to parse source with link_template");

        assert_eq!(
            source.link_template,
            Some("https://example.com/alerts?query={{.Labels.alertname}}".to_string())
        );
    }

    #[test]
    fn test_severity_label_default() {
        let source: Source = serde_yaml::from_str(
            r#"
            name: test
            type: alertmanager
            url: http://localhost:9093
        "#,
        )
        .expect("Failed to parse source");

        assert_eq!(source.severity_label, "severity");
    }

    #[test]
    fn test_severity_label_custom() {
        let source: Source = serde_yaml::from_str(
            r#"
            name: test
            type: alertmanager
            url: http://localhost:9093
            severity_label: Severity
        "#,
        )
        .expect("Failed to parse source with severity_label");

        assert_eq!(source.severity_label, "Severity");
    }

    #[test]
    fn test_log_format_default() {
        let config = config_from("sources: []").expect("Failed to parse config");
        assert_eq!(config.log_format, "text");
    }

    // Goes through the real entry point, so the tests exercise the same strict
    // path a user's file does.
    fn config_from(yaml: &str) -> Result<Config> {
        Config::from_yaml("test", yaml)
    }

    #[test]
    fn test_unknown_keys_are_refused_with_their_path() {
        // Every one of these was recommended by the project's own docs. They
        // used to be swallowed in silence: the dashboard ignored half of
        // someone's settings and never said so.
        let err = config_from(
            "sources:\n  - name: a\n    type: alertmanager\n    url: http://x\n    cache_ttl: 30\n\
             display:\n  compact_mode: true\n",
        )
        .expect_err("an unknown key must stop the server");
        let msg = err.to_string();

        // The path locates the key, index included — a bare `cache_ttl` would
        // not say which source it came from.
        assert!(msg.contains("sources[0].cache_ttl"), "{msg}");
        assert!(msg.contains("display.compact_mode"), "{msg}");
        // And the message says what to write instead.
        assert!(msg.contains("cache_ttl_seconds"), "{msg}");
        assert!(msg.contains("tv_mode_default"), "{msg}");
    }

    #[test]
    fn test_normalise_indices() {
        // serde_ignored hands us dotted indices; both shapes come from that.
        assert_eq!(
            normalise_indices("sources.12.cache_ttl"),
            "sources[].cache_ttl"
        );
        assert_eq!(pretty_path("sources.12.cache_ttl"), "sources[12].cache_ttl");
        assert_eq!(
            normalise_indices("display.compact_mode"),
            "display.compact_mode"
        );
        assert_eq!(pretty_path("display.compact_mode"), "display.compact_mode");
    }

    #[test]
    fn test_shipped_files_pass_strict_loading() {
        // config.example is the reference users copy: it must survive its own
        // strictness. The k8s ConfigMap ships in the repo too.
        Config::load("config.example").expect("config.example must load");
        let cm = std::fs::read_to_string("02-configmap.yaml").expect("configmap");
        let doc: serde_yaml::Value = serde_yaml::from_str(&cm).expect("valid yaml");
        let embedded = doc["data"]["config.yaml"]
            .as_str()
            .expect("config.yaml key");
        Config::from_yaml("02-configmap.yaml", embedded).expect("the shipped ConfigMap must load");
    }

    #[test]
    fn test_validate_rejects_unknown_enums() {
        // A typo here used to fall through to the other branch in silence.
        assert!(config_from("sources: []\nlog_format: jsonn").is_err());
        assert!(config_from("sources: []\nconfig_watch_method: inotifty").is_err());
        assert!(config_from("sources: []\nconfig_poll_interval: 0").is_err());
        // An empty order ranks every severity the same, disabling sorting.
        assert!(config_from("sources: []\ndisplay:\n  severity_order: []").is_err());
        // The valid spellings still load.
        assert!(config_from("sources: []\nlog_format: json\nconfig_watch_method: inotify").is_ok());
    }

    #[test]
    fn test_log_format_json() {
        let config = config_from("log_format: json\nsources: []").expect("Failed to parse config");
        assert_eq!(config.log_format, "json");
    }
}
