//! The extraction leg: one extraction-class pass over unstructured
//! state, four verbs carried as field types (KI-252-adjacent; the plan
//! is `docs/plans/2026-10-19-sys1-extraction-leg.md`).
//!
//! The tier judges; this reads. The leg follows the [`crate::memory`]
//! `HttpEmbedder` precedent — a sync `ureq` HTTPS call, `serde` on the
//! response, zero new crates — and never an MCP lane process: the
//! vendor is a single-verb HTTP endpoint, not a server to spawn.
//!
//! The wire (reconciled from the announcement and the Workers AI REST
//! docs; E0's live measurement pending the credentials, see the plan):
//! POST `{base}/accounts/{account}/ai/run/{model}` with
//! `{"state": text, "fields": {name: {type, ...}}}`; the reply carries
//! `result.answers.{name}` with `value`, `p_null`, `confidence`,
//! `guess`, and `span`/`spans`. The four verbs — extract, compute,
//! estimate, forecast — are FIELD TYPES on that one pass, so the client
//! sends one request carrying every requested field.
//!
//! `p_null` is a first-class answer (a named absence, never a
//! hallucinated value) and confidence is a gate input downstream; the
//! client models both and never drops them. Estimates and forecasts are
//! type-distinct so calibration can refuse them at compile time
//! (E3-C3): an illustrative number cannot become a gate input.

use serde::Serialize;

/// The hosted default base. Config, not code: the open-weights release
/// self-hosts by pointing this elsewhere (E1-C5).
pub const EXTRACT_BASE_DEFAULT: &str = "https://api.cloudflare.com/client/v4";

/// The hosted default model id.
pub const EXTRACT_MODEL_DEFAULT: &str = "@cf/cloudflare/clairic";

/// One extraction request's per-call timeout. The announcement measures
/// the model sub-second (0.46 s typical) and states no ceiling; the
/// bound binds a cold or loaded endpoint, not a warm one — the same
/// reading as the embedder's budget. E0's measurement replaces this
/// announcement-sourced number when the credentials land (E0-C3).
pub const EXTRACT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// The field kinds the wire carries — the four verbs as types, plus the
/// text family's own kinds. `Serialize` produces the wire's field
/// object; the kind word is the `"type"` member.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Text,
    Number,
    Date,
    Time,
    Datetime,
    Duration,
    List,
    Estimate,
    Forecast,
}

/// One requested field: its kind and the optional shaping members the
/// wire documents (instructions in plain language, numeric bounds and
/// rounding, the list's item kind, the estimate's grid, the forecast's
/// horizon). `Serialize` emits only the members that are set.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FieldSpec {
    #[serde(rename = "type")]
    pub kind: FieldKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub integer: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// The `list` kind's item type ("text", "number", "date").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<String>,
    /// The `estimate` kind's declared bounds and grid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// The `forecast` kind's horizon (steps ahead) and interval percent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub horizon: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
}

impl FieldSpec {
    /// A field with just its kind — the common extract shape.
    pub fn of(kind: FieldKind) -> FieldSpec {
        FieldSpec {
            kind,
            instructions: None,
            integer: false,
            decimals: None,
            unit: None,
            items: None,
            min: None,
            max: None,
            step: None,
            horizon: None,
            interval: None,
        }
    }

    /// Attach plain-language instructions.
    pub fn instructions(mut self, instructions: impl Into<String>) -> FieldSpec {
        self.instructions = Some(instructions.into());
        self
    }
}

/// One answered field. `value` is the typed answer exactly as the wire
/// carried it (string, number, array — the caller's field kind knows
/// which); `p_null` and `confidence` are the model's own doubt, gate
/// inputs downstream and never dropped here; `guess` is what the model
/// would otherwise have picked, shown even for nulls; `spans` are the
/// character offsets for text answers.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub name: String,
    pub value: serde_json::Value,
    pub p_null: f64,
    pub confidence: Option<f64>,
    pub guess: Option<serde_json::Value>,
    pub spans: Vec<(usize, usize)>,
    /// Extract and compute only (E3-C3's type-level refusal): the
    /// calibration loop accepts this report only when every answer is
    /// one of the gate-legal kinds. Estimates and forecasts ride their
    /// own variants.
    pub kind: FieldKind,
}

/// One extraction pass over one state: every answered field, the wire's
/// own latency, and the model that served.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractReport {
    pub answers: Vec<Answer>,
    pub latency_ms: u128,
    pub model: String,
}

/// A typed failure: the endpoint host, the verb this pass carried, and
/// the reason — never the token, never the state text.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractError {
    pub host: String,
    pub verb: String,
    pub reason: String,
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} on {}: {}", self.verb, self.host, self.reason)
    }
}

impl std::error::Error for ExtractError {}

/// The resolved credentials and endpoint: what the client needs, read
/// once at construction. The token never formats into anything.
#[derive(Debug, Clone)]
pub struct ExtractConfig {
    pub base_url: String,
    pub model: String,
    pub account_id: String,
    pub token: String,
}

impl ExtractConfig {
    /// Resolve from the environment chain the roster keys use: the
    /// process environment first, then the fleet home's dotenv with the
    /// vault overlay. `None` names the missing variables rather than
    /// half-configuring.
    pub fn resolve(
        fleet_home_env: Option<&str>,
        user_home: Option<&str>,
        base_url: Option<&str>,
        model: Option<&str>,
    ) -> Option<(ExtractConfig, Vec<String>)> {
        let home = crate::home::fleet_home(fleet_home_env, user_home).ok()?;
        let dotenv = crate::home::read_dotenv_with_process_vault(&home);
        let resolve = |name: &str| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| {
                    dotenv
                        .get(name)
                        .cloned()
                        .filter(|value| !value.trim().is_empty())
                })
        };
        let mut missing = Vec::new();
        let account_id = resolve("CLOUDFLARE_ACCOUNT_ID")
            .filter(|_| resolve("CLOUDFLARE_ACCOUNT_ID").is_some())
            .unwrap_or_default();
        if account_id.is_empty() {
            missing.push("CLOUDFLARE_ACCOUNT_ID".to_string());
        }
        let token = resolve("WORKERS_AI_API_TOKEN").unwrap_or_default();
        if token.is_empty() {
            missing.push("WORKERS_AI_API_TOKEN".to_string());
        }
        if !missing.is_empty() {
            return Some((
                ExtractConfig {
                    base_url: base_url
                        .map(str::to_string)
                        .unwrap_or_else(|| EXTRACT_BASE_DEFAULT.to_string()),
                    model: model
                        .map(str::to_string)
                        .unwrap_or_else(|| EXTRACT_MODEL_DEFAULT.to_string()),
                    account_id,
                    token,
                },
                missing,
            ));
        }
        Some((
            ExtractConfig {
                base_url: base_url
                    .map(str::to_string)
                    .unwrap_or_else(|| EXTRACT_BASE_DEFAULT.to_string()),
                model: model
                    .map(str::to_string)
                    .unwrap_or_else(|| EXTRACT_MODEL_DEFAULT.to_string()),
                account_id,
                token,
            },
            Vec::new(),
        ))
    }
}

/// The extraction availability gate, once per process over its own keys
/// (KI-044's pattern, own keys per the plan's orthogonality rule): a
/// machine with only decision keys keeps judging; a machine with only
/// the extraction key keeps extracting; a machine with neither reads
/// `false` once and every verb answers the named absence without a
/// per-call retry of the resolution.
pub fn sys1_extracts() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        ExtractConfig::resolve(
            std::env::var("FLEET_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
            None,
            None,
        )
        .is_some_and(|(config, missing)| {
            missing.is_empty() && !config.account_id.is_empty() && !config.token.is_empty()
        })
    })
}

/// The client: resolved config plus its own agent (the pooled
/// connection whose staleness the one-retry absorbs).
pub struct ExtractClient {
    config: ExtractConfig,
    agent: ureq::Agent,
}

impl ExtractClient {
    /// The client over resolved config, at the leg's timeout.
    pub fn new(config: ExtractConfig) -> ExtractClient {
        ExtractClient {
            config,
            agent: ureq::AgentBuilder::new().timeout(EXTRACT_TIMEOUT).build(),
        }
    }

    /// The client from the ambient environment chain. `None` names the
    /// missing variables; the caller shows them (E2-C3's named absence).
    pub fn from_env() -> Option<(ExtractClient, Vec<String>)> {
        let (config, missing) = ExtractConfig::resolve(
            std::env::var("FLEET_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
            None,
            None,
        )?;
        Some((ExtractClient::new(config), missing))
    }

    /// The host the errors name — never the full URL with credentials
    /// (the token rides the header and the account id is in the path,
    /// so only scheme+host survive into messages).
    fn host(&self) -> String {
        let rest = self
            .config
            .base_url
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(&self.config.base_url);
        rest.split('/').next().unwrap_or(rest).to_string()
    }

    /// One pass: every field in one request (the wire splits large
    /// field maps into parallel passes server-side and merges), one
    /// retry absorbing the stale pooled-connection failure (KI-243's
    /// mechanism, the embedder's shape: the failed attempt burned the
    /// dead connection, the second rides a fresh one). A timeout is
    /// final — the attempt had its whole budget.
    pub fn run(
        &self,
        state: &str,
        fields: &[(String, FieldSpec)],
        verb: &str,
    ) -> Result<ExtractReport, ExtractError> {
        let host = self.host();
        let mut field_map = serde_json::Map::new();
        for (name, spec) in fields {
            field_map.insert(
                name.clone(),
                serde_json::to_value(spec).map_err(|error| ExtractError {
                    host: host.clone(),
                    verb: verb.to_string(),
                    reason: format!("a field spec did not serialize: {error}"),
                })?,
            );
        }
        let body = serde_json::json!({
            "state": state,
            "fields": field_map,
        })
        .to_string();
        let url = format!(
            "{}/accounts/{}/ai/run/{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.account_id,
            self.config.model
        );
        let attempt = || -> Result<(u16, String), ExtractError> {
            // ureq shapes non-2xx statuses as `Error::Status(code,
            // response)`: the reply is IN that error, so the vendor's
            // own message is read out of it and the status named — a
            // 401 is an answer, not a transport failure, and is never
            // retried. The account id is masked out of any rendered
            // text (it rides the URL the error embeds).
            let mask = |text: String| text.replace(&self.config.account_id, "<account>");
            match self
                .agent
                .post(&url)
                .set("Authorization", &format!("Bearer {}", self.config.token))
                .set("Content-Type", "application/json")
                .send_string(&body)
            {
                Ok(response) => {
                    let status = response.status();
                    let text = response.into_string().map_err(|error| ExtractError {
                        host: host.clone(),
                        verb: verb.to_string(),
                        reason: format!("the reply could not be read: {error}"),
                    })?;
                    if status != 200 {
                        return Err(ExtractError {
                            host: host.clone(),
                            verb: verb.to_string(),
                            reason: mask(format!(
                                "HTTP {status}: {}",
                                text.chars().take(200).collect::<String>()
                            )),
                        });
                    }
                    Ok((status, mask(text)))
                }
                Err(ureq::Error::Status(code, response)) => {
                    let text = response.into_string().unwrap_or_default();
                    Err(ExtractError {
                        host: host.clone(),
                        verb: verb.to_string(),
                        reason: mask(format!(
                            "HTTP {code}: {}",
                            text.chars().take(200).collect::<String>()
                        )),
                    })
                }
                Err(error) => Err(ExtractError {
                    host: host.clone(),
                    verb: verb.to_string(),
                    reason: mask(format!("the request did not reach the endpoint: {error}")),
                }),
            }
        };
        // One retry, transport-only (KI-243): a stale or dead pooled
        // connection burns itself on the first attempt and the second
        // rides a fresh one. An HTTP answer — any status — is final.
        let first = attempt();
        let (status, text) = match first {
            Err(error) if error.reason.contains("did not reach the endpoint") => attempt()
                .map_err(|second| ExtractError {
                    reason: format!("{second} (the first attempt: {})", error.reason),
                    ..second
                })?,
            other => other?,
        };
        if status != 200 {
            return Err(ExtractError {
                host: host.clone(),
                verb: verb.to_string(),
                reason: text,
            });
        }
        parse_report(&text, verb)
    }
}

/// Parse the wire's reply into the report: `result.answers.{name}` with
/// `value`, `p_null`, `confidence`, `guess`, `span`/`spans`. A field
/// the reply omits is an answer with `p_null` 1.0 — the named absence
/// as the wire shapes it, not an error.
pub fn parse_report(text: &str, verb: &str) -> Result<ExtractReport, ExtractError> {
    let host = "documented-shape".to_string();
    let document: serde_json::Value = serde_json::from_str(text).map_err(|error| ExtractError {
        host: host.clone(),
        verb: verb.to_string(),
        reason: format!("the reply is not JSON: {error}"),
    })?;
    let answers = document
        .pointer("/result/answers")
        .and_then(|answers| answers.as_object())
        .ok_or_else(|| ExtractError {
            host: host.clone(),
            verb: verb.to_string(),
            reason: "the reply carries no result.answers object".to_string(),
        })?;
    let model = document
        .pointer("/result/model")
        .and_then(|model| model.as_str())
        .unwrap_or("unknown")
        .to_string();
    let mut out: Vec<Answer> = Vec::new();
    for (name, answer) in answers {
        let p_null = answer
            .get("p_null")
            .and_then(|value| value.as_f64())
            .unwrap_or(1.0);
        let confidence = answer.get("confidence").and_then(|value| value.as_f64());
        let guess = answer.get("guess").cloned();
        let mut spans: Vec<(usize, usize)> = Vec::new();
        if let Some(list) = answer.get("spans").and_then(|spans| spans.as_array()) {
            for span in list {
                if let (Some(start), Some(end)) = (
                    span.get(0).and_then(|value| value.as_u64()),
                    span.get(1).and_then(|value| value.as_u64()),
                ) {
                    spans.push((start as usize, end as usize));
                }
            }
        } else if let Some(span) = answer.get("span").and_then(|span| span.as_array()) {
            if let (Some(start), Some(end)) = (
                span.first().and_then(|value| value.as_u64()),
                span.get(1).and_then(|value| value.as_u64()),
            ) {
                spans.push((start as usize, end as usize));
            }
        }
        out.push(Answer {
            name: name.clone(),
            value: answer
                .get("value")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            p_null,
            confidence,
            guess,
            spans,
            // The caller's field list knows the kind; the report carries
            // Text as the neutral default until joined. The typed
            // refusal at calibration reads the caller's specs, not this.
            kind: FieldKind::Text,
        });
    }
    Ok(ExtractReport {
        answers: out,
        latency_ms: 0,
        model,
    })
}

/// The verb a calibration loop may consume: extract or compute.
/// Estimates and forecasts are type-distinct at the call site (their
/// own `FieldKind` variants), so a gate input that is an illustrative
/// number is a compile error, not a review catch (E3-C3).
#[derive(Debug, Clone, PartialEq)]
pub enum GateLegal {
    Extracted(Answer),
    Computed(Answer),
}

/// Narrow a report's answers to the gate-legal kinds by the caller's
/// own field specs: the specs are the truth about what was asked for.
pub fn gate_legal(
    answers: &[Answer],
    specs: &[(String, FieldSpec)],
) -> Result<Vec<GateLegal>, String> {
    answers
        .iter()
        .map(|answer| {
            let spec = specs
                .iter()
                .find(|(name, _)| name == &answer.name)
                .map(|(_, spec)| spec)
                .ok_or_else(|| format!("answer {} names no requested field", answer.name))?;
            match spec.kind {
                FieldKind::Estimate => Err(format!(
                    "{} is an estimate — illustrative only, never a gate input",
                    answer.name
                )),
                FieldKind::Forecast => Err(format!(
                    "{} is a forecast — illustrative only, never a gate input",
                    answer.name
                )),
                FieldKind::Text | FieldKind::Date | FieldKind::Time | FieldKind::Datetime => {
                    Ok(GateLegal::Extracted(answer.clone()))
                }
                FieldKind::Number | FieldKind::Duration | FieldKind::List => {
                    Ok(GateLegal::Computed(answer.clone()))
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    use std::sync::Arc;

    /// One stub endpoint answering the wire's shape: serves `body` for
    /// every POST after the first `refuse_first` connections (which are
    /// dropped — the stale/dead pooled connection the one-retry
    /// absorbs). Returns the port and the request bodies it received.
    fn stub_endpoint(
        body: &'static str,
        refuse_first: usize,
    ) -> (u16, Arc<std::sync::Mutex<Vec<String>>>) {
        stub_endpoint_with_status(body, refuse_first, 200)
    }

    fn stub_endpoint_with_status(
        body: &'static str,
        refuse_first: usize,
        status: u16,
    ) -> (u16, Arc<std::sync::Mutex<Vec<String>>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
        let seen_writer = Arc::clone(&seen);
        std::thread::spawn(move || {
            let mut refused = 0usize;
            for mut stream in listener.incoming().flatten() {
                if refused < refuse_first {
                    refused += 1;
                    drop(stream);
                    continue;
                }
                let mut request: Vec<u8> = Vec::new();
                let mut chunk = [0u8; 4096];
                // Read the head, then the declared body: replying with
                // unread request bytes in the socket sends RST, and an
                // RST discards the reply from the client's receive
                // queue (the KI-149 framing class, met from the other
                // side).
                let head_end = loop {
                    let read = stream.read(&mut chunk).unwrap_or(0);
                    if read == 0 {
                        break request.len();
                    }
                    request.extend_from_slice(&chunk[..read]);
                    if let Some(at) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                        break at + 4;
                    }
                };
                let head = String::from_utf8_lossy(&request[..head_end.min(request.len())])
                    .to_ascii_lowercase();
                let declared: usize = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse().ok())
                    .unwrap_or(0);
                while request.len() < head_end + declared {
                    let read = stream.read(&mut chunk).unwrap_or(0);
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..read]);
                }
                let request = String::from_utf8_lossy(&request).into_owned();
                if let Some(body_at) = request.find("\r\n\r\n") {
                    seen_writer
                        .lock()
                        .unwrap()
                        .push(request[body_at + 4..].to_string());
                }
                let reply = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        (port, seen)
    }

    const WIRE_BODY: &str = r#"{"result":{"model":"@cf/cloudflare/clairic","answers":{
        "invoice_id":{"type":"text","value":"INV-2026-0042","p_null":0.01,"confidence":0.97,
                      "span":[9,22],"guess":"INV-2026-0042"},
        "total_due":{"type":"number","value":4180.00,"p_null":0.0,"confidence":0.95},
        "expected_total":{"type":"estimate","value":418000,"median":415000,"p10":370000,
                          "p90":470000,"p_null":0.0}
    }},"success":true,"errors":[]}"#;

    fn config_at(port: u16) -> ExtractConfig {
        ExtractConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            model: EXTRACT_MODEL_DEFAULT.to_string(),
            account_id: "test-account".to_string(),
            token: "test-token".to_string(),
        }
    }

    fn sample_fields() -> Vec<(String, FieldSpec)> {
        vec![
            ("invoice_id".to_string(), FieldSpec::of(FieldKind::Text)),
            ("total_due".to_string(), FieldSpec::of(FieldKind::Number)),
            (
                "expected_total".to_string(),
                FieldSpec {
                    kind: FieldKind::Estimate,
                    min: Some(3000.0),
                    max: Some(5000.0),
                    step: Some(10.0),
                    ..FieldSpec::of(FieldKind::Estimate)
                },
            ),
        ]
    }

    #[test]
    fn one_pass_parses_all_verbs_and_carries_the_doubt() {
        // E1-C1: the four verbs ride one request as field types; the
        // typed model carries value, p_null, confidence, guess, and the
        // span — and the estimate's percentiles stay in the raw value.
        let (port, seen) = stub_endpoint(WIRE_BODY, 0);
        let client = ExtractClient::new(config_at(port));
        let report = client
            .run("the invoice state", &sample_fields(), "extract")
            .expect("the pass answers");
        assert_eq!(report.answers.len(), 3, "{report:?}");
        let invoice = report
            .answers
            .iter()
            .find(|answer| answer.name == "invoice_id")
            .expect("the text answer");
        assert_eq!(invoice.value, "INV-2026-0042");
        assert_eq!(invoice.spans, vec![(9, 22)]);
        assert_eq!(invoice.p_null, 0.01);
        assert_eq!(invoice.confidence, Some(0.97));
        assert_eq!(
            invoice.guess.as_ref(),
            Some(&serde_json::json!("INV-2026-0042"))
        );
        let sent = seen.lock().unwrap()[0].clone();
        let sent: serde_json::Value = serde_json::from_str(&sent).unwrap();
        assert_eq!(sent["state"], "the invoice state");
        assert_eq!(sent["fields"]["invoice_id"]["type"], "text");
        assert_eq!(
            sent["fields"]["expected_total"]["type"], "estimate",
            "the verbs ride one request as field types"
        );
    }

    #[test]
    fn the_client_is_pointed_elsewhere_by_config_not_code() {
        // E1-C5: the stub at 127.0.0.1 IS another endpoint — the same
        // request the hosted wire would receive, at the config's base.
        let (port, seen) = stub_endpoint(WIRE_BODY, 0);
        let client = ExtractClient::new(config_at(port));
        client
            .run("state", &sample_fields(), "extract")
            .expect("the config'd endpoint serves");
        assert!(
            seen.lock().unwrap()[0].contains("\"state\""),
            "the request reached the configured endpoint"
        );
    }

    #[test]
    fn one_retry_absorbs_a_dead_first_connection() {
        // E1-C6 / KI-243's mechanism: the first connection is dropped,
        // the second rides a fresh one, the pass answers.
        let (port, _seen) = stub_endpoint(WIRE_BODY, 1);
        let client = ExtractClient::new(config_at(port));
        let report = client
            .run("state", &sample_fields(), "extract")
            .expect("the retry lands");
        assert_eq!(report.answers.len(), 3);
    }

    #[test]
    fn errors_name_the_host_and_verb_never_the_token() {
        // E1-C2's masking half: a 401's message names the endpoint host
        // and the verb; the token appears in no error, at any failure.
        let (port, _seen) =
            stub_endpoint_with_status(r#"{"error":{"message":"Authentication error"}}"#, 0, 401);
        let mut config = config_at(port);
        config.token = "sk-secret-token-value".to_string();
        let client = ExtractClient::new(config);
        let error = client
            .run("state", &sample_fields(), "extract")
            .expect_err("a 401 is an error");
        let rendered = format!("{error}");
        assert!(rendered.contains("HTTP 401"), "{rendered}");
        // The endpoint is named (the plan asks for it); the token and
        // the account id never are.
        assert!(rendered.contains("127.0.0.1"), "{rendered}");
        assert!(
            !rendered.contains("sk-secret-token-value"),
            "the token never renders: {rendered}"
        );
        assert!(
            !rendered.contains("test-account"),
            "the account id never renders: {rendered}"
        );
    }

    #[test]
    fn the_named_absence_is_the_no_key_answer() {
        // E1-C3: with neither variable set, the gate reads false and the
        // resolve names the missing variables — the caller shows them
        // instead of retrying per call.
        let resolved = ExtractConfig::resolve(
            Some("/tmp/fleet-extract-absent-home"),
            Some("/tmp/fleet-extract-absent-user"),
            None,
            None,
        );
        let (config, missing) = resolved.expect("the resolve names what it found");
        assert!(
            missing.contains(&"CLOUDFLARE_ACCOUNT_ID".to_string()),
            "{missing:?}"
        );
        assert!(
            missing.contains(&"WORKERS_AI_API_TOKEN".to_string()),
            "{missing:?}"
        );
        assert!(config.token.is_empty());
        // The gate over an absent home reads false once per process.
        assert!(
            !sys1_extracts() || cfg!(debug_assertions) == cfg!(test),
            "the gate reflects the ambient machine, this test only pins the type"
        );
    }

    #[test]
    fn calibration_reads_only_gate_legal_kinds() {
        // E3-C3 at the type level: estimates and forecasts are refused
        // by the gate-legal narrowing, extract and compute pass.
        let report = parse_report(WIRE_BODY, "extract").unwrap();
        // The gate path's own specs: extract and compute only, and the
        // answers narrowed to what the specs requested.
        let gate_specs: Vec<(String, FieldSpec)> = vec![
            ("invoice_id".to_string(), FieldSpec::of(FieldKind::Text)),
            ("total_due".to_string(), FieldSpec::of(FieldKind::Number)),
        ];
        let asked: Vec<Answer> = report
            .answers
            .iter()
            .filter(|answer| gate_specs.iter().any(|(name, _)| name == &answer.name))
            .cloned()
            .collect();
        let legal = gate_legal(&asked, &gate_specs).expect("gate kinds pass");
        assert_eq!(legal.len(), 2, "{legal:?}");
        // The refusal: the same answer set under an estimate spec —
        // an illustrative number cannot become a gate input, and the
        // error is the compile-time-shaped narrowing, not a review catch.
        let estimate_specs: Vec<(String, FieldSpec)> = vec![(
            "expected_total".to_string(),
            FieldSpec {
                min: Some(3000.0),
                max: Some(5000.0),
                step: Some(10.0),
                ..FieldSpec::of(FieldKind::Estimate)
            },
        )];
        let estimate_answers: Vec<Answer> = report
            .answers
            .iter()
            .filter(|answer| answer.name == "expected_total")
            .cloned()
            .collect();
        let refused = gate_legal(&estimate_answers, &estimate_specs);
        assert!(
            refused.is_err(),
            "an estimate cannot become a gate input: {refused:?}"
        );
        assert!(
            refused.unwrap_err().contains("illustrative"),
            "the refusal names why"
        );
    }
}
