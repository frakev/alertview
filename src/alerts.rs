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
    #[serde(default)]
    pub groups: Vec<AlertGroup>,
    #[serde(default)]
    pub group_by: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AlertGroup {
    pub key: String,
    pub labels: std::collections::HashMap<String, String>,
    pub alerts: Vec<Alert>,
    pub count: usize,
    pub severity_counts: std::collections::HashMap<String, usize>,
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
    #[serde(default)]
    silenced_by: Vec<String>, // List of silence IDs that silenced this alert
}

#[derive(Debug, Deserialize)]
struct AmSilence {
    id: String,
    #[serde(rename = "createdBy")]
    #[allow(dead_code)]
    created_by: String,
    comment: String,
    // Other fields we don't need for now
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
    #[serde(default = "default_acknowledged", deserialize_with = "deserialize_acknowledged")]
    acknowledged: String, // "0" or "1" or true/false - whether problem is acknowledged
    #[serde(default)]
    acknowledgements: Vec<ZabbixAcknowledgement>, // ACK details from Zabbix
    #[serde(default)]
    tags: Vec<ZabbixTag>,
}

// Handle both string ("0"/"1") and boolean (true/false) for acknowledged field
fn deserialize_acknowledged<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AcknowledgedValue {
        String(String),
        Bool(bool),
    }
    
    match AcknowledgedValue::deserialize(deserializer)? {
        AcknowledgedValue::String(s) => Ok(s),
        AcknowledgedValue::Bool(b) => Ok(if b { "1".to_string() } else { "0".to_string() }),
    }
}

fn default_acknowledged() -> String {
    "0".to_string()
}

#[derive(Deserialize)]
struct ZabbixAcknowledgement {
    #[serde(rename = "acknowledgeid")]
    #[allow(dead_code)]
    id: String,
    #[serde(rename = "useralias")]
    user: String,
    #[serde(rename = "clock")]
    timestamp: String,
    #[serde(alias = "message", alias = "comment")]
    message: Option<String>, // The acknowledgment comment/message (Zabbix may use message or comment)
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
    status: String, // "0" = enabled, "1" = disabled
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

    // Step 1: active problems (with acknowledgements to get ACK messages)
    // Try selectAcknowledgements first (Zabbix 6.0+), fall back to select_acknowledges (Zabbix 5.x)
    // If both fail, try without acknowledgement selection (older versions)
    let problems: Vec<ZabbixProblem> = match zabbix_rpc(
        client,
        source,
        &api_url,
        "problem.get",
        serde_json::json!({ 
            "output": "extend", 
            "selectTags": "extend",
            "selectAcknowledgements": true 
        }),
    ).await {
        Ok(p) => p,
        Err(_) => {
            // Fallback for older Zabbix versions
            match zabbix_rpc(
                client,
                source,
                &api_url,
                "problem.get",
                serde_json::json!({ 
                    "output": "extend", 
                    "selectTags": "extend",
                    "select_acknowledges": true 
                }),
            ).await {
                Ok(p) => p,
                Err(_) => {
                    // Fallback to basic query without acknowledgements
                    zabbix_rpc(
                        client,
                        source,
                        &api_url,
                        "problem.get",
                        serde_json::json!({ 
                            "output": "extend", 
                            "selectTags": "extend"
                        }),
                    ).await?
                }
            }
        }
    };

    if problems.is_empty() {
        tracing::debug!("No Zabbix problems found");
        return Ok(Vec::new());
    }
    
    // Log acknowledged problems for debugging
    let ack_count = problems.iter().filter(|p| p.acknowledged == "1").count();
    if ack_count > 0 {
        tracing::debug!("Found {} acknowledged problems out of {}", ack_count, problems.len());
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
            "output": ["triggerid", "status"],
            "selectGroups": ["name"],
            "selectHosts": ["name"]
        }),
    )
    .await?;

    let trigger_map: HashMap<String, ZabbixTrigger> =
        triggers.into_iter().map(|t| (t.triggerid.clone(), t)).collect();

    // Drop problems whose trigger is disabled. Zabbix's own Problems UI hides these,
    // but problem.get still returns them — e.g. orphaned LLD triggers stuck "not
    // supported" (rescheduled Nomad per-alloc CSI mounts). Without this filter,
    // alertview shows alerts that no longer appear in Zabbix.
    let before = problems.len();
    let problems: Vec<ZabbixProblem> = problems
        .into_iter()
        .filter(|p| trigger_map.get(&p.objectid).map_or(true, |t| t.status != "1"))
        .collect();
    let dropped = before - problems.len();
    if dropped > 0 {
        tracing::debug!("Filtered {} Zabbix problem(s) from disabled triggers", dropped);
    }

    let alerts = problems
        .into_iter()
        .map(|p| {
            let severity = zabbix_severity(&p.severity).to_string();
            // Zabbix: suppressed (manually silenced) and acknowledged both result in "silenced" status
            // If acknowledged, include the ack message in annotations
            let status = if p.suppressed == "1" || p.acknowledged == "1" {
                "silenced"
            } else {
                "firing"
            }.to_string();

            let mut labels: HashMap<String, String> = HashMap::new();

            // Add Zabbix event and trigger IDs for template usage
            labels.insert("eventid".to_string(), p.eventid.clone());
            labels.insert("triggerid".to_string(), p.objectid.clone());

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
            
            // Add Zabbix acknowledged status as a label
            labels.insert("acknowledged".to_string(), p.acknowledged.clone());

            let mut annotations: HashMap<String, String> = HashMap::new();
            annotations.insert("summary".to_string(), p.name.clone());
            
            // If acknowledged in Zabbix, add the ack message and user info to annotations
            if p.acknowledged == "1" {
                tracing::debug!("Zabbix problem {} is acknowledged, acks count: {}", p.name, p.acknowledgements.len());
                if !p.acknowledgements.is_empty() {
                    // Use the most recent acknowledgement
                    if let Some(latest_ack) = p.acknowledgements.last() {
                        tracing::debug!("Using ACK from user {}: {:?}", latest_ack.user, latest_ack.message);
                        if let Some(ref msg) = latest_ack.message {
                            annotations.insert("acknowledgement".to_string(), msg.clone());
                        } else {
                            annotations.insert("acknowledgement".to_string(), "Acknowledged in Zabbix".to_string());
                        }
                        // Add who acknowledged and when
                        labels.insert("acknowledged_by".to_string(), latest_ack.user.clone());
                        labels.insert("acknowledged_at".to_string(), latest_ack.timestamp.clone());
                    }
                } else {
                    tracing::debug!("Problem is acknowledged but has no acknowledgements array");
                    annotations.insert("acknowledgement".to_string(), "Acknowledged in Zabbix".to_string());
                }
            }

            let ends_at = if p.r_clock != "0" {
                Some(unix_ts_to_iso(&p.r_clock))
            } else {
                None
            };

            // Build direct URL to Zabbix alert using triggerid
            // Try: link_template -> dashboard_url -> default zabbix URL
            // Note: Zabbix uses triggerids[] parameter to view specific problems
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
                    // If dashboard_url already contains triggerids parameter, use it as-is
                    if url.contains("triggerids") {
                        url
                    } else {
                        // Remove any existing query params and rebuild with triggerids
                        let clean_url = url.split_once('?').map(|(base, _)| base.to_string()).unwrap_or(url);
                        let base = if clean_url.contains("zabbix.php") {
                            clean_url
                        } else if clean_url.ends_with('/') {
                            format!("{}/zabbix.php", clean_url.trim_end_matches('/'))
                        } else {
                            format!("{}/zabbix.php", clean_url)
                        };
                        format!("{}/zabbix.php?action=problem.view&triggerids[]={}", 
                            base.trim_end_matches("/zabbix.php"), p.objectid)
                    }
                })
            }).or_else(|| {
                Some(format!(
                    "{}/zabbix.php?action=problem.view&triggerids[]={}",
                    source.url.trim_end_matches('/'),
                    p.objectid
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

    let url = format!("{}/alerts", source.url.trim_end_matches('/'));

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
    
    // Fetch silences to get comment information for silenced alerts
    let silences = match fetch_am_silences(client, source).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to fetch silences: {}", e);
            Vec::new()
        }
    };
    let silence_map: HashMap<String, String> = silences
        .into_iter()
        .map(|s| (s.id.clone(), s.comment.clone()))
        .collect();

    let alerts = am_alerts
        .into_iter()
        .map(|a| {
            let severity = lookup_label(&a.labels, &source.severity_label)
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

            // Add silence comment to annotations if alert is silenced
            let mut annotations = a.annotations.clone();
            if status == "silenced" && !a.status.silenced_by.is_empty() {
                // Try to find a matching silence comment
                for silence_id in &a.status.silenced_by {
                    if let Some(comment) = silence_map.get(silence_id) {
                        annotations.insert("silence_comment".to_string(), comment.clone());
                        break; // Use first matching silence
                    }
                }
                // If no specific comment found but silenced, add a generic message
                if !annotations.contains_key("silence_comment") {
                    annotations.insert("silence_comment".to_string(), "Silenced in Alertmanager".to_string());
                }
            }

            // dashboard_url from config takes priority over the internal generator_url
            // Then try link_template, then generator_url
            let link_url = source.dashboard_url.clone()
                .or(source.link_template.clone().and_then(|t| apply_link_template(&t, &Alert {
                    fingerprint: format!("{}:{}", source.name, a.fingerprint),
                    source: source.name.clone(),
                    source_type: source_type_str.clone(),
                    status: status.clone(),
                    severity: severity.clone(),
                    name: name.clone(),
                    labels: a.labels.clone(),
                    annotations: annotations.clone(),
                    starts_at: a.starts_at.clone(),
                    ends_at: ends_at.clone(),
                    link_url: None,
                })))
                .or(if a.generator_url.is_empty() {
                    None
                } else {
                    Some(a.generator_url)
                });

            Alert {
                fingerprint: format!("{}:{}", source.name, a.fingerprint),
                source: source.name.clone(),
                source_type: source_type_str,
                status,
                severity,
                name,
                labels: a.labels,
                annotations,
                starts_at: a.starts_at,
                ends_at,
                link_url,
            }
        })
        .collect();

    Ok(alerts)
}

/// Fetch Alertmanager silences to get comment information
async fn fetch_am_silences(client: &reqwest::Client, source: &Source) -> Result<Vec<AmSilence>> {
    if matches!(source.source_type, SourceType::Zabbix) {
        return Ok(vec![]);
    }

    let url = format!("{}/silences", source.url.trim_end_matches('/'));
    
    let mut req = client.get(&url);
    if let Some(auth) = &source.basic_auth {
        req = req.basic_auth(&auth.username, Some(&auth.password));
    }
    if let Some(token) = &source.bearer_token {
        req = req.bearer_auth(token);
    }
    
    let resp = req.send().await?;
    if !resp.status().is_success() {
        // Silences endpoint may not be available or may require different auth
        // Return empty list - alerts will still work, just without silence comments
        tracing::warn!("Failed to fetch silences from {}: {}", url, resp.status());
        return Ok(vec![]);
    }
    
    let silences: Vec<AmSilence> = resp.json().await?;
    Ok(silences)
}

/// Looks up a label by key, case-insensitively.
/// Tries an exact match first (fast path), then falls back to a
/// case-insensitive comparison.
fn lookup_label<'a>(labels: &'a HashMap<String, String>, key: &str) -> Option<&'a String> {
    if let Some(v) = labels.get(key) {
        return Some(v);
    }
    labels
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
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

/// Group alerts by specified labels
pub fn group_alerts(alerts: &[Alert], group_by: &[String]) -> Vec<AlertGroup> {
    if group_by.is_empty() {
        return Vec::new();
    }

    use std::collections::HashMap;

    let mut groups_map: HashMap<String, Vec<Alert>> = HashMap::new();

    for alert in alerts {
        let mut group_key_parts: Vec<String> = Vec::new();

        for label_key in group_by {
            if let Some(label_value) = alert.labels.get(label_key) {
                group_key_parts.push(format!("{}={}", label_key, label_value));
            } else {
                group_key_parts.push(format!("{}=<missing>", label_key));
            }
        }

        let group_key = group_key_parts.join(",");
        groups_map.entry(group_key).or_default().push(alert.clone());
    }

    let mut groups: Vec<AlertGroup> = groups_map
        .into_iter()
        .map(|(key, alerts)| {
            let severity_counts = count_severities(&alerts);
            // Extract labels from the first alert in the group (they should all have the same group_by labels)
            let mut labels = HashMap::new();
            if let Some(first_alert) = alerts.first() {
                for label_key in group_by {
                    if let Some(label_value) = first_alert.labels.get(label_key) {
                        labels.insert(label_key.clone(), label_value.clone());
                    } else {
                        labels.insert(label_key.clone(), "<missing>".to_string());
                    }
                }
            }
            AlertGroup {
                key,
                labels,
                alerts: alerts.clone(),
                count: alerts.len(),
                severity_counts,
            }
        })
        .collect();

    // Sort groups by key
    groups.sort_by(|a, b| a.key.cmp(&b.key));

    groups
}

fn count_severities(alerts: &[Alert]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for alert in alerts {
        *counts.entry(alert.severity.clone()).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_lookup_label_exact_match() {
        let labels: HashMap<String, String> =
            [("severity".to_string(), "critical".to_string())].into();
        assert_eq!(lookup_label(&labels, "severity"), Some(&"critical".to_string()));
    }

    #[test]
    fn test_lookup_label_case_insensitive() {
        let labels: HashMap<String, String> =
            [("Severity".to_string(), "high".to_string())].into();
        // Configured key "severity" still matches a "Severity" label.
        assert_eq!(lookup_label(&labels, "severity"), Some(&"high".to_string()));
    }

    #[test]
    fn test_lookup_label_custom_key() {
        let labels: HashMap<String, String> =
            [("priority".to_string(), "warning".to_string())].into();
        assert_eq!(lookup_label(&labels, "priority"), Some(&"warning".to_string()));
        assert_eq!(lookup_label(&labels, "severity"), None);
    }

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

    #[test]
    fn test_group_alerts_by_namespace() {
        let alerts = vec![
            Alert {
                fingerprint: "1".to_string(),
                source: "test".to_string(),
                source_type: "alertmanager".to_string(),
                status: "firing".to_string(),
                severity: "critical".to_string(),
                name: "Alert1".to_string(),
                labels: HashMap::from([("namespace".to_string(), "prod".to_string())]),
                annotations: HashMap::new(),
                starts_at: "2024-01-01T00:00:00Z".to_string(),
                ends_at: None,
                link_url: None,
            },
            Alert {
                fingerprint: "2".to_string(),
                source: "test".to_string(),
                source_type: "alertmanager".to_string(),
                status: "firing".to_string(),
                severity: "high".to_string(),
                name: "Alert2".to_string(),
                labels: HashMap::from([("namespace".to_string(), "prod".to_string())]),
                annotations: HashMap::new(),
                starts_at: "2024-01-01T00:00:00Z".to_string(),
                ends_at: None,
                link_url: None,
            },
            Alert {
                fingerprint: "3".to_string(),
                source: "test".to_string(),
                source_type: "alertmanager".to_string(),
                status: "firing".to_string(),
                severity: "warning".to_string(),
                name: "Alert3".to_string(),
                labels: HashMap::from([("namespace".to_string(), "dev".to_string())]),
                annotations: HashMap::new(),
                starts_at: "2024-01-01T00:00:00Z".to_string(),
                ends_at: None,
                link_url: None,
            },
        ];

        let groups = group_alerts(&alerts, &["namespace".to_string()]);
        assert_eq!(groups.len(), 2);
        
        // Find prod and dev groups
        let prod_group = groups.iter().find(|g| g.key == "namespace=prod").unwrap();
        let dev_group = groups.iter().find(|g| g.key == "namespace=dev").unwrap();
        
        assert_eq!(prod_group.count, 2);
        assert_eq!(dev_group.count, 1);
    }

    #[test]
    fn test_group_alerts_multiple_labels() {
        let alerts = vec![
            Alert {
                fingerprint: "1".to_string(),
                source: "test".to_string(),
                source_type: "alertmanager".to_string(),
                status: "firing".to_string(),
                severity: "critical".to_string(),
                name: "Alert1".to_string(),
                labels: HashMap::from([
                    ("namespace".to_string(), "prod".to_string()),
                    ("job".to_string(), "api".to_string()),
                ]),
                annotations: HashMap::new(),
                starts_at: "2024-01-01T00:00:00Z".to_string(),
                ends_at: None,
                link_url: None,
            },
            Alert {
                fingerprint: "2".to_string(),
                source: "test".to_string(),
                source_type: "alertmanager".to_string(),
                status: "firing".to_string(),
                severity: "high".to_string(),
                name: "Alert2".to_string(),
                labels: HashMap::from([
                    ("namespace".to_string(), "prod".to_string()),
                    ("job".to_string(), "web".to_string()),
                ]),
                annotations: HashMap::new(),
                starts_at: "2024-01-01T00:00:00Z".to_string(),
                ends_at: None,
                link_url: None,
            },
        ];

        let groups = group_alerts(&alerts, &["namespace".to_string(), "job".to_string()]);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].key, "namespace=prod,job=api");
        assert_eq!(groups[1].key, "namespace=prod,job=web");
    }

    #[test]
    fn test_group_alerts_empty_group_by() {
        let alerts = vec![
            Alert {
                fingerprint: "1".to_string(),
                source: "test".to_string(),
                source_type: "alertmanager".to_string(),
                status: "firing".to_string(),
                severity: "critical".to_string(),
                name: "Alert1".to_string(),
                labels: HashMap::from([("namespace".to_string(), "prod".to_string())]),
                annotations: HashMap::new(),
                starts_at: "2024-01-01T00:00:00Z".to_string(),
                ends_at: None,
                link_url: None,
            },
        ];

        let groups = group_alerts(&alerts, &[]);
        assert_eq!(groups.len(), 0);
    }

    #[test]
    fn test_zabbix_acknowledgement_parsing() {
        use serde_json::json;
        
        // Test avec acknowledged comme string
        let json_data = json!({
            "eventid": "12345",
            "objectid": "23456",
            "name": "Linux: Interface virbr0: Link down",
            "severity": "2",
            "clock": "1714000000",
            "r_clock": "0",
            "suppressed": "0",
            "acknowledged": "1",
            "acknowledgements": [
                {
                    "acknowledgeid": "789",
                    "useralias": "Admin",
                    "clock": "1714000100",
                    "message": "Working on it - scheduled maintenance"
                }
            ],
            "tags": []
        });
        
        let problem: ZabbixProblem = serde_json::from_value(json_data).unwrap();
        
        assert_eq!(problem.name, "Linux: Interface virbr0: Link down");
        assert_eq!(problem.acknowledged, "1");
        assert_eq!(problem.acknowledgements.len(), 1);
        assert_eq!(problem.acknowledgements[0].user, "Admin");
        assert_eq!(problem.acknowledgements[0].message, Some("Working on it - scheduled maintenance".to_string()));
        
        // Test avec acknowledged comme booléen
        let json_bool = json!({
            "eventid": "12346",
            "objectid": "23457",
            "name": "Test",
            "severity": "1",
            "clock": "1714000000",
            "r_clock": "0",
            "suppressed": "0",
            "acknowledged": true,
            "acknowledgements": [
                {
                    "acknowledgeid": "790",
                    "useralias": "User",
                    "clock": "1714000100",
                    "comment": "Boolean ack test"
                }
            ],
            "tags": []
        });
        
        let problem2: ZabbixProblem = serde_json::from_value(json_bool).unwrap();
        assert_eq!(problem2.acknowledged, "1");
        assert_eq!(problem2.acknowledgements[0].message, Some("Boolean ack test".to_string()));
    }
}
