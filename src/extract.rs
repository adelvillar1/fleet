//! The extraction leg: reading structured facts out of unstructured text
//! on sys1's own hosted models (the plan is
//! `docs/plans/2026-10-19-sys1-extraction-leg.md`; re-pointed 2026-10-25
//! by owner ruling — the Cloudflare Clairic client was the wrong vendor
//! for a capability the tier's own vendor already serves, and the wrong
//! shape does not stay).
//!
//! The tier judges; this reads — but it reads with the tier's models,
//! on the pattern jobhound measured for exactly this problem
//! (`jobhound/apps/worker/src/jobhound/requirement_pass.py`): cheap
//! deterministic passes first, one decision call only where evidence
//! lives, asymmetric reconciliation, and every answer carrying the
//! sentence it was read from. What the models can and cannot do was
//! measured there, not assumed: entity spans are reliable (0.81–0.93
//! confidence on genuine claims); relations and free-text structures
//! are not; confidence tracks correctness. So the leg is a pipeline:
//!
//!   pass 1  LOCATE — one hosted GLiNER 2.5 Multi entity call with the
//!           requested fields as entity names; candidates come back as
//!           span text with character offsets and the model's own
//!           confidence (half-open offsets, 50k-char input cap).
//!   pass 2  SEGMENT — pure Rust: cut the state into claim-sized
//!           segments on sentence ends, merging fragments (the jobhound
//!           segmentation, minus its posting-specific section markers).
//!   pass 3  VERIFY — per segment carrying a candidate, one decision
//!           call over the sys1 chain (glide → drex → jev, the router's
//!           own measured default order, filtered by the keys present):
//!           one noul head per field, "does this segment state it".
//!   pass 4  RECONCILE — asymmetrically, the measured floors: a span is
//!           asserted at p ≥ 0.85 (asserting is a strong claim), vetoed
//!           at p ≤ 0.35 (a veto only removes a claim, and a missed
//!           value is a named absence, never a fabricated one), and
//!           between the floors it is contested — one adjudication call
//!           to the next verifier in the chain (capped), and if still
//!           unsettled the span ships marked contested with its
//!           confidence. The calling model is the loop's own reader; it
//!           sees the mark and the quote and judges.
//!
//! The absence semantics come from the shape, not from a `p_null` field:
//! a field with no surviving span is absent BY NAME, with the reason —
//! no candidate above the entity threshold, or none that cleared the
//! verify floors. Estimates and forecasts do not exist here at all: the
//! field-kind enum has no variant for them, so an illustrative number
//! cannot even be requested (E3-C3's refusal, now structural — the
//! Clairic enum carried the variants; this one does not).

/// The hosted default base: the tier's own vendor, the key the decision
/// gates already resolve. Config, not code.
pub const EXTRACT_BASE_DEFAULT: &str = "https://api.fastino.ai";

/// The hosted default entity model (the extraction-class GLiNER, per
/// the vendor docs; the Decide model is its classification sibling and
/// stays the `decide` provider's business).
pub const ENTITY_MODEL_DEFAULT: &str = "fastino/gliner2.5-multi-v1";

/// The entity pass's input cap (the vendor's own documented ceiling —
/// longer inputs are the async endpoint's job, which the leg does not
/// grow until a caller needs it).
pub const ENTITY_INPUT_CAP: usize = 50_000;

/// One HTTP call's timeout (either pass). The bound binds a cold or
/// loaded endpoint, not a warm one — the embedder's reading.
pub const EXTRACT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// The measured reconcile floors (jobhound's requirement_pass: genuine
/// claims scored 0.85+; the wrong verdicts sat at 0.40–0.55, and a veto
/// only removes a claim, so it fires cheap).
pub const ACCEPT_FLOOR: f64 = 0.85;
pub const VETO_FLOOR: f64 = 0.35;

/// The segmentation constants (jobhound's, measured): fragments smaller
/// than this merge into their neighbour rather than costing a call; a
/// segment reads at most this many characters, which covered every
/// claim measured.
pub const MIN_SEGMENT_CHARS: usize = 40;
pub const MAX_SEGMENT_CHARS: usize = 900;

/// Budgets, bounding one call — not one run: a long state can carry
/// many segments, but the verify pass stops at this many segment calls
/// (the jobhound ceiling's reading), and adjudication at this many.
pub const MAX_CARRYING_SEGMENTS: usize = 24;
pub const MAX_ADJUDICATIONS: usize = 3;

/// The field kinds the leg reads. Every kind is an entity-shaped claim
/// — the value is span TEXT from the state, never a normalized or
/// inferred number (normalizing is the reading model's job, with the
/// quote in hand). `List` shapes the value as every survivor in reading
/// order; the other kinds name the kind of thing the span should be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Number,
    Date,
    Time,
    Datetime,
    Duration,
    List,
}

impl FieldKind {
    /// The type hint folded into an entity description when the caller
    /// gave no instructions of their own.
    pub fn hint(&self) -> &'static str {
        match self {
            FieldKind::Text => "a value named in the text",
            FieldKind::Number => "a number or amount written in the text",
            FieldKind::Date => "a calendar date written in the text",
            FieldKind::Time => "a time of day written in the text",
            FieldKind::Datetime => "a date with time written in the text",
            FieldKind::Duration => "a length of time written in the text",
            FieldKind::List => "an item in a list of such values",
        }
    }
}

/// One requested field: its kind and optional plain-language
/// instructions — which together are the entity description the locate
/// pass reads.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSpec {
    pub kind: FieldKind,
    pub instructions: Option<String>,
}

impl FieldSpec {
    pub fn of(kind: FieldKind) -> FieldSpec {
        FieldSpec {
            kind,
            instructions: None,
        }
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> FieldSpec {
        self.instructions = Some(instructions.into());
        self
    }

    /// The entity description for the locate pass: the caller's own
    /// words when given, else the kind's hint.
    pub fn description(&self, name: &str) -> String {
        match &self.instructions {
            Some(instructions) if !instructions.trim().is_empty() => instructions.clone(),
            _ => format!("{name}: {}", self.kind.hint()),
        }
    }
}

/// One candidate span the locate pass returned. Offsets are half-open
/// character offsets into the state as sent.
#[derive(Debug, Clone, PartialEq)]
pub struct Located {
    pub field: String,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f64,
}

/// One answered field. `value` is span TEXT from the state (an array of
/// span texts for `List`, `Null` with `absent_reason` when nothing
/// survived) — the leg never invents or normalizes a value. `confidence`
/// is the verify pass's p(true) for the best survivor; `quote` is the
/// sentence the value was read from, trimmed around the span; `contested`
/// marks a span neither floor settled.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub name: String,
    pub value: serde_json::Value,
    pub confidence: Option<f64>,
    pub spans: Vec<(usize, usize)>,
    pub quote: String,
    pub contested: bool,
    pub absent_reason: Option<String>,
}

/// One run's counts — the audit trail of the passes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExtractStats {
    pub segments: usize,
    pub verify_calls: usize,
    pub adjudications: usize,
    pub vetoed: usize,
    pub contested: usize,
}

/// One extraction over one state: every requested field answered, the
/// latency, the models that served, and the counts.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractReport {
    pub answers: Vec<Answer>,
    pub latency_ms: u128,
    pub models: Vec<String>,
    pub stats: ExtractStats,
}

/// A typed failure: the endpoint host, the pass that failed, and the
/// reason — never the key.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractError {
    pub host: String,
    pub pass: String,
    pub reason: String,
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} on {}: {}", self.pass, self.host, self.reason)
    }
}

impl std::error::Error for ExtractError {}

/// One verifier in the decision chain: sys1's own systemone endpoints,
/// shaped per provider (jev takes a `{"text": ..}` state and an `id` on
/// every question; glide and drex take the plain-string state).
#[derive(Debug, Clone, PartialEq)]
pub struct Verifier {
    pub name: String,
    pub url: String,
    pub model: String,
    pub key: String,
    pub state_object: bool,
}

impl Verifier {
    fn glide(key: &str, base_url: &str) -> Verifier {
        Verifier {
            name: "glide".to_string(),
            url: format!("{base_url}/v1/systemone"),
            model: "fastino/glide".to_string(),
            key: key.to_string(),
            state_object: false,
        }
    }

    fn drex(key: &str) -> Option<Verifier> {
        let key = key.trim();
        if key.is_empty() {
            return None;
        }
        Some(Verifier {
            name: "drex".to_string(),
            url: "https://drex.nace.ai/v1/systemone".to_string(),
            model: "drex-v1.5".to_string(),
            key: key.to_string(),
            state_object: false,
        })
    }

    fn jev(key: &str) -> Option<Verifier> {
        let key = key.trim();
        if key.is_empty() {
            return None;
        }
        Some(Verifier {
            name: "jev".to_string(),
            url: "https://api.typesafe.ai/v1/systemone".to_string(),
            model: "jev-1.13.0".to_string(),
            key: key.to_string(),
            state_object: true,
        })
    }
}

/// The resolved credentials and endpoints. The fastino key is required
/// (it serves both the locate pass and the glide verifier); the drex and
/// jev keys extend the verify chain when present. No key value ever
/// formats into anything but its own header.
#[derive(Debug, Clone)]
pub struct ExtractConfig {
    pub base_url: String,
    pub entity_model: String,
    pub token: String,
    pub verifiers: Vec<Verifier>,
}

impl ExtractConfig {
    /// Resolve from the environment chain the roster keys use: the
    /// process environment first, then the fleet home's dotenv with the
    /// vault overlay. `Some` always; the missing list names what a
    /// complete configuration still needs (the named absence — the
    /// caller shows it instead of retrying per call).
    pub fn resolve(
        fleet_home_env: Option<&str>,
        user_home: Option<&str>,
        base_url: Option<&str>,
        entity_model: Option<&str>,
    ) -> Option<(ExtractConfig, Vec<String>)> {
        let home = crate::home::fleet_home(fleet_home_env, user_home).ok()?;
        let dotenv = crate::home::read_dotenv_with_process_vault(&home);
        let resolve = |name: &str| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| dotenv.get(name).cloned().filter(|v| !v.trim().is_empty()))
        };
        let token = resolve("FASTINO_API_KEY").unwrap_or_default();
        let mut missing = Vec::new();
        if token.is_empty() {
            missing.push("FASTINO_API_KEY".to_string());
        }
        let base_url = base_url
            .map(str::to_string)
            .unwrap_or_else(|| EXTRACT_BASE_DEFAULT.to_string());
        let entity_model = entity_model
            .map(str::to_string)
            .unwrap_or_else(|| ENTITY_MODEL_DEFAULT.to_string());
        let mut verifiers = vec![Verifier::glide(&token, &base_url)];
        if let Some(drex) = Verifier::drex(&resolve("DREX_API_KEY").unwrap_or_default()) {
            verifiers.push(drex);
        }
        if let Some(jev) = Verifier::jev(&resolve("TYPESAFE_API_KEY").unwrap_or_default()) {
            verifiers.push(jev);
        }
        Some((
            ExtractConfig {
                base_url,
                entity_model,
                token,
                verifiers,
            },
            missing,
        ))
    }
}

/// The extraction availability gate, once per process over its own key
/// (KI-044's pattern): a machine with the fastino key reads and
/// verifies; a machine without answers the named absence without a
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
        .is_some_and(|(config, missing)| missing.is_empty() && !config.token.is_empty())
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
    /// missing variables; the caller shows them (the named absence).
    pub fn from_env() -> Option<(ExtractClient, Vec<String>)> {
        let (config, missing) = ExtractConfig::resolve(
            std::env::var("FLEET_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
            None,
            None,
        )?;
        Some((ExtractClient::new(config), missing))
    }

    /// The host an endpoint names in errors — never the full URL with
    /// credentials.
    fn host_of(url: &str) -> String {
        let rest = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
        rest.split('/').next().unwrap_or(rest).to_string()
    }

    /// One POST of JSON, one retry, transport-only (KI-243's mechanism:
    /// the failed attempt burned the stale pooled connection, the
    /// second rides a fresh one). An HTTP answer — any status — is
    /// final and is never retried. The key value is masked out of any
    /// rendered text.
    fn post_json(
        &self,
        url: &str,
        key: &str,
        body: &serde_json::Value,
        pass: &str,
    ) -> Result<serde_json::Value, ExtractError> {
        let host = Self::host_of(url);
        let mask = |text: String| text.replace(key, "<key>");
        let payload = body.to_string();
        let attempt = || -> Result<serde_json::Value, ExtractError> {
            let fail = |reason: String| ExtractError {
                host: host.clone(),
                pass: pass.to_string(),
                reason: mask(reason),
            };
            match self
                .agent
                .post(url)
                .set("Authorization", &format!("Bearer {key}"))
                .set("Content-Type", "application/json")
                .send_string(&payload)
            {
                Ok(response) => {
                    let text = response
                        .into_string()
                        .map_err(|error| fail(format!("the reply could not be read: {error}")))?;
                    serde_json::from_str(&text)
                        .map_err(|error| fail(format!("the reply is not JSON: {error}")))
                }
                Err(ureq::Error::Status(code, response)) => {
                    let text = response.into_string().unwrap_or_default();
                    Err(fail(format!(
                        "HTTP {code}: {}",
                        text.chars().take(200).collect::<String>()
                    )))
                }
                Err(error) => Err(fail(format!(
                    "the request did not reach the endpoint: {error}"
                ))),
            }
        };
        let first = attempt();
        match first {
            Err(error) if error.reason.contains("did not reach the endpoint") => {
                attempt().map_err(|second| ExtractError {
                    reason: format!("{second} (the first attempt: {})", error.reason),
                    ..second
                })
            }
            other => other,
        }
    }

    /// Pass 1 — the entity call. The requested fields ride the schema
    /// as entity names; the reply's `choices[0].message.content` is a
    /// JSON string carrying the spans.
    fn locate(
        &self,
        state: &str,
        fields: &[(String, FieldSpec)],
    ) -> Result<Vec<Located>, ExtractError> {
        let url = format!(
            "{}/v1/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );
        let state = take_chars(state, ENTITY_INPUT_CAP);
        let mut entities = Vec::new();
        for (name, spec) in fields {
            entities.push(serde_json::json!({
                "name": name,
                "description": spec.description(name),
            }));
        }
        let body = serde_json::json!({
            "model": self.config.entity_model,
            "messages": [{"role": "user", "content": state}],
            "schema": {"entities": entities},
            "threshold": 0.5,
            "include_confidence": true,
            "include_spans": true,
            "store": false,
        });
        let reply = self.post_json(&url, &self.config.token, &body, "locate")?;
        let content = reply
            .pointer("/choices/0/message/content")
            .and_then(|content| content.as_str())
            .ok_or_else(|| ExtractError {
                host: Self::host_of(&url),
                pass: "locate".to_string(),
                reason: "the reply carries no choices[0].message.content".to_string(),
            })?;
        Ok(parse_entity_content(content))
    }

    /// Pass 3 — one verify call over one segment: one noul head per
    /// distinct field carrying a candidate there. Returns p(true) per
    /// field asked.
    fn verify(
        &self,
        verifier: &Verifier,
        segment_text: &str,
        heads: &[(String, String)],
        pass: &str,
    ) -> Result<Vec<(String, f64)>, ExtractError> {
        let mut questions = serde_json::Map::new();
        for (name, description) in heads {
            let mut head = serde_json::json!({
                "type": "noul",
                "instructions": format!("Does this text state {description}?"),
                "criteria": {
                    "true": format!("Yes, {description} is stated here."),
                    "false": "No.",
                },
            });
            if verifier.state_object {
                head["id"] = serde_json::Value::String(name.clone());
            }
            questions.insert(name.clone(), head);
        }
        let state = if verifier.state_object {
            serde_json::json!({ "text": segment_text })
        } else {
            serde_json::Value::String(segment_text.to_string())
        };
        let body = serde_json::json!({
            "model": verifier.model,
            "state": state,
            "questions": questions,
        });
        let reply = self.post_json(&verifier.url, &verifier.key, &body, pass)?;
        // The systemone family answers at `answers.{id}`; the
        // chat-completions wrapping is tolerated, the same read sys1's
        // own parsers make.
        let answers = unwrap_content(&reply)
            .and_then(|inner| inner.get("answers").cloned())
            .or_else(|| reply.get("answers").cloned())
            .unwrap_or(serde_json::Value::Null);
        Ok(heads
            .iter()
            .map(|(name, _)| {
                let p = answers.get(name).and_then(p_true).unwrap_or(0.0);
                (name.clone(), p)
            })
            .collect())
    }

    /// The pipeline: locate, segment, verify, reconcile. Every
    /// requested field is answered — with its value and quote, or by
    /// name with the reason it is absent.
    pub fn run(
        &self,
        state: &str,
        fields: &[(String, FieldSpec)],
    ) -> Result<ExtractReport, ExtractError> {
        let started = std::time::Instant::now();
        let located = self.locate(state, fields)?;
        let asked: std::collections::HashSet<&str> =
            fields.iter().map(|(name, _)| name.as_str()).collect();
        let candidates: Vec<&Located> = located
            .iter()
            .filter(|span| asked.contains(span.field.as_str()))
            .collect();
        let mut stats = ExtractStats::default();
        let mut used_verifiers: Vec<String> = Vec::new();
        if candidates.is_empty() {
            // Nothing above the entity threshold: every field answers
            // absent, and the verify pass costs nothing.
            let answers = fields
                .iter()
                .map(|(name, _)| Answer {
                    name: name.clone(),
                    value: serde_json::Value::Null,
                    confidence: None,
                    spans: Vec::new(),
                    quote: String::new(),
                    contested: false,
                    absent_reason: Some("no candidate span above the entity threshold".to_string()),
                })
                .collect();
            return Ok(ExtractReport {
                answers,
                latency_ms: started.elapsed().as_millis(),
                models: vec![self.config.entity_model.clone()],
                stats,
            });
        }

        // Pass 2 — segments; only the ones carrying a candidate cost a
        // call, bounded by the budget.
        let segs = segments(state);
        stats.segments = segs.len();
        let mut carrying: Vec<(&Segment, Vec<&Located>)> = Vec::new();
        for segment in &segs {
            let spans: Vec<&Located> = candidates
                .iter()
                .filter(|span| segment.start <= span.start && span.end <= segment.end)
                .copied()
                .collect();
            if !spans.is_empty() {
                carrying.push((segment, spans));
            }
        }
        carrying.truncate(MAX_CARRYING_SEGMENTS);

        // Pass 3 + 4 — verify per segment, adjudicate contested spans.
        let mut survivors: Vec<(Located, f64, bool)> = Vec::new();
        let mut adjudications = 0usize;
        for (segment, spans) in carrying.iter() {
            let mut head_list: Vec<(String, String)> = Vec::new();
            for span in spans {
                if !head_list.iter().any(|(name, _)| name == &span.field) {
                    let spec = fields
                        .iter()
                        .find(|(name, _)| name == &span.field)
                        .map(|(_, spec)| spec)
                        .expect("candidates are filtered to asked fields");
                    head_list.push((span.field.clone(), spec.description(&span.field)));
                }
            }
            // A segment reads at most MAX_SEGMENT_CHARS — the measured
            // claim size; the budget, not the state, bounds the rest.
            let read = take_chars(&segment.text, MAX_SEGMENT_CHARS);
            let primary = &self.config.verifiers[0];
            let verdicts = self.verify(primary, read, &head_list, "verify")?;
            stats.verify_calls += 1;
            if !used_verifiers.contains(&primary.name) {
                used_verifiers.push(primary.name.clone());
            }
            let p_of = |field: &str| {
                verdicts
                    .iter()
                    .find(|(name, _)| name == field)
                    .map(|(_, p)| *p)
                    .unwrap_or(0.0)
            };
            for span in spans {
                let p = p_of(&span.field);
                if p >= ACCEPT_FLOOR {
                    survivors.push(((*span).clone(), p, false));
                    continue;
                }
                if p <= VETO_FLOOR {
                    // A veto only removes a claim; the field's absence
                    // is named downstream, never filled.
                    stats.vetoed += 1;
                    continue;
                }
                // Contested: one adjudication to the next verifier in
                // the chain, capped. Still unsettled, the span ships
                // marked, with the stronger of the two evidences.
                let (p_final, contested) =
                    if adjudications < MAX_ADJUDICATIONS && self.config.verifiers.len() > 1 {
                        adjudications += 1;
                        let second = &self.config.verifiers[1];
                        let second_p = self
                            .verify(second, read, &head_list, "verify")?
                            .iter()
                            .find(|(name, _)| name == &span.field)
                            .map(|(_, p)| *p)
                            .unwrap_or(0.0);
                        if !used_verifiers.contains(&second.name) {
                            used_verifiers.push(second.name.clone());
                        }
                        if second_p >= ACCEPT_FLOOR {
                            (second_p, false)
                        } else {
                            (p.max(second_p), true)
                        }
                    } else {
                        (p, true)
                    };
                if contested {
                    stats.contested += 1;
                }
                survivors.push(((*span).clone(), p_final, contested));
            }
        }
        stats.adjudications = adjudications;
        let mut models = vec![self.config.entity_model.clone()];
        models.extend(used_verifiers);

        // Pass 4's other half — assemble the answers: every requested
        // field, best survivor first; the absence named when nothing
        // survived.
        let mut answers = Vec::new();
        for (name, spec) in fields {
            let mut mine: Vec<&(Located, f64, bool)> = survivors
                .iter()
                .filter(|(span, _, _)| &span.field == name)
                .collect();
            mine.sort_by(|(a, pa, _), (b, pb, _)| {
                pb.partial_cmp(pa)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.start.cmp(&b.start))
            });
            if mine.is_empty() {
                let reason = if candidates.iter().any(|span| &span.field == name) {
                    "no candidate cleared the verify floors"
                } else {
                    "no candidate span above the entity threshold"
                };
                answers.push(Answer {
                    name: name.clone(),
                    value: serde_json::Value::Null,
                    confidence: None,
                    spans: Vec::new(),
                    quote: String::new(),
                    contested: false,
                    absent_reason: Some(reason.to_string()),
                });
                continue;
            }
            let (best, confidence, contested) = &mine[0];
            let spans = mine
                .iter()
                .map(|(span, _, _)| (span.start, span.end))
                .collect();
            let value = if spec.kind == FieldKind::List {
                serde_json::Value::Array(
                    mine.iter()
                        .map(|(span, _, _)| serde_json::Value::String(span.text.clone()))
                        .collect(),
                )
            } else {
                serde_json::Value::String(best.text.clone())
            };
            answers.push(Answer {
                name: name.clone(),
                value,
                confidence: Some(*confidence),
                spans,
                quote: quote_at(state, best.start, best.end),
                contested: *contested,
                absent_reason: None,
            });
        }
        Ok(ExtractReport {
            answers,
            latency_ms: started.elapsed().as_millis(),
            models,
            stats,
        })
    }
}

/// The entity reply's content is a JSON string; unwrap it into the
/// spans, half-open offsets as sent. A label absent from the reply is
/// simply no candidates for that field.
pub fn parse_entity_content(content: &str) -> Vec<Located> {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) else {
        return Vec::new();
    };
    let Some(entities) = parsed.get("entities").and_then(|e| e.as_object()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (field, spans) in entities {
        let Some(list) = spans.as_array() else {
            continue;
        };
        for span in list {
            let (Some(text), Some(start), Some(end)) = (
                span.get("text").and_then(|v| v.as_str()),
                span.get("start").and_then(|v| v.as_u64()),
                span.get("end").and_then(|v| v.as_u64()),
            ) else {
                continue;
            };
            out.push(Located {
                field: field.clone(),
                text: text.to_string(),
                start: start as usize,
                end: end as usize,
                confidence: span
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
            });
        }
    }
    out
}

/// The systemone family may wrap its answers object in the
/// chat-completions content shape; unwrap the string when present (the
/// read sys1's own parsers make).
fn unwrap_content(reply: &serde_json::Value) -> Option<serde_json::Value> {
    let content = reply
        .get("choices")?
        .get(0)?
        .get("message")?
        .get("content")?;
    let text = content.as_str()?;
    serde_json::from_str(text).ok()
}

/// p(true) from a noul answer entry, across the family's shapes:
/// glide/drex carry the probability itself (`noul: p`, 1.0 = certainly
/// true, 0.0 = certainly false); jev may carry a boolean with a
/// confidence; a yes/no string is tolerated the way sys1's parser
/// tolerates one. A missing confidence on a bare boolean is certainty —
/// the model said so without hedging.
fn p_true(entry: &serde_json::Value) -> Option<f64> {
    let confidence = entry.get("confidence").and_then(|v| v.as_f64());
    match entry.get("noul") {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::Bool(b)) => Some(match (b, confidence) {
            (true, Some(c)) => c,
            (true, None) => 1.0,
            (false, Some(c)) => 1.0 - c,
            (false, None) => 0.0,
        }),
        Some(serde_json::Value::String(word)) => match word.to_ascii_lowercase().as_str() {
            "yes" => Some(confidence.unwrap_or(1.0)),
            "no" => Some(1.0 - confidence.unwrap_or(1.0)),
            _ => None,
        },
        _ => None,
    }
}

/// A claim-sized segment with its offsets into the state.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Pass 2 — the jobhound segmentation: cut on sentence ends, merge
/// fragments into their neighbour rather than costing a call. (The
/// original also cut on posting-section markers; those are a job
/// posting's vocabulary, not the state's, so they stay there.)
pub fn segments(text: &str) -> Vec<Segment> {
    let mut cuts: Vec<usize> = vec![0, text.len()];
    let bytes = text.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if matches!(bytes[index], b'.' | b'!' | b'?') {
            let mut after = index + 1;
            while after < bytes.len() && matches!(bytes[after], b'.' | b'!' | b'?') {
                after += 1;
            }
            // A sentence end is the punctuation, the whitespace that
            // follows, and a word-like continuation. Jobhound's regex
            // named lowercase only — right for postings whose claims
            // run on, wrong for prose, where "end. The" is the normal
            // boundary. Uppercase joins the set; the abbreviation case
            // ("Mr. Smith…") survives as a fragment, which merges into
            // its neighbour instead of costing a call.
            if after < bytes.len()
                && bytes[after] == b' '
                && after + 1 < bytes.len()
                && matches!(
                    bytes[after + 1],
                    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'"' | b'\'' | b'('
                )
            {
                cuts.push(after + 1);
                index = after + 1;
                continue;
            }
        }
        index += 1;
    }
    cuts.sort_unstable();
    cuts.dedup();
    let mut out: Vec<Segment> = Vec::new();
    for pair in cuts.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        let chunk = &text[start..end];
        if chunk.trim().is_empty() {
            continue;
        }
        if let Some(previous) = out.last_mut() {
            if previous.text.chars().count() < MIN_SEGMENT_CHARS {
                let merged_end = end;
                let merged = &text[previous.start..merged_end];
                *previous = Segment {
                    start: previous.start,
                    end: merged_end,
                    text: merged.to_string(),
                };
                continue;
            }
        }
        out.push(Segment {
            start,
            end,
            text: chunk.to_string(),
        });
    }
    out
}

/// The sentence a span was read from, trimmed around the span so the
/// answer shows the claim without a wall of text (the jobhound quote).
fn quote_at(state: &str, start: usize, end: usize) -> String {
    let text = state;
    if text.chars().count() <= 200 {
        return text.trim().to_string();
    }
    let lo = start.saturating_sub(90);
    let hi = (end + 90).min(text.len());
    let (mut lo, mut hi) = (lo, hi);
    while lo > 0 && !text.is_char_boundary(lo) {
        lo -= 1;
    }
    while hi < text.len() && !text.is_char_boundary(hi) {
        hi += 1;
    }
    let mut quote = String::new();
    if lo > 0 {
        quote.push('…');
    }
    quote.push_str(text[lo..hi].trim());
    if hi < text.len() {
        quote.push('…');
    }
    quote
}

/// Take at most `cap` characters without splitting one.
fn take_chars(text: &str, cap: usize) -> &str {
    if text.chars().count() <= cap {
        return text;
    }
    let mut end = cap;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    use std::sync::{Arc, Mutex};

    /// One stub endpoint: serves `body` for every POST, recording the
    /// request bodies it received. The first `refuse_first` connections
    /// are dropped — the stale pooled connection the one-retry absorbs.
    /// The request is read head-then-declared-length before replying:
    /// replying with unread request bytes sends RST, and an RST
    /// discards the reply from the client's receive queue (KI-149's
    /// class, met from the serving side).
    fn stub_endpoint(body: &'static str, refuse_first: usize) -> (u16, Arc<Mutex<Vec<String>>>) {
        stub_endpoint_with_status(body, refuse_first, 200)
    }

    fn stub_endpoint_with_status(
        body: &'static str,
        refuse_first: usize,
        status: u16,
    ) -> (u16, Arc<Mutex<Vec<String>>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
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
                let head_end = loop {
                    let read = stream.read(&mut chunk).unwrap_or(0);
                    if read == 0 {
                        break request.len();
                    }
                    request.extend_from_slice(&chunk[..read]);
                    if let Some(at) = request.windows(4).position(|w| w == b"\r\n\r\n") {
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

    const STATE: &str = "Invoice INV-2026-0042 is due on 2026-11-30. The total due is $4,180.00 payable within 30 days. Late fees apply after the due date at 1.5 percent monthly.";

    /// The entity reply's content string: candidates in three fields.
    fn entity_body() -> String {
        let content = serde_json::json!({
            "entities": {
                "invoice_id": [
                    {"text": "INV-2026-0042", "confidence": 0.93, "start": 8, "end": 22}
                ],
                "total_due": [
                    {"text": "$4,180.00", "confidence": 0.88, "start": 57, "end": 66}
                ],
                "due_date": [
                    {"text": "2026-11-30", "confidence": 0.57, "start": 31, "end": 41}
                ]
            }
        })
        .to_string();
        serde_json::json!({
            "choices": [{"message": {"role": "assistant", "content": content}}]
        })
        .to_string()
    }

    /// A verify reply in the glide shape: noul carries the probability.
    fn verify_body(p_invoice: f64, p_total: f64, p_date: f64) -> String {
        serde_json::json!({
            "answers": {
                "invoice_id": {"type": "noul", "noul": p_invoice},
                "total_due": {"type": "noul", "noul": p_total},
                "due_date": {"type": "noul", "noul": p_date}
            }
        })
        .to_string()
    }

    fn leak(text: String) -> &'static str {
        Box::leak(text.into_boxed_str())
    }

    fn config_at(entity_port: u16, verify_port: u16) -> ExtractConfig {
        ExtractConfig {
            base_url: format!("http://127.0.0.1:{entity_port}"),
            entity_model: ENTITY_MODEL_DEFAULT.to_string(),
            token: "test-token".to_string(),
            verifiers: vec![Verifier {
                name: "glide".to_string(),
                url: format!("http://127.0.0.1:{verify_port}/v1/systemone"),
                model: "fastino/glide".to_string(),
                key: "test-token".to_string(),
                state_object: false,
            }],
        }
    }

    fn sample_fields() -> Vec<(String, FieldSpec)> {
        vec![
            ("invoice_id".to_string(), FieldSpec::of(FieldKind::Text)),
            ("total_due".to_string(), FieldSpec::of(FieldKind::Number)),
            ("due_date".to_string(), FieldSpec::of(FieldKind::Date)),
            ("net_terms".to_string(), FieldSpec::of(FieldKind::Duration)),
        ]
    }

    #[test]
    fn the_pipeline_carries_span_confidence_quote_and_absence() {
        // The locate pass declares the fields as entities; the verify
        // pass asks one noul per field over the carrying segment; the
        // answers carry span text (never normalized), the quote, the
        // confidence — and the named absence for the field with no
        // candidate at all.
        let (entity_port, entity_seen) = stub_endpoint(leak(entity_body()), 0);
        let (verify_port, verify_seen) = stub_endpoint(leak(verify_body(0.93, 0.90, 0.90)), 0);
        let client = ExtractClient::new(config_at(entity_port, verify_port));
        let report = client
            .run(STATE, &sample_fields())
            .expect("the run answers");
        assert_eq!(report.answers.len(), 4, "{report:?}");
        let invoice = report
            .answers
            .iter()
            .find(|a| a.name == "invoice_id")
            .expect("the invoice answer");
        assert_eq!(invoice.value, serde_json::json!("INV-2026-0042"));
        assert_eq!(invoice.spans, vec![(8, 22)]);
        assert_eq!(invoice.confidence, Some(0.93));
        assert!(!invoice.contested);
        assert!(
            invoice.quote.contains("Invoice INV-2026-0042"),
            "{:?}",
            invoice.quote
        );
        let net_terms = report
            .answers
            .iter()
            .find(|a| a.name == "net_terms")
            .expect("the net terms answer");
        assert_eq!(net_terms.value, serde_json::Value::Null);
        assert_eq!(
            net_terms.absent_reason.as_deref(),
            Some("no candidate span above the entity threshold")
        );
        // The locate body declares the fields as entities with their
        // descriptions; the store-off and confidence flags ride it.
        let sent: serde_json::Value =
            serde_json::from_str(&entity_seen.lock().unwrap()[0].clone()).unwrap();
        assert_eq!(sent["model"], ENTITY_MODEL_DEFAULT);
        assert_eq!(sent["schema"]["entities"][0]["name"], "invoice_id");
        assert!(sent["schema"]["entities"][0]["description"]
            .as_str()
            .is_some());
        assert_eq!(sent["store"], false);
        assert_eq!(sent["include_spans"], true);
        // The verify body is the systemone shape: model, state, noul
        // questions with true/false criteria — one call per carrying
        // segment (the fixture state carries candidates in two).
        assert_eq!(verify_seen.lock().unwrap().len(), 2, "one per segment");
        let sent: serde_json::Value =
            serde_json::from_str(&verify_seen.lock().unwrap()[0].clone()).unwrap();
        assert_eq!(sent["model"], "fastino/glide");
        assert!(sent["state"].is_string(), "{sent}");
        assert_eq!(sent["questions"]["invoice_id"]["type"], "noul");
        assert!(
            sent["questions"]["invoice_id"]["criteria"]["true"]
                .as_str()
                .is_some(),
            "{sent}"
        );
    }

    #[test]
    fn the_floors_reconcile_asymmetrically_and_adjudicate_once() {
        // accept at 0.85; veto at 0.35; between the floors the span is
        // contested and adjudicated once against the chain's next
        // verifier — the second opinion asserts, the span survives
        // marked by the deciding evidence.
        let entity_port = stub_endpoint(leak(entity_body()), 0).0;
        let (primary_port, primary_seen) = stub_endpoint(leak(verify_body(0.93, 0.10, 0.60)), 0);
        let (second_port, second_seen) = stub_endpoint(leak(verify_body(0.93, 0.10, 0.90)), 0);
        let mut config = config_at(entity_port, primary_port);
        config.verifiers.push(Verifier {
            name: "drex".to_string(),
            url: format!("http://127.0.0.1:{second_port}/v1/systemone"),
            model: "drex-v1.5".to_string(),
            key: "test-token".to_string(),
            state_object: false,
        });
        let client = ExtractClient::new(config);
        let report = client
            .run(STATE, &sample_fields())
            .expect("the run answers");
        let invoice = report
            .answers
            .iter()
            .find(|a| a.name == "invoice_id")
            .expect("invoice asserted");
        assert_eq!(invoice.confidence, Some(0.93));
        // Vetoed: the total's noul was 0.10 — the span is gone and the
        // field is absent BY NAME, with the reason that says the verify
        // floors did it.
        let total = report
            .answers
            .iter()
            .find(|a| a.name == "total_due")
            .expect("total answered");
        assert_eq!(total.value, serde_json::Value::Null);
        assert_eq!(
            total.absent_reason.as_deref(),
            Some("no candidate cleared the verify floors")
        );
        // Contested at 0.60, adjudicated, the second opinion's 0.90
        // asserts it.
        let date = report
            .answers
            .iter()
            .find(|a| a.name == "due_date")
            .expect("date answered");
        assert!(!date.contested);
        assert_eq!(date.confidence, Some(0.90));
        assert_eq!(report.stats.adjudications, 1, "{:?}", report.stats);
        assert!(report.stats.vetoed >= 1);
        // Two carrying segments: sentence one (invoice + date), sentence
        // two (the total). The adjudication rode the chain's second
        // verifier exactly once.
        assert_eq!(primary_seen.lock().unwrap().len(), 2, "one per segment");
        assert_eq!(second_seen.lock().unwrap().len(), 1, "one adjudication");
    }

    #[test]
    fn list_kind_returns_the_survivors_in_reading_order() {
        let content = serde_json::json!({
            "entities": {
                "milestone": [
                    {"text": "kickoff", "confidence": 0.9, "start": 0, "end": 7},
                    {"text": "handover", "confidence": 0.9, "start": 22, "end": 30}
                ]
            }
        })
        .to_string();
        let entity_body =
            leak(serde_json::json!({"choices": [{"message": {"content": content}}]}).to_string());
        let verify_body = leak(
            serde_json::json!({
                "answers": {"milestone": {"type": "noul", "noul": 0.95}}
            })
            .to_string(),
        );
        let entity_port = stub_endpoint(entity_body, 0).0;
        let verify_port = stub_endpoint(verify_body, 0).0;
        let client = ExtractClient::new(config_at(entity_port, verify_port));
        let fields = vec![("milestone".to_string(), FieldSpec::of(FieldKind::List))];
        let report = client
            .run("kickoff on Monday, handover on Friday", &fields)
            .expect("the run answers");
        let answer = &report.answers[0];
        assert_eq!(
            answer.value,
            serde_json::json!(["kickoff", "handover"]),
            "reading order, both survivors"
        );
        assert_eq!(answer.spans, vec![(0, 7), (22, 30)]);
    }

    #[test]
    fn one_retry_absorbs_a_dead_first_connection() {
        let entity_port = stub_endpoint(leak(entity_body()), 1).0;
        let verify_port = stub_endpoint(leak(verify_body(0.93, 0.93, 0.93)), 0).0;
        let client = ExtractClient::new(config_at(entity_port, verify_port));
        let report = client
            .run(STATE, &sample_fields())
            .expect("the retry lands");
        assert_eq!(report.answers.len(), 4);
    }

    #[test]
    fn errors_name_the_host_and_pass_never_the_token() {
        let (port, _seen) = stub_endpoint_with_status(r#"{"error":{"message":"bad key"}}"#, 0, 401);
        let mut config = config_at(port, port);
        config.token = "sk-secret-token-value".to_string();
        config.verifiers[0].key = "sk-secret-token-value".to_string();
        let client = ExtractClient::new(config);
        let error = client
            .run(STATE, &sample_fields())
            .expect_err("a 401 is an error");
        let rendered = format!("{error}");
        assert!(rendered.contains("HTTP 401"), "{rendered}");
        assert!(rendered.contains("127.0.0.1"), "{rendered}");
        assert!(rendered.contains("locate"), "{rendered}");
        assert!(
            !rendered.contains("sk-secret-token-value"),
            "the token never renders: {rendered}"
        );
    }

    #[test]
    fn the_named_absence_names_the_key() {
        let resolved = ExtractConfig::resolve(
            Some("/tmp/fleet-extract-absent-home"),
            Some("/tmp/fleet-extract-absent-user"),
            None,
            None,
        );
        let (config, missing) = resolved.expect("the resolve names what it found");
        assert_eq!(missing, vec!["FASTINO_API_KEY".to_string()], "{missing:?}");
        assert!(config.token.is_empty());
    }

    #[test]
    fn p_true_reads_every_family_shape() {
        // glide/drex: the probability itself. jev: a boolean with a
        // confidence. The string tolerance sys1's parser keeps.
        assert_eq!(
            p_true(&serde_json::json!({"type": "noul", "noul": 0.93})),
            Some(0.93)
        );
        assert_eq!(
            p_true(&serde_json::json!({"type": "noul", "noul": true, "confidence": 0.8})),
            Some(0.8)
        );
        assert_eq!(
            p_true(&serde_json::json!({"type": "noul", "noul": false, "confidence": 0.8})),
            Some(1.0 - 0.8)
        );
        assert_eq!(
            p_true(&serde_json::json!({"type": "noul", "noul": true})),
            Some(1.0)
        );
        assert_eq!(
            p_true(&serde_json::json!({"type": "noul", "noul": "yes", "confidence": 0.7})),
            Some(0.7)
        );
        assert_eq!(p_true(&serde_json::json!({"type": "noul"})), None);
    }

    #[test]
    fn segments_cut_on_sentence_ends_and_merge_fragments() {
        let text = "The first sentence is long enough to stand alone. B. Then a third sentence follows it.";
        let segs = segments(text);
        assert_eq!(segs.len(), 2, "{segs:?}");
        // The stub fragment merges into its neighbour rather than
        // standing alone.
        assert!(
            !segs.iter().any(|s| s.text.trim() == "B."),
            "fragments merge: {segs:?}"
        );
        assert!(
            segs[1].text.starts_with("B. Then"),
            "the merge extends to the neighbour's end: {segs:?}"
        );
        // Offsets are true: every segment's text is its slice.
        for segment in &segs {
            assert_eq!(&text[segment.start..segment.end], segment.text);
        }
    }

    #[test]
    fn the_entity_reply_tolerates_missing_labels_and_bad_content() {
        assert!(parse_entity_content("not json at all").is_empty());
        let spans = parse_entity_content(r#"{"entities": {"due_date": []}}"#);
        assert!(spans.is_empty());
        let spans = parse_entity_content(
            r#"{"entities": {"x": [{"text": "v", "confidence": 0.9, "start": 1, "end": 2}]}}"#,
        );
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].field, "x");
    }
}
