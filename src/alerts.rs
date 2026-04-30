use crate::config::{Source, SourceType};
use anyhow::Result;
use chrono::TimeZone;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub fingerprint: String,
    pub source: String,
    pub source_type: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default)]
    pub play_sounds: bool,
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

// ── Zabbix JSON-RPC types ────────────────────────────────────────────────────

#[derive(Serialize)]
struct ZabbixRpcRequest<'a> {
    jsonrpc: &'static str,
    method: &'a str,
    params: serde_json::Value,
    id: u32,
}

#[derive(Deserialize)]
struct ZabbixRpcResponse<T> {
    result: Option<T>,
    error: Option<ZabbixRpcError>,
}

#[derive(Deserialize)]
struct ZabbixRpcError {
    code: i32,
    message: String,
    data: String,
}

#[derive(Deserialize)]
struct ZabbixProblem {
    eventid: String,
    objectid: String, // triggerid
    name: String,
    severity: String, // "0"–"5"
    clock: String,    // Unix timestamp
    r_clock: String,  // "0" if unresolved
    suppressed: String,
    #[serde(default)]
    tags: Vec<ZabbixTag>,
}

#[derive(Deserialize)]
struct ZabbixTag {
    tag: String,
    value: String,
}

#[derive(Deserialize)]
struct ZabbixTrigger {
    triggerid: String,
    #[serde(default)]
    hosts: Vec<ZabbixHostInfo>,
    #[serde(default)]
    groups: Vec<ZabbixGroupInfo>,
}

#[derive(Deserialize)]
struct ZabbixHostInfo {
    name: String,
}

#[derive(Deserialize)]
struct ZabbixGroupInfo {
    name: String,
}

// ── Zabbix helpers ───────────────────────────────────────────────────────────

fn zabbix_severity(s: &str) -> &'static str {
    match s {
        "5" => "critical", // Disaster
        "4" => "high",     // High
        "3" | "2" => "warning", // Average / Warning
        "1" => "info",     // Information
        _ => "none",       // Not classified
    }
}

fn unix_ts_to_iso(ts: &str) -> String {
    ts.parse::<i64>()
        .ok()
        .and_then(|secs| chrono::Utc.timestamp_opt(secs, 0).single())
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| ts.to_string())
}

async fn zabbix_rpc<T>(
    client: &reqwest::Client,
    source: &Source,
    url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let body = ZabbixRpcRequest { jsonrpc: "2.0", method, params, id: 1 };
    let mut req = client.post(url).json(&body);
    if let Some(token) = &source.bearer_token {
        req = req.bearer_auth(token);
    }
    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("HTTP {} from {}", resp.status(), url));
    }
    let rpc: ZabbixRpcResponse<T> = resp.json().await?;
    if let Some(err) = rpc.error {
        return Err(anyhow::anyhow!(
            "Zabbix API error {}: {} — {}",
            err.code,
            err.message,
            err.data
        ));
    }
    rpc.result.ok_or_else(|| anyhow::anyhow!("Zabbix API returned empty result"))
}

async fn fetch_zabbix_alerts(client: &reqwest::Client, source: &Source) -> Result<Vec<Alert>> {
    let api_url = format!("{}/api_jsonrpc.php", source.url.trim_end_matches('/'));

    // Step 1: active problems
    let problems: Vec<ZabbixProblem> = zabbix_rpc(
        client,
        source,
        &api_url,
        "problem.get",
        serde_json::json!({ "output": "extend", "selectTags": "extend" }),
    )
    .await?;

    if problems.is_empty() {
        return Ok(vec![]);
    }

    // Step 2: enrich with hosts + hostgroups via their trigger
    let trigger_ids: Vec<&str> = problems.iter().map(|p| p.objectid.as_str()).collect();
    let triggers: Vec<ZabbixTrigger> = zabbix_rpc(
        client,
        source,
        &api_url,
        "trigger.get",
        serde_json::json!({
            "triggerids": trigger_ids,
            "output": ["triggerid"],
            "selectGroups": ["name"],
            "selectHosts": ["name"]
        }),
    )
    .await?;

    let trigger_map: HashMap<String, ZabbixTrigger> =
        triggers.into_iter().map(|t| (t.triggerid.clone(), t)).collect();

    let alerts = problems
        .into_iter()
        .map(|p| {
            let severity = zabbix_severity(&p.severity).to_string();
            let status = if p.suppressed == "1" { "silenced" } else { "firing" }.to_string();

            let mut labels: HashMap<String, String> = HashMap::new();

            if let Some(trigger) = trigger_map.get(&p.objectid) {
                if let Some(host) = trigger.hosts.first() {
                    labels.insert("host".to_string(), host.name.clone());
                }
                let groups: Vec<String> =
                    trigger.groups.iter().map(|g| g.name.clone()).collect();
                if !groups.is_empty() {
                    labels.insert("hostgroup".to_string(), groups.join(", "));
                }
            }

            // Zabbix tags → labels
            for tag in &p.tags {
                labels.insert(tag.tag.clone(), tag.value.clone());
            }

            let mut annotations: HashMap<String, String> = HashMap::new();
            annotations.insert("summary".to_string(), p.name.clone());

            let ends_at = if p.r_clock != "0" {
                Some(unix_ts_to_iso(&p.r_clock))
            } else {
                None
            };

            // Build direct URL to Zabbix alert using eventid
            // Try: link_template -> dashboard_url -> default zabbix URL
            let link_url = source.link_template.clone().and_then(|t| {
                apply_link_template(&t, &Alert {
                    fingerprint: format!("{}:{}", source.name, p.eventid),
                    source: source.name.clone(),
                    source_type: "zabbix".to_string(),
                    status: status.clone(),
                    severity: severity.clone(),
                    name: p.name.clone(),
                    labels: labels.clone(),
                    annotations: annotations.clone(),
                    starts_at: unix_ts_to_iso(&p.clock),
                    ends_at: ends_at.clone(),
                    link_url: None,
                })
            }).or_else(|| {
                source.dashboard_url.clone().map(|url| {
                    // If dashboard_url already contains eventid filter with filter_set, use it as-is
                    if url.contains("filter_eventid") && url.contains("filter_set=1") {
                        url
                    } else {
                        // Remove any existing query params and rebuild with filter
                        let clean_url = url.split_once('?').map(|(base, _)| base.to_string()).unwrap_or(url);
                        let base = if clean_url.contains("zabbix.php") {
                            clean_url
                        } else if clean_url.ends_with('/') {
                            format!("{}/zabbix.php", clean_url.trim_end_matches('/'))
                        } else {
                            format!("{}/zabbix.php", clean_url)
                        };
                        format!("{}/zabbix.php?action=problem.view&filter_set=1&filter_eventid={}", 
                            base.trim_end_matches("/zabbix.php"), p.eventid)
                    }
                })
            }).or_else(|| {
                Some(format!(
                    "{}/zabbix.php?action=problem.view&filter_set=1&filter_eventid={}",
                    source.url.trim_end_matches('/'),
                    p.eventid
                ))
            });

            Alert {
                fingerprint: format!("{}:{}", source.name, p.eventid),
                source: source.name.clone(),
                source_type: "zabbix".to_string(),
                status,
                severity,
                name: p.name,
                labels,
                annotations,
                starts_at: unix_ts_to_iso(&p.clock),
                ends_at,
                link_url,
            }
        })
        .collect();

    Ok(alerts)
}

// ── Main dispatcher ──────────────────────────────────────────────────────────

pub async fn fetch_source_alerts(client: &reqwest::Client, source: &Source) -> Result<Vec<Alert>> {
    if source.source_type == SourceType::Zabbix {
        return fetch_zabbix_alerts(client, source).await;
    }

    let url = match source.source_type {
        SourceType::Alertmanager => {
            format!("{}/api/v2/alerts", source.url.trim_end_matches('/'))
        }
        SourceType::Grafana => format!(
            "{}/api/alertmanager/grafana/api/v2/alerts",
            source.url.trim_end_matches('/')
        ),
        SourceType::Zabbix => unreachable!(),
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

            // Determine source type string
            let source_type_str = match source.source_type {
                SourceType::Alertmanager => "alertmanager",
                SourceType::Grafana => "grafana",
                SourceType::Zabbix => "zabbix",
            }
            .to_string();

            // dashboard_url from config takes priority over the internal generator_url
            // Then try link_template, then generator_url
            let link_url = source.dashboard_url.clone()
                .or_else(|| source.link_template.clone().and_then(|t| apply_link_template(&t, &Alert {
                    fingerprint: format!("{}:{}", source.name, a.fingerprint),
                    source: source.name.clone(),
                    source_type: source_type_str.clone(),
                    status: status.clone(),
                    severity: severity.clone(),
                    name: name.clone(),
                    labels: a.labels.clone(),
                    annotations: a.annotations.clone(),
                    starts_at: a.starts_at.clone(),
                    ends_at: ends_at.clone(),
                    link_url: None,
                })))
                .or_else(|| {
                    if a.generator_url.is_empty() {
                        None
                    } else {
                        Some(a.generator_url)
                    }
                });

            Alert {
                fingerprint: format!("{}:{}", source.name, a.fingerprint),
                source: source.name.clone(),
                source_type: source_type_str,
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

/// Applique un template de lien avec les variables de l'alerte
pub fn apply_link_template(template: &str, alert: &Alert) -> Option<String> {
    if template.is_empty() {
        return None;
    }
    
    let mut result = template.to_string();
    
    // Remplacer les variables de labels
    for (key, value) in &alert.labels {
        let placeholder = format!("{{{{.Labels.{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    
    // Remplacer les variables d'annotations
    for (key, value) in &alert.annotations {
        let placeholder = format!("{{{{.Annotations.{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    
    // Remplacer les variables standards
    result = result.replace("{{.Id}}", &alert.fingerprint);
    result = result.replace("{{.Fingerprint}}", &alert.fingerprint);
    result = result.replace("{{.Source}}", &alert.source);
    result = result.replace("{{.SourceType}}", &alert.source_type);
    result = result.replace("{{.Status}}", &alert.status);
    result = result.replace("{{.Severity}}", &alert.severity);
    result = result.replace("{{.Name}}", &alert.name);
    result = result.replace("{{.StartsAt}}", &alert.starts_at);
    
    if let Some(ends_at) = &alert.ends_at {
        result = result.replace("{{.EndsAt}}", ends_at);
    }
    
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_alert() -> Alert {
        let labels: HashMap<String, String> = [
            ("alertname".to_string(), "HighCPU".to_string()),
            ("namespace".to_string(), "production".to_string()),
        ].into();
        
        let annotations: HashMap<String, String> = [
            ("summary".to_string(), "CPU is high".to_string()),
        ].into();
        
        Alert {
            fingerprint: "test:123".to_string(),
            source: "test".to_string(),
            source_type: "alertmanager".to_string(),
            status: "firing".to_string(),
            severity: "critical".to_string(),
            name: "HighCPU".to_string(),
            labels,
            annotations,
            starts_at: "2024-01-01T00:00:00Z".to_string(),
            ends_at: None,
            link_url: None,
        }
    }

    #[test]
    fn test_apply_link_template_basic() {
        let alert = create_test_alert();
        
        // Test basic template
        let template = "https://example.com/alerts?query={{.Labels.alertname}}";
        let result = apply_link_template(template, &alert).unwrap();
        assert_eq!(result, "https://example.com/alerts?query=HighCPU");
        
        // Test with namespace
        let template = "https://example.com/ns/{{.Labels.namespace}}/alerts/{{.Labels.alertname}}";
        let result = apply_link_template(template, &alert).unwrap();
        assert_eq!(result, "https://example.com/ns/production/alerts/HighCPU");
    }

    #[test]
    fn test_apply_link_template_annotations() {
        let labels: HashMap<String, String> = HashMap::new();
        let annotations: HashMap<String, String> = [
            ("dashboardUid".to_string(), "abc123".to_string()),
            ("panelId".to_string(), "42".to_string()),
        ].into();
        
        let alert = Alert {
            fingerprint: "test:123".to_string(),
            source: "test".to_string(),
            source_type: "grafana".to_string(),
            status: "firing".to_string(),
            severity: "high".to_string(),
            name: "TestAlert".to_string(),
            labels,
            annotations,
            starts_at: "2024-01-01T00:00:00Z".to_string(),
            ends_at: None,
            link_url: None,
        };
        
        let template = "https://grafana.com/d/{{.Annotations.dashboardUid}}?viewPanel={{.Annotations.panelId}}";
        let result = apply_link_template(template, &alert).unwrap();
        assert_eq!(result, "https://grafana.com/d/abc123?viewPanel=42");
    }

    #[test]
    fn test_apply_link_template_standard_vars() {
        let alert = Alert {
            fingerprint: "source1:abc123".to_string(),
            source: "Alertmanager".to_string(),
            source_type: "alertmanager".to_string(),
            status: "firing".to_string(),
            severity: "critical".to_string(),
            name: "MyAlert".to_string(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
            starts_at: "2024-01-01T12:30:00Z".to_string(),
            ends_at: Some("2024-01-01T13:00:00Z".to_string()),
            link_url: None,
        };
        
        let template = "{{.Source}}/{{.Name}}?severity={{.Severity}}&status={{.Status}}";
        let result = apply_link_template(template, &alert).unwrap();
        assert_eq!(result, "Alertmanager/MyAlert?severity=critical&status=firing");
    }

    #[test]
    fn test_apply_link_template_empty() {
        let alert = create_test_alert();
        let result = apply_link_template("", &alert);
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_link_template_missing_var() {
        let alert = create_test_alert();
        // Variable doesn't exist - should leave placeholder
        let template = "https://example.com/{{.Labels.nonexistent}}";
        let result = apply_link_template(template, &alert).unwrap();
        assert_eq!(result, "https://example.com/{{.Labels.nonexistent}}");
    }
}
