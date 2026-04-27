use crate::config::{Source, SourceType};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub fingerprint: String,
    pub source: String,
    pub status: String,
    pub severity: String,
    pub name: String,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub link_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SourceStatus {
    pub name: String,
    pub status: String,
    pub alert_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AlertsResponse {
    pub alerts: Vec<Alert>,
    pub sources: Vec<SourceStatus>,
    pub refresh_interval: u64,
    pub display_labels: Vec<String>,
}

// Alertmanager v2 API wire types
#[derive(Debug, Deserialize)]
struct AmAlert {
    annotations: HashMap<String, String>,
    #[serde(rename = "endsAt")]
    ends_at: String,
    fingerprint: String,
    #[serde(rename = "generatorURL", default)]
    generator_url: String,
    labels: HashMap<String, String>,
    #[serde(rename = "startsAt")]
    starts_at: String,
    status: AmStatus,
}

#[derive(Debug, Deserialize)]
struct AmStatus {
    state: String,
}

pub async fn fetch_source_alerts(client: &reqwest::Client, source: &Source) -> Result<Vec<Alert>> {
    let url = match source.source_type {
        SourceType::Alertmanager => {
            format!("{}/api/v2/alerts", source.url.trim_end_matches('/'))
        }
        SourceType::Grafana => format!(
            "{}/api/alertmanager/grafana/api/v2/alerts",
            source.url.trim_end_matches('/')
        ),
    };

    let mut req = client.get(&url);

    if let Some(auth) = &source.basic_auth {
        req = req.basic_auth(&auth.username, Some(&auth.password));
    }
    if let Some(token) = &source.bearer_token {
        req = req.bearer_auth(token);
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("HTTP {} from {}", resp.status(), url));
    }

    let am_alerts: Vec<AmAlert> = resp.json().await?;

    let alerts = am_alerts
        .into_iter()
        .map(|a| {
            let severity = a
                .labels
                .get("severity")
                .map(|s| s.to_lowercase())
                .unwrap_or_else(|| "none".to_string());

            let name = a
                .labels
                .get("alertname")
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            let status = match a.status.state.as_str() {
                "active" => "firing",
                "suppressed" => "silenced",
                _ => "pending",
            }
            .to_string();

            let ends_at = if a.ends_at.starts_with("0001") {
                None
            } else {
                Some(a.ends_at)
            };

            // dashboard_url from config takes priority over the internal generator_url
            let link_url = source.dashboard_url.clone().or_else(|| {
                if a.generator_url.is_empty() {
                    None
                } else {
                    Some(a.generator_url)
                }
            });

            Alert {
                fingerprint: format!("{}:{}", source.name, a.fingerprint),
                source: source.name.clone(),
                status,
                severity,
                name,
                labels: a.labels,
                annotations: a.annotations,
                starts_at: a.starts_at,
                ends_at,
                link_url,
            }
        })
        .collect();

    Ok(alerts)
}

pub fn severity_order(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "warning" | "warn" => 2,
        "info" | "information" => 3,
        _ => 4,
    }
}
