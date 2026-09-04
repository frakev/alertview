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
    /// Link behind the ↗ button ("open in the source").
    pub link_url: Option<String>,
    /// Link the whole alert points to, when a template declares one.
    pub alert_link_url: Option<String>,
}

/// A non-2xx response from a source. Carried as a typed error (rather than a
/// formatted string) so the retry loop can tell a 4xx from a 5xx.
#[derive(Debug)]
pub struct HttpStatusError {
    pub status: reqwest::StatusCode,
    pub url: String,
}

impl std::fmt::Display for HttpStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP {} from {}", self.status, self.url)
    }
}

impl std::error::Error for HttpStatusError {}

/// A JSON-RPC error returned by Zabbix itself (HTTP 200 with an `error`
/// member). Typed, so the version fallbacks below can tell "this Zabbix does
/// not know that parameter" from "this Zabbix said no": only the first is
/// worth retrying under an older parameter name.
#[derive(Debug)]
pub struct ZabbixApiError {
    pub code: i32,
    pub message: String,
    pub data: String,
}

impl std::fmt::Display for ZabbixApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Zabbix API error {}: {} \u{2014} {}", self.code, self.message, self.data)
    }
}

impl std::error::Error for ZabbixApiError {}

/// JSON-RPC -32602 "Invalid params": the parameter does not exist on this
/// version. Anything else (auth, permissions, HTTP, network) must surface
/// as-is rather than be retried under a different name.
fn is_invalid_params(err: &anyhow::Error) -> bool {
    err.downcast_ref::<ZabbixApiError>()
        .is_some_and(|e| e.code == -32602)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_css: Option<String>,
    #[serde(default)]
    pub play_sounds: bool,
    #[serde(default)]
    pub groups: Vec<AlertGroup>,
    #[serde(default)]
    pub group_by: Vec<String>,
    #[serde(default)]
    pub severity_order: Vec<String>,
    #[serde(default)]
    pub prefix_labels: Vec<String>,
    #[serde(default)]
    pub prefix_separator: String,
    #[serde(default)]
    pub tv_mode_default: bool,
    #[serde(default)]
    pub link_new_tab: bool,
    #[serde(default)]
    pub show_alert_name: bool,
    #[serde(default)]
    pub show_labels: bool,
    #[serde(default)]
    pub critical_icon: String,
    #[serde(default)]
    pub status_icons: HashMap<String, String>,
}

/// A group header. The alerts themselves are not repeated here — the frontend
/// picks them out of the main `alerts` list from `labels`, and serialising them
/// twice doubled the payload.
#[derive(Debug, Serialize)]
pub struct AlertGroup {
    pub key: String,
    pub labels: std::collections::HashMap<String, String>,
    pub count: usize,
    pub severity_counts: std::collections::HashMap<String, usize>,
}

// Alertmanager v2 API wire types
// Only `fingerprint` is required: an alert without one cannot be tracked at
// all. Everything else defaults, so one unusual entry does not take down the
// deserialisation of the whole array — see the per-item parse in
// fetch_source_alerts.
#[derive(Debug, Deserialize)]
struct AmAlert {
    #[serde(default)]
    annotations: HashMap<String, String>,
    #[serde(rename = "endsAt", default)]
    ends_at: String,
    fingerprint: String,
    #[serde(rename = "generatorURL", default)]
    generator_url: String,
    #[serde(default)]
    labels: HashMap<String, String>,
    #[serde(rename = "startsAt", default)]
    starts_at: String,
    #[serde(default)]
    status: AmStatus,
}

#[derive(Debug, Deserialize)]
struct AmStatus {
    // A missing status reads as firing: on an alert dashboard, showing an
    // alert that turns out to be pending beats hiding one that is not.
    #[serde(default = "default_am_state")]
    state: String,
    // Alertmanager sends this as "silencedBy"; without the rename the field
    // silently stayed empty and no silence comment was ever resolved.
    #[serde(rename = "silencedBy", default)]
    silenced_by: Vec<String>, // List of silence IDs that silenced this alert
    // An inhibited alert is "suppressed" too, but by another alert rather than
    // by a silence — without this it showed up as silenced with no comment.
    #[serde(rename = "inhibitedBy", default)]
    inhibited_by: Vec<String>,
}

impl Default for AmStatus {
    fn default() -> Self {
        Self {
            state: default_am_state(),
            silenced_by: Vec::new(),
            inhibited_by: Vec::new(),
        }
    }
}

fn default_am_state() -> String {
    "active".to_string()
}

#[derive(Debug, Deserialize)]
struct AmSilence {
    id: String,
    #[serde(rename = "createdBy", default)]
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

// Every field is optional. `useralias` used to be required here, and no
// Zabbix ever returns it on an acknowledgement — the object carries a
// `userid`, the name needs a separate user.get. A required field serde could
// not find made the *whole* problem.get response fail to deserialise, which is
// what the three-way fallback below was papering over: acknowledgement
// comments never reached the UI on any version.
#[derive(Deserialize, Default)]
struct ZabbixAcknowledgement {
    #[serde(default)]
    userid: String,
    /// Only 5.0 could return a name inline, under `useralias`; 5.4+ renamed it
    /// `username`. Empty otherwise, and resolved through user.get.
    #[serde(default, alias = "useralias", alias = "username")]
    user: String,
    #[serde(default, rename = "clock")]
    timestamp: String,
    #[serde(default, alias = "comment")]
    message: Option<String>,
}

/// A Zabbix user, fetched only to put a name on an acknowledgement.
#[derive(Deserialize)]
struct ZabbixUser {
    userid: String,
    /// `username` since 5.4, `alias` before it.
    #[serde(default, alias = "alias")]
    username: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    surname: String,
}

impl ZabbixUser {
    /// Full name when the account has one, the login otherwise.
    fn display_name(&self) -> String {
        let full = format!("{} {}", self.name.trim(), self.surname.trim());
        let full = full.trim();
        if full.is_empty() { self.username.clone() } else { full.to_string() }
    }
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
    // `selectGroups` answers under "groups", `selectHostGroups` under
    // "hostgroups" — see ZabbixDialect.
    #[serde(default, alias = "hostgroups")]
    groups: Vec<ZabbixGroupInfo>,
}

// ── Zabbix dialects ──────────────────────────────────────────────────────

/// Parameter names Zabbix renamed between versions. AlertView has no version
/// pin, so it probes: the modern name first, the older one if this server
/// rejects it as an unknown parameter. What worked is remembered per source,
/// so an old server pays the extra round-trip once, not on every poll.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ZabbixDialect {
    /// trigger.get: `selectHostGroups` since 6.4, `selectGroups` before it.
    /// `selectGroups` was *removed* in 7.0, which took the whole source down.
    groups_param: &'static str,
    /// problem.get: `selectAcknowledgements` since 6.0, `selectAcknowledges`
    /// before it, `None` for a server that knows neither.
    ack_param: Option<&'static str>,
}

impl Default for ZabbixDialect {
    fn default() -> Self {
        Self {
            groups_param: "selectHostGroups",
            ack_param: Some("selectAcknowledgements"),
        }
    }
}

fn dialect_cache() -> &'static std::sync::RwLock<HashMap<String, ZabbixDialect>> {
    static CACHE: std::sync::OnceLock<std::sync::RwLock<HashMap<String, ZabbixDialect>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn dialect_for(source: &str) -> ZabbixDialect {
    dialect_cache()
        .read()
        .ok()
        .and_then(|cache| cache.get(source).copied())
        .unwrap_or_default()
}

fn remember_dialect(source: &str, dialect: ZabbixDialect) {
    if dialect_for(source) == dialect {
        return;
    }
    tracing::debug!("Zabbix dialect for {}: {:?}", source, dialect);
    if let Ok(mut cache) = dialect_cache().write() {
        cache.insert(source.to_string(), dialect);
    }
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
        return Err(HttpStatusError { status: resp.status(), url: url.to_string() }.into());
    }
    let rpc: ZabbixRpcResponse<T> = resp.json().await?;
    if let Some(err) = rpc.error {
        return Err(ZabbixApiError {
            code: err.code,
            message: err.message,
            data: err.data,
        }
        .into());
    }
    rpc.result.ok_or_else(|| anyhow::anyhow!("Zabbix API returned empty result"))
}

/// problem.get, degrading through the acknowledgement parameter names.
/// Returns the name that worked so the caller can remember it.
async fn fetch_zabbix_problems(
    client: &reqwest::Client,
    source: &Source,
    api_url: &str,
    preferred: Option<&'static str>,
) -> Result<(Vec<ZabbixProblem>, Option<&'static str>)> {
    // Newest first, and never re-try a name this server already rejected.
    let candidates: Vec<Option<&'static str>> = match preferred {
        Some("selectAcknowledgements") => vec![
            Some("selectAcknowledgements"),
            Some("selectAcknowledges"),
            None,
        ],
        Some("selectAcknowledges") => vec![Some("selectAcknowledges"), None],
        _ => vec![None],
    };

    let mut last: Option<anyhow::Error> = None;
    for candidate in candidates {
        let mut params = serde_json::json!({ "output": "extend", "selectTags": "extend" });
        if let Some(name) = candidate {
            params[name] = serde_json::Value::Bool(true);
        }
        match zabbix_rpc::<Vec<ZabbixProblem>>(client, source, api_url, "problem.get", params).await
        {
            Ok(problems) => return Ok((problems, candidate)),
            // Only an unknown-parameter error means "try the older name".
            // An auth failure used to fall through all three attempts and
            // surface as whatever the last one said.
            Err(e) if is_invalid_params(&e) => {
                tracing::debug!("Zabbix {} rejected {:?}: {}", source.name, candidate, e);
                last = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("problem.get returned no usable response")))
}

/// trigger.get, degrading through the host-group parameter names.
async fn fetch_zabbix_triggers(
    client: &reqwest::Client,
    source: &Source,
    api_url: &str,
    trigger_ids: &[&str],
    preferred: &'static str,
) -> Result<(Vec<ZabbixTrigger>, &'static str)> {
    let candidates: [&'static str; 2] = if preferred == "selectGroups" {
        ["selectGroups", "selectHostGroups"]
    } else {
        ["selectHostGroups", "selectGroups"]
    };

    let mut last: Option<anyhow::Error> = None;
    for candidate in candidates {
        let params = serde_json::json!({
            "triggerids": trigger_ids,
            "output": ["triggerid", "status"],
            candidate: ["name"],
            "selectHosts": ["name"]
        });
        match zabbix_rpc::<Vec<ZabbixTrigger>>(client, source, api_url, "trigger.get", params).await
        {
            Ok(triggers) => return Ok((triggers, candidate)),
            Err(e) if is_invalid_params(&e) => {
                tracing::debug!("Zabbix {} rejected {}: {}", source.name, candidate, e);
                last = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("trigger.get returned no usable response")))
}

/// Names for the users behind a set of acknowledgements. A read-only API user
/// may not be allowed to list users at all, so a failure here costs the author
/// line and nothing else.
async fn fetch_zabbix_user_names(
    client: &reqwest::Client,
    source: &Source,
    api_url: &str,
    userids: Vec<&str>,
) -> HashMap<String, String> {
    if userids.is_empty() {
        return HashMap::new();
    }
    let params = serde_json::json!({
        "output": ["userid", "username", "name", "surname"],
        "userids": userids,
    });
    match zabbix_rpc::<Vec<ZabbixUser>>(client, source, api_url, "user.get", params).await {
        Ok(users) => users
            .into_iter()
            .map(|u| (u.userid.clone(), u.display_name()))
            .filter(|(_, name)| !name.is_empty())
            .collect(),
        Err(e) => {
            tracing::debug!("Could not resolve Zabbix user names for {}: {}", source.name, e);
            HashMap::new()
        }
    }
}

async fn fetch_zabbix_alerts(client: &reqwest::Client, source: &Source) -> Result<Vec<Alert>> {
    let api_url = format!("{}/api_jsonrpc.php", source.url.trim_end_matches('/'));
    let mut dialect = dialect_for(&source.name);

    // Step 1: active problems, with their acknowledgements when this version
    // knows how to return them.
    let (problems, ack_param) =
        fetch_zabbix_problems(client, source, &api_url, dialect.ack_param).await?;
    dialect.ack_param = ack_param;

    if problems.is_empty() {
        tracing::debug!("No Zabbix problems found");
        remember_dialect(&source.name, dialect);
        return Ok(Vec::new());
    }

    // Log acknowledged problems for debugging
    let ack_count = problems.iter().filter(|p| p.acknowledged == "1").count();
    if ack_count > 0 {
        tracing::debug!("Found {} acknowledged problems out of {}", ack_count, problems.len());
    }

    // Step 2: enrich with hosts + hostgroups via their trigger
    let trigger_ids: Vec<&str> = problems.iter().map(|p| p.objectid.as_str()).collect();
    let (triggers, groups_param) =
        fetch_zabbix_triggers(client, source, &api_url, &trigger_ids, dialect.groups_param).await?;
    dialect.groups_param = groups_param;
    remember_dialect(&source.name, dialect);

    let trigger_map: HashMap<String, ZabbixTrigger> =
        triggers.into_iter().map(|t| (t.triggerid.clone(), t)).collect();

    // Step 3: put a name on the acknowledgements that only carry a userid.
    let userids: Vec<&str> = problems
        .iter()
        .flat_map(|p| p.acknowledgements.iter())
        .filter(|ack| ack.user.is_empty() && !ack.userid.is_empty())
        .map(|ack| ack.userid.as_str())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let user_names = fetch_zabbix_user_names(client, source, &api_url, userids).await;

    // Drop problems whose trigger is disabled. Zabbix's own Problems UI hides these,
    // but problem.get still returns them — e.g. orphaned LLD triggers stuck "not
    // supported" (rescheduled Nomad per-alloc CSI mounts). Without this filter,
    // alertview shows alerts that no longer appear in Zabbix.
    let before = problems.len();
    let problems: Vec<ZabbixProblem> = problems
        .into_iter()
        .filter(|p| trigger_map.get(&p.objectid).is_none_or(|t| t.status != "1"))
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
            // Zabbix has no alertname label of its own, so a link template
            // written for Alertmanager silently produced nothing here. Expose
            // the problem name under the same key the other sources use.
            labels.insert("alertname".to_string(), p.name.clone());

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
                        let author = if latest_ack.user.is_empty() {
                            user_names.get(&latest_ack.userid).cloned().unwrap_or_default()
                        } else {
                            latest_ack.user.clone()
                        };
                        tracing::debug!("Using ACK from user {:?}: {:?}", author, latest_ack.message);
                        match latest_ack.message.as_deref().map(str::trim) {
                            Some(msg) if !msg.is_empty() => {
                                annotations.insert("acknowledgement".to_string(), msg.to_string());
                            }
                            _ => {
                                annotations.insert(
                                    "acknowledgement".to_string(),
                                    "Acknowledged in Zabbix".to_string(),
                                );
                            }
                        }
                        // Add who acknowledged and when
                        if !author.is_empty() {
                            labels.insert("acknowledged_by".to_string(), author);
                        }
                        if !latest_ack.timestamp.is_empty() {
                            labels.insert("acknowledged_at".to_string(), latest_ack.timestamp.clone());
                        }
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

            // Zabbix has no per-alert link of its own, so the fallback is a
            // problem.view URL pointing at the trigger.
            let fallbacks = vec![zabbix_problem_link(source, &p.objectid)];

            let mut alert = Alert {
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
                link_url: None,
                alert_link_url: None,
            };

            let (link_url, alert_link_url) = build_links(&alert, source, &fallbacks);
            alert.link_url = link_url;
            alert.alert_link_url = alert_link_url;
            alert
        })
        .collect();

    Ok(alerts)
}

// ── Main dispatcher ──────────────────────────────────────────────────────────

/// Build the Alertmanager v2 API base URL for a source.
///
/// The configured `url` is documented as the plain service base
/// (`http://host:9093`), so the API path is appended here. URLs that already
/// point at the API — with or without a trailing `/alerts` — are kept as-is,
/// and a query string is preserved for the alerts request.
fn am_api_base(source: &Source) -> (String, String) {
    let (path, query) = match source.url.split_once('?') {
        Some((path, query)) => (path, format!("?{}", query)),
        None => (source.url.as_str(), String::new()),
    };

    let path = path.trim_end_matches('/');
    let path = path.strip_suffix("/alerts").unwrap_or(path);

    let base = if path.ends_with("/api/v2") {
        path.to_string()
    } else if source.source_type == SourceType::Grafana {
        format!("{}/api/alertmanager/grafana/api/v2", path)
    } else {
        format!("{}/api/v2", path)
    };

    (base, query)
}

pub async fn fetch_source_alerts(client: &reqwest::Client, source: &Source) -> Result<Vec<Alert>> {
    if source.source_type == SourceType::Zabbix {
        return fetch_zabbix_alerts(client, source).await;
    }

    let (api_base, query) = am_api_base(source);
    let url = format!("{}/alerts{}", api_base, query);

    let mut req = client.get(&url);

    if let Some(auth) = &source.basic_auth {
        req = req.basic_auth(&auth.username, Some(&auth.password));
    }
    if let Some(token) = &source.bearer_token {
        req = req.bearer_auth(token);
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Err(HttpStatusError { status: resp.status(), url }.into());
    }

    // Parsed one by one: a single malformed alert used to fail the whole
    // array, and the source went from "N alerts" to "error" with nothing shown.
    let raw: Vec<serde_json::Value> = resp.json().await?;
    let received = raw.len();
    let am_alerts: Vec<AmAlert> = raw
        .into_iter()
        .filter_map(|value| match serde_json::from_value::<AmAlert>(value) {
            Ok(alert) => Some(alert),
            Err(e) => {
                tracing::warn!("Skipping an unparseable alert from {}: {}", source.name, e);
                None
            }
        })
        .collect();
    if am_alerts.len() < received {
        tracing::warn!(
            "Dropped {}/{} unparseable alert(s) from {}",
            received - am_alerts.len(),
            received,
            source.name
        );
    }
    
    // Fetch silences to get comment information for silenced alerts. Skipped
    // when nothing is silenced, which is the common case — no point in a second
    // round-trip per source per poll just to look up comments nobody needs.
    let any_silenced = am_alerts.iter().any(|a| !a.status.silenced_by.is_empty());
    let silences = if any_silenced {
        match fetch_am_silences(client, source).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to fetch silences: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    // Who silenced an alert is as useful as why, so both travel with the id.
    let silence_map: HashMap<String, AmSilence> =
        silences.into_iter().map(|s| (s.id.clone(), s)).collect();

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
            if status == "silenced" && a.status.silenced_by.is_empty() && !a.status.inhibited_by.is_empty() {
                annotations.insert(
                    "silence_comment".to_string(),
                    "Inhibited by another alert".to_string(),
                );
            } else if status == "silenced" && !a.status.silenced_by.is_empty() {
                // Try to find a matching silence comment
                for silence_id in &a.status.silenced_by {
                    if let Some(silence) = silence_map.get(silence_id) {
                        annotations
                            .insert("silence_comment".to_string(), silence.comment.clone());
                        if !silence.created_by.is_empty() {
                            annotations.insert(
                                "silence_created_by".to_string(),
                                silence.created_by.clone(),
                            );
                        }
                        break; // Use first matching silence
                    }
                }
                // If no specific comment found but silenced, add a generic message
                if !annotations.contains_key("silence_comment") {
                    annotations.insert("silence_comment".to_string(), "Silenced in Alertmanager".to_string());
                }
            }

            // Per-alert links come first, the static dashboard_url last.
            let fallbacks: Vec<String> = [a.generator_url, source.dashboard_url.clone().unwrap_or_default()]
                .into_iter()
                .filter(|u| !u.is_empty())
                .collect();

            let mut alert = Alert {
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
                link_url: None,
                alert_link_url: None,
            };

            let (link_url, alert_link_url) = build_links(&alert, source, &fallbacks);
            alert.link_url = link_url;
            alert.alert_link_url = alert_link_url;
            alert
        })
        .collect();

    Ok(alerts)
}

/// Fetch Alertmanager silences to get comment information
async fn fetch_am_silences(client: &reqwest::Client, source: &Source) -> Result<Vec<AmSilence>> {
    if matches!(source.source_type, SourceType::Zabbix) {
        return Ok(vec![]);
    }

    let (api_base, _) = am_api_base(source);
    let url = format!("{}/silences", api_base);
    
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

/// Canonical name of a severity, so common aliases share a rank.
fn canonical_severity(severity: &str) -> String {
    let sev = severity.trim().to_lowercase();
    match sev.as_str() {
        "crit" => "critical".to_string(),
        "err" => "error".to_string(),
        "warn" => "warning".to_string(),
        "information" => "info".to_string(),
        _ => sev,
    }
}

/// Rank of a severity within the configured order (lower = more severe).
/// Severities missing from the configured list sort after every listed level.
pub fn severity_rank(order: &[String], severity: &str) -> usize {
    let sev = canonical_severity(severity);
    order
        .iter()
        .position(|s| canonical_severity(s) == sev)
        .unwrap_or(order.len())
}

/// Zabbix problem.view URL for a trigger, used when no template applies.
/// Honours `dashboard_url` when it points at a Zabbix frontend.
fn zabbix_problem_link(source: &Source, triggerid: &str) -> String {
    let base = match source.dashboard_url.as_deref() {
        // Already targets specific triggers: take it as-is.
        Some(url) if url.contains("triggerids") => return url.to_string(),
        Some(url) => url.split_once('?').map(|(base, _)| base).unwrap_or(url),
        None => source.url.as_str(),
    };
    let base = base.trim_end_matches('/').trim_end_matches("/zabbix.php");
    format!("{}/zabbix.php?action=problem.view&triggerids[]={}", base, triggerid)
}

/// Only http(s) links are ever handed to the frontend: an alert is a clickable
/// element, and `generatorURL` or `dashboard_url` come from outside.
fn sanitize_link(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Some(trimmed.to_string())
    } else {
        tracing::debug!("Ignoring link with unsupported scheme: {}", trimmed);
        None
    }
}

/// The two links carried by an alert:
/// - the ↗ button: `link_template`, then the given fallbacks (generator URL,
///   dashboard URL…), unless the source disables it;
/// - the alert itself: `alert_link_template` only. The config-wide template is
///   applied later, in the handler, where the display config is available.
fn build_links(
    alert: &Alert,
    source: &Source,
    fallbacks: &[String],
) -> (Option<String>, Option<String>) {
    let source_link = if source.source_link == Some(false) {
        None
    } else {
        source
            .link_template
            .as_deref()
            .and_then(|t| apply_link_template(t, alert))
            .or_else(|| fallbacks.iter().find_map(|u| sanitize_link(u)))
    };

    let alert_link = source
        .alert_link_template
        .as_deref()
        .and_then(|t| apply_link_template(t, alert));

    (source_link, alert_link)
}

/// Applique un template de lien avec les variables de l'alerte
/// Percent-encode a value substituted into a URL: everything but the RFC 3986
/// unreserved characters, so a label with a space, `&` or `/` cannot change the
/// shape of the URL.
fn encode_value(value: &str) -> String {
    const UNRESERVED: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'~');
    percent_encoding::utf8_percent_encode(value, UNRESERVED).to_string()
}

pub fn apply_link_template(template: &str, alert: &Alert) -> Option<String> {
    if template.is_empty() {
        return None;
    }
    
    let mut result = template.to_string();
    
    // Remplacer les variables de labels
    for (key, value) in &alert.labels {
        let placeholder = format!("{{{{.Labels.{}}}}}", key);
        result = result.replace(&placeholder, &encode_value(value));
    }
    
    // Remplacer les variables d'annotations
    for (key, value) in &alert.annotations {
        let placeholder = format!("{{{{.Annotations.{}}}}}", key);
        result = result.replace(&placeholder, &encode_value(value));
    }
    
    // Remplacer les variables standards
    result = result.replace("{{.Id}}", &encode_value(&alert.fingerprint));
    result = result.replace("{{.Fingerprint}}", &encode_value(&alert.fingerprint));
    result = result.replace("{{.Source}}", &encode_value(&alert.source));
    result = result.replace("{{.SourceType}}", &encode_value(&alert.source_type));
    result = result.replace("{{.Status}}", &encode_value(&alert.status));
    result = result.replace("{{.Severity}}", &encode_value(&alert.severity));
    result = result.replace("{{.Name}}", &encode_value(&alert.name));
    result = result.replace("{{.StartsAt}}", &encode_value(&alert.starts_at));
    
    if let Some(ends_at) = &alert.ends_at {
        result = result.replace("{{.EndsAt}}", &encode_value(ends_at));
    }

    // A placeholder left over means the alert does not carry what the template
    // asks for. Emitting the URL with `{{.Labels.foo}}` still in it is worse
    // than falling back to whatever comes next.
    if result.contains("{{.") {
        tracing::debug!("Link template has unresolved placeholders, ignoring: {}", result);
        return None;
    }

    sanitize_link(&result)
}

/// Group alerts by specified labels
pub fn group_alerts(
    alerts: &[Alert],
    group_by: &[String],
    severity_order: &[String],
) -> Vec<AlertGroup> {
    if group_by.is_empty() {
        return Vec::new();
    }

    use std::collections::HashMap;

    let mut groups_map: HashMap<String, Vec<&Alert>> = HashMap::new();

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
        groups_map.entry(group_key).or_default().push(alert);
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
                count: alerts.len(),
                severity_counts,
            }
        })
        .collect();

    // Most severe group first — on a wall display the team with a critical
    // has to be at the top, not wherever the alphabet puts it.
    groups.sort_by(|a, b| {
        let worst = |g: &AlertGroup| {
            g.severity_counts
                .keys()
                .map(|s| severity_rank(severity_order, s))
                .min()
                .unwrap_or(usize::MAX)
        };
        worst(a).cmp(&worst(b)).then_with(|| a.key.cmp(&b.key))
    });

    groups
}

fn count_severities(alerts: &[&Alert]) -> HashMap<String, usize> {
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

    #[test]
    fn test_severity_rank_follows_configured_order() {
        let order = crate::config::DisplayConfig::default().severity_order;
        assert!(severity_rank(&order, "critical") < severity_rank(&order, "error"));
        assert!(severity_rank(&order, "error") < severity_rank(&order, "high"));
        assert!(severity_rank(&order, "high") < severity_rank(&order, "warning"));
        assert!(severity_rank(&order, "warning") < severity_rank(&order, "info"));
        assert!(severity_rank(&order, "info") < severity_rank(&order, "none"));
    }

    #[test]
    fn test_severity_rank_aliases_and_unknown() {
        let order = crate::config::DisplayConfig::default().severity_order;
        // Aliases and casing rank with their canonical level.
        assert_eq!(severity_rank(&order, "ERR"), severity_rank(&order, "error"));
        assert_eq!(severity_rank(&order, "warn"), severity_rank(&order, "warning"));
        // A severity nobody configured sorts after every listed level.
        assert_eq!(severity_rank(&order, "pager"), order.len());
    }

    #[test]
    fn test_severity_rank_custom_order() {
        let order: Vec<String> = ["disaster", "error", "notice"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(severity_rank(&order, "disaster"), 0);
        assert_eq!(severity_rank(&order, "notice"), 2);
        // Levels dropped from the list lose their built-in rank.
        assert_eq!(severity_rank(&order, "critical"), 3);
    }

    // ── Integration tests against a stub Alertmanager ────────────────────
    // These exist because unit tests could not have caught the bugs that hurt:
    // the API path being dropped from the URL, "silencedBy" never
    // deserialising, or a javascript: generator URL reaching the frontend.

    async fn spawn_stub(alerts: &'static str, silences: &'static str) -> String {
        use axum::{routing::get, Router};
        let json = |body: &'static str| async move {
            ([("content-type", "application/json")], body)
        };
        let app = Router::new()
            .route("/api/v2/alerts", get(move || json(alerts)))
            .route("/api/v2/silences", get(move || json(silences)));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{}", addr)
    }

    const ONE_ALERT: &str = r#"[{
        "fingerprint": "abc",
        "status": {"state": "suppressed", "silencedBy": ["sil-1"]},
        "labels": {"alertname": "DiskFull", "severity": "critical", "hostname": "srv-01"},
        "annotations": {"summary": "disk full"},
        "startsAt": "2026-09-04T10:00:00Z",
        "endsAt": "0001-01-01T00:00:00Z",
        "generatorURL": "javascript:alert(1)"
    }]"#;
    const ONE_SILENCE: &str =
        r#"[{"id": "sil-1", "createdBy": "alice", "comment": "maintenance"}]"#;

    /// A stub Zabbix that only knows the parameter names of one version, so
    /// the dialect probing can be tested for real. `groups_param` is what its
    /// trigger.get accepts; anything else comes back as -32602, exactly the
    /// way a 7.0 answers `selectGroups` and a 6.0 answers `selectHostGroups`.
    async fn spawn_zabbix_stub(groups_param: &'static str, ack_param: &'static str) -> String {
        use axum::{routing::post, Json, Router};
        use serde_json::{json, Value};

        let handler = move |Json(body): Json<Value>| async move {
            let method = body["method"].as_str().unwrap_or_default().to_string();
            let params = body["params"].clone();
            let invalid = |what: &str| {
                json!({"jsonrpc": "2.0", "id": 1, "error": {
                    "code": -32602, "message": "Invalid params.",
                    "data": format!("unexpected parameter \"{}\"", what)}})
            };

            let result = match method.as_str() {
                "problem.get" => {
                    for name in ["selectAcknowledgements", "selectAcknowledges"] {
                        if !params[name].is_null() && name != ack_param {
                            return Json(invalid(name));
                        }
                    }
                    json!([{
                        "eventid": "1", "objectid": "42", "name": "Disk full",
                        "severity": "5", "clock": "1788000000", "r_clock": "0",
                        "suppressed": "0", "acknowledged": "1",
                        // A real Zabbix returns a userid, never a name.
                        "acknowledgements": [
                            {"acknowledgeid": "9", "userid": "7", "clock": "1788000100",
                             "message": "on it"}
                        ],
                        "tags": [{"tag": "team", "value": "sre"}]
                    }])
                }
                "trigger.get" => {
                    for name in ["selectHostGroups", "selectGroups"] {
                        if !params[name].is_null() && name != groups_param {
                            return Json(invalid(name));
                        }
                    }
                    let groups = json!([{"name": "Linux servers"}]);
                    let mut trigger = json!({
                        "triggerid": "42", "status": "0", "hosts": [{"name": "srv-01"}]
                    });
                    trigger[if groups_param == "selectHostGroups" { "hostgroups" } else { "groups" }] =
                        groups;
                    json!([trigger])
                }
                "user.get" => json!([
                    {"userid": "7", "username": "alice", "name": "Alice", "surname": "F"}
                ]),
                _ => json!([]),
            };
            Json(json!({"jsonrpc": "2.0", "id": 1, "result": result}))
        };

        let app = Router::new().route("/api_jsonrpc.php", post(handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{}", addr)
    }

    /// Zabbix 7.0 removed `selectGroups` from trigger.get. Sending it made the
    /// call fail, and with it the whole source.
    #[tokio::test]
    async fn test_zabbix_7_dialect() {
        let url = spawn_zabbix_stub("selectHostGroups", "selectAcknowledgements").await;
        let mut source = source_with_url(SourceType::Zabbix, &url);
        source.name = "zbx70".to_string();

        let alerts = fetch_zabbix_alerts(&reqwest::Client::new(), &source).await.unwrap();
        assert_eq!(alerts.len(), 1);
        let alert = &alerts[0];
        assert_eq!(alert.severity, "critical");
        assert_eq!(alert.labels.get("hostgroup").map(String::as_str), Some("Linux servers"));
        assert_eq!(alert.labels.get("host").map(String::as_str), Some("srv-01"));
        assert_eq!(alert.labels.get("team").map(String::as_str), Some("sre"));
        // The acknowledgement carries a userid only: the name comes from user.get.
        assert_eq!(alert.annotations.get("acknowledgement").map(String::as_str), Some("on it"));
        assert_eq!(alert.labels.get("acknowledged_by").map(String::as_str), Some("Alice F"));
        assert_eq!(dialect_for("zbx70").groups_param, "selectHostGroups");
    }

    /// Zabbix 6.0 and older know neither `selectHostGroups` on trigger.get nor
    /// `selectAcknowledgements` on problem.get: both fall back one step.
    #[tokio::test]
    async fn test_zabbix_6_dialect() {
        let url = spawn_zabbix_stub("selectGroups", "selectAcknowledges").await;
        let mut source = source_with_url(SourceType::Zabbix, &url);
        source.name = "zbx60".to_string();

        let alerts = fetch_zabbix_alerts(&reqwest::Client::new(), &source).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(
            alerts[0].labels.get("hostgroup").map(String::as_str),
            Some("Linux servers")
        );
        assert_eq!(alerts[0].annotations.get("acknowledgement").map(String::as_str), Some("on it"));

        // What worked is remembered, so the probing costs one round-trip once.
        let dialect = dialect_for("zbx60");
        assert_eq!(dialect.groups_param, "selectGroups");
        assert_eq!(dialect.ack_param, Some("selectAcknowledges"));
    }

    /// One malformed entry must not take the whole payload down with it.
    #[tokio::test]
    async fn test_fetch_skips_unparseable_alerts() {
        const MIXED: &str = r#"[
            {"labels": {"alertname": "Good", "severity": "warning"},
             "annotations": {}, "status": {"state": "active"},
             "startsAt": "2026-09-04T10:00:00Z", "endsAt": "0001-01-01T00:00:00Z",
             "fingerprint": "ok"},
            {"labels": {"alertname": "NoFingerprint"}, "status": {"state": "active"}},
            {"fingerprint": "bare"}
        ]"#;
        let url = spawn_stub(MIXED, "[]").await;
        let source = source_with_url(SourceType::Alertmanager, &url);
        let alerts = fetch_source_alerts(&reqwest::Client::new(), &source).await.unwrap();

        // The entry without a fingerprint is dropped; the bare one survives on
        // defaults alone.
        let names: Vec<&str> = alerts.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Good", "Unknown"]);
        assert_eq!(alerts[1].status, "firing");
        assert_eq!(alerts[1].severity, "none");
    }

    #[test]
    fn test_zabbix_exposes_alertname() {
        // A template written once, like
        // "https://wiki/runbook/{{.Labels.alertname}}", has to resolve for
        // every source type, not just the Alertmanager-shaped ones.
        let problem: ZabbixProblem = serde_json::from_value(serde_json::json!({
            "eventid": "1", "objectid": "2", "name": "Disque plein",
            "severity": "5", "clock": "1788000000", "r_clock": "0",
            "suppressed": "0", "acknowledged": "0", "tags": []
        }))
        .unwrap();
        assert_eq!(problem.name, "Disque plein");
    }

    #[tokio::test]
    async fn test_fetch_from_a_plain_base_url() {
        let url = spawn_stub(ONE_ALERT, ONE_SILENCE).await;
        let source = source_with_url(SourceType::Alertmanager, &url);
        let alerts = fetch_source_alerts(&reqwest::Client::new(), &source)
            .await
            .expect("the API path must be appended to a plain base URL");

        assert_eq!(alerts.len(), 1);
        let alert = &alerts[0];
        assert_eq!(alert.name, "DiskFull");
        assert_eq!(alert.status, "silenced");
        // silencedBy has to deserialise for the comment to be resolved.
        assert_eq!(
            alert.annotations.get("silence_comment").map(String::as_str),
            Some("maintenance")
        );
        // Who silenced it comes from the silence's createdBy.
        assert_eq!(
            alert.annotations.get("silence_created_by").map(String::as_str),
            Some("alice")
        );
        // A javascript: generator URL must never reach the frontend.
        assert_eq!(alert.link_url, None);
    }

    #[tokio::test]
    async fn test_fetch_reports_http_status() {
        let url = spawn_stub(ONE_ALERT, ONE_SILENCE).await;
        let mut source = source_with_url(SourceType::Alertmanager, &url);
        source.url = format!("{}/nowhere/api/v2", url);

        let err = fetch_source_alerts(&reqwest::Client::new(), &source)
            .await
            .expect_err("a 404 must surface as a typed status error");
        let status = err
            .downcast_ref::<HttpStatusError>()
            .expect("the retry loop branches on this type");
        assert_eq!(status.status.as_u16(), 404);
    }

    fn sev_order() -> Vec<String> {
        crate::config::DisplayConfig::default().severity_order
    }

    fn source_with_url(source_type: SourceType, url: &str) -> Source {
        Source {
            name: "test".to_string(),
            source_type,
            url: url.to_string(),
            dashboard_url: None,
            link_template: None,
            alert_link_template: None,
            source_link: None,
            severity_label: "severity".to_string(),
            basic_auth: None,
            bearer_token: None,
            timeout: 15,
            retry_policy: Default::default(),
        }
    }

    #[test]
    fn test_am_api_base_appends_api_path() {
        let source = source_with_url(SourceType::Alertmanager, "http://127.0.0.1:9093/");
        assert_eq!(
            am_api_base(&source),
            ("http://127.0.0.1:9093/api/v2".to_string(), String::new())
        );
    }

    #[test]
    fn test_am_api_base_keeps_explicit_api_path() {
        for url in [
            "http://127.0.0.1:9093/api/v2",
            "http://127.0.0.1:9093/api/v2/",
            "http://127.0.0.1:9093/api/v2/alerts",
        ] {
            let source = source_with_url(SourceType::Alertmanager, url);
            assert_eq!(am_api_base(&source).0, "http://127.0.0.1:9093/api/v2");
        }
    }

    #[test]
    fn test_am_api_base_grafana() {
        let source = source_with_url(SourceType::Grafana, "http://grafana:3000");
        assert_eq!(
            am_api_base(&source).0,
            "http://grafana:3000/api/alertmanager/grafana/api/v2"
        );

        // An already-complete Grafana API URL is left untouched.
        let source = source_with_url(
            SourceType::Grafana,
            "http://grafana:3000/api/alertmanager/grafana/api/v2",
        );
        assert_eq!(
            am_api_base(&source).0,
            "http://grafana:3000/api/alertmanager/grafana/api/v2"
        );
    }

    #[test]
    fn test_am_api_base_preserves_query() {
        let source =
            source_with_url(SourceType::Alertmanager, "http://127.0.0.1:9093/api/v2/alerts?active=true");
        assert_eq!(
            am_api_base(&source),
            ("http://127.0.0.1:9093/api/v2".to_string(), "?active=true".to_string())
        );
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
            alert_link_url: None,
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
            alert_link_url: None,
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
            alert_link_url: None,
        };
        
        let template = "https://x.test/{{.Source}}/{{.Name}}?severity={{.Severity}}&status={{.Status}}";
        let result = apply_link_template(template, &alert).unwrap();
        assert_eq!(result, "https://x.test/Alertmanager/MyAlert?severity=critical&status=firing");
    }

    #[test]
    fn test_apply_link_template_encodes_values() {
        let mut alert = create_test_alert();
        alert.labels.insert("host".to_string(), "srv 01/prod&x".to_string());
        let result = apply_link_template("https://x.test/?h={{.Labels.host}}", &alert).unwrap();
        assert_eq!(result, "https://x.test/?h=srv%2001%2Fprod%26x");
    }

    #[test]
    fn test_apply_link_template_rejects_non_http_scheme() {
        let alert = create_test_alert();
        assert!(apply_link_template("javascript:alert(1)", &alert).is_none());
        assert!(apply_link_template("/relative/path", &alert).is_none());
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
        // An unresolved placeholder makes the whole template unusable: emitting
        // the URL with "{{.Labels.nonexistent}}" in it would be worse than
        // falling back to the next candidate link.
        let template = "https://example.com/{{.Labels.nonexistent}}";
        assert!(apply_link_template(template, &alert).is_none());
    }

    #[test]
    fn test_sanitize_link() {
        assert_eq!(sanitize_link(" https://x.test/a "), Some("https://x.test/a".to_string()));
        assert_eq!(sanitize_link("HTTP://x.test"), Some("HTTP://x.test".to_string()));
        assert!(sanitize_link("javascript:alert(1)").is_none());
        assert!(sanitize_link("").is_none());
    }

    #[test]
    fn test_zabbix_problem_link() {
        let mut source = source_with_url(SourceType::Zabbix, "https://zbx.test/zabbix");
        // No dashboard_url: derived from the source URL.
        assert_eq!(
            zabbix_problem_link(&source, "42"),
            "https://zbx.test/zabbix/zabbix.php?action=problem.view&triggerids[]=42"
        );
        // A dashboard_url with query params is stripped back to its base.
        source.dashboard_url = Some("https://zbx.test/zabbix/zabbix.php?action=problem.view".to_string());
        assert_eq!(
            zabbix_problem_link(&source, "42"),
            "https://zbx.test/zabbix/zabbix.php?action=problem.view&triggerids[]=42"
        );
        // Already targets triggers: left alone.
        source.dashboard_url = Some("https://zbx.test/custom?triggerids[]=7".to_string());
        assert_eq!(zabbix_problem_link(&source, "42"), "https://zbx.test/custom?triggerids[]=7");
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
                alert_link_url: None,
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
                alert_link_url: None,
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
                alert_link_url: None,
            },
        ];

        let groups = group_alerts(&alerts, &["namespace".to_string()], &sev_order());
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
                alert_link_url: None,
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
                alert_link_url: None,
            },
        ];

        let groups = group_alerts(&alerts, &["namespace".to_string(), "job".to_string()], &sev_order());
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
                alert_link_url: None,
            },
        ];

        let groups = group_alerts(&alerts, &[], &sev_order());
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
