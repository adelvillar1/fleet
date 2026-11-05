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

/// The lane's entity verb: the extraction-class GLiNER, called by sys1
/// on whichever vendor sys1 has configured. The leg names no vendor.
pub const EXTRACT_TOOL: &str = "sys1_extract";

/// The lane's decision verb: the verify battery rides it with an inline
/// task spec, so the leg's per-field heads get sys1's routing and
/// cascade as any registered task does.
pub const CLASSIFY_TOOL: &str = "sys1_classify";

/// The verify task's inline id: the name the lane's answer carries, so
/// a recorded judgment says what was judged.
pub const VERIFY_TASK: &str = "extract-verify";

/// The entity pass's input cap (the vendor's own documented ceiling —
/// longer inputs are the async endpoint's job, which the leg does not
/// grow until a caller needs it). The lane enforces the same bound; the
/// leg bounds what it sends.
pub const EXTRACT_INPUT_CAP: usize = 50_000;

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

/// One head the leg asks the lane: the id the answer keys by, and the
/// question in plain language.
#[derive(Debug, Clone, PartialEq)]
pub struct HeadQuestion {
    pub id: String,
    pub question: String,
}

/// The leg's lane session: one sys1 lane, opened once per extraction and
/// closed at the end — the judge beat's own shape. Every vendor fact
/// (the base, the key, the chain) lives inside the lane; the leg names
/// none of it.
pub struct ExtractSession {
    server: crate::mcp::McpServer,
}

impl ExtractSession {
    /// Open the lane and confirm it serves the entity verb. A lane that
    /// cannot start, refuses the handshake, or lacks the tool is a named
    /// error — never a quiet fallback to a vendor.
    pub async fn open(source: &crate::lanes::LaneSource) -> Result<ExtractSession, String> {
        let spec = crate::judge::lane_spec(source, "sys1")?;
        let server = crate::mcp::McpServer::spawn(&spec)
            .map_err(|error| format!("the extraction lane could not start: {error}"))?;
        if let Err(error) = server.initialize().await {
            server.close().await.ok();
            return Err(format!("the extraction lane's handshake failed: {error}"));
        }
        let tools = match server.list_tools().await {
            Ok(tools) => tools,
            Err(error) => {
                server.close().await.ok();
                return Err(format!("the extraction lane's tool list failed: {error}"));
            }
        };
        if !tools.iter().any(|tool| tool.name == EXTRACT_TOOL) {
            server.close().await.ok();
            return Err(format!(
                "lane `sys1` declares no `{EXTRACT_TOOL}` tool — it cannot serve the extraction leg"
            ));
        }
        Ok(ExtractSession { server })
    }

    /// Pass 1 — the entity read. Answers the spans (offsets, confidence)
    /// and the model that served them; a refused or failed read is the
    /// lane's own named error.
    pub async fn locate(
        &self,
        text: &str,
        fields: &[(String, FieldSpec)],
    ) -> Result<(Vec<Located>, String), ExtractError> {
        let list: Vec<serde_json::Value> = fields
            .iter()
            .map(|(name, spec)| {
                serde_json::json!({
                    "name": name,
                    "description": spec.description(name),
                })
            })
            .collect();
        let arguments = serde_json::json!({
            "text": take_chars(text, EXTRACT_INPUT_CAP),
            "fields": list,
        });
        let call = self
            .server
            .call_tool(EXTRACT_TOOL, arguments)
            .await
            .map_err(|error| ExtractError {
                host: "sys1".to_string(),
                pass: "locate".to_string(),
                reason: error.to_string(),
            })?;
        if call.is_error {
            return Err(ExtractError {
                host: "sys1".to_string(),
                pass: "locate".to_string(),
                reason: call.text(),
            });
        }
        let reply: serde_json::Value =
            serde_json::from_str(call.text().as_str()).map_err(|error| ExtractError {
                host: "sys1".to_string(),
                pass: "locate".to_string(),
                reason: format!("the lane's answer was not the entity JSON it promises: {error}"),
            })?;
        if reply.get("ok").and_then(|ok| ok.as_bool()) != Some(true) {
            let code = reply
                .get("code")
                .and_then(|code| code.as_str())
                .unwrap_or("refused");
            let reason = reply
                .get("reason")
                .and_then(|reason| reason.as_str())
                .unwrap_or("the entity read was refused without a reason");
            return Err(ExtractError {
                host: "sys1".to_string(),
                pass: "locate".to_string(),
                reason: format!("{code}: {reason}"),
            });
        }
        let model = reply
            .get("model")
            .and_then(|model| model.as_str())
            .unwrap_or("gliner")
            .to_string();
        let mut located = Vec::new();
        if let Some(spans) = reply.get("spans").and_then(|spans| spans.as_array()) {
            for span in spans {
                let (Some(field), Some(value), Some(start), Some(end)) = (
                    span.get("field").and_then(|field| field.as_str()),
                    span.get("text").and_then(|text| text.as_str()),
                    span.get("start").and_then(|start| start.as_u64()),
                    span.get("end").and_then(|end| end.as_u64()),
                ) else {
                    continue;
                };
                located.push(Located {
                    field: field.to_string(),
                    text: value.to_string(),
                    start: start as usize,
                    end: end as usize,
                    confidence: span
                        .get("confidence")
                        .and_then(|value| value.as_f64())
                        .unwrap_or(0.0),
                });
            }
        }
        Ok((located, model))
    }

    /// One decision battery over one text, with the caller's own heads:
    /// the answering provider's head map, the provider's name, and the
    /// chain the lane walked. `provider` names a chain to walk (`"all"`
    /// for the adjudication's second opinion); `None` is the lane's
    /// routing. The desk's claim pass rides this for its choice heads;
    /// `verdicts` wraps it for the leg's nouls.
    pub async fn answers(
        &self,
        text: &str,
        heads: serde_json::Value,
        provider: Option<&str>,
        pass: &str,
    ) -> Result<(serde_json::Value, String, Vec<String>), ExtractError> {
        let mut arguments = serde_json::json!({
            "text": take_chars(text, MAX_SEGMENT_CHARS),
            "task_spec": {
                "id": VERIFY_TASK,
                // The leg applies its own floors; the lane's gate stays
                // neutral so every probability comes back.
                "confidence_floor": 0.0,
                "heads": heads,
            },
        });
        if let Some(provider) = provider {
            arguments["provider"] = serde_json::Value::String(provider.to_string());
        }
        let call = self
            .server
            .call_tool(CLASSIFY_TOOL, arguments)
            .await
            .map_err(|error| ExtractError {
                host: "sys1".to_string(),
                pass: pass.to_string(),
                reason: error.to_string(),
            })?;
        if call.is_error {
            return Err(ExtractError {
                host: "sys1".to_string(),
                pass: pass.to_string(),
                reason: call.text(),
            });
        }
        let reply: serde_json::Value =
            serde_json::from_str(call.text().as_str()).map_err(|error| ExtractError {
                host: "sys1".to_string(),
                pass: pass.to_string(),
                reason: format!("the lane's answer was not the JSON it promises: {error}"),
            })?;
        let provider_used = reply
            .get("provider")
            .and_then(|provider| provider.as_str())
            .unwrap_or("sys1")
            .to_string();
        let chain: Vec<String> = reply
            .get("providers_used")
            .and_then(|used| used.as_array())
            .map(|used| {
                used.iter()
                    .filter_map(|name| name.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        // The answers map is keyed by provider. The routing's answer rides
        // the provider that answered, and the adjudication's rides every
        // provider the lane walked — so the caller gets the whole map and
        // decides (the primary reads its own provider's entry; the
        // adjudication takes the strongest across them). serde_json's map
        // is sorted, so "the first entry" is never "the provider".
        let answers = reply
            .get("answers")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        Ok((answers, provider_used, chain))
    }

    /// Pass 3 — one decision battery over one segment: one `noul` head per
    /// field, asked through an inline task spec. Answers the p(true) per
    /// head, the provider that answered, and the chain the lane walked.
    pub async fn verdicts(
        &self,
        segment_text: &str,
        heads: &[HeadQuestion],
        provider: Option<&str>,
        pass: &str,
    ) -> Result<(Vec<(String, f64)>, String, Vec<String>), ExtractError> {
        let head_list: Vec<serde_json::Value> = heads
            .iter()
            .map(|head| {
                serde_json::json!({
                    "id": head.id,
                    "kind": "noul",
                    "task": head.question,
                    "descriptions": {
                        "true": "Yes, it is stated here.",
                        "false": "No.",
                    },
                })
            })
            .collect();
        let (answers, provider_used, chain) = self
            .answers(
                segment_text,
                serde_json::Value::Array(head_list),
                provider,
                pass,
            )
            .await?;
        let heads_map = provider_map(&answers, &provider_used);
        let verdicts = heads
            .iter()
            .map(|head| {
                let p = heads_map.get(&head.id).and_then(p_true).unwrap_or(0.0);
                (head.id.clone(), p)
            })
            .collect();
        Ok((verdicts, provider_used, chain))
    }

    /// The adjudication's reading: per head, the strongest probability any
    /// provider the lane walked returned. One call across every provider
    /// (`provider: "all"`); the leg's own floors decide what the number
    /// means.
    pub async fn strongest(
        &self,
        segment_text: &str,
        heads: &[HeadQuestion],
    ) -> Result<(Vec<(String, f64)>, String, Vec<String>), ExtractError> {
        let head_list: Vec<serde_json::Value> = heads
            .iter()
            .map(|head| {
                serde_json::json!({
                    "id": head.id,
                    "kind": "noul",
                    "task": head.question,
                    "descriptions": { "true": "Yes, it is stated here.", "false": "No." },
                })
            })
            .collect();
        let (answers, provider_used, chain) = self
            .answers(
                segment_text,
                serde_json::Value::Array(head_list),
                Some("all"),
                "adjudicate",
            )
            .await?;
        let verdicts = heads
            .iter()
            .map(|head| {
                let p = answers
                    .as_object()
                    .map(|providers| {
                        providers
                            .values()
                            .filter_map(|map| map.get(&head.id).and_then(p_true))
                            .fold(0.0_f64, f64::max)
                    })
                    .unwrap_or(0.0);
                (head.id.clone(), p)
            })
            .collect();
        Ok((verdicts, provider_used, chain))
    }

    /// Torn down after the leg's last call — the judge beat's own order.
    pub async fn close(self) {
        self.server.close().await.ok();
    }
}

/// The pipeline: locate, segment, verify, reconcile. Every requested
/// field is answered — with its value and quote, or by name with the
/// reason it is absent. The vendor is sys1's; the judgment here is the
/// leg's own.
pub async fn run(
    source: &crate::lanes::LaneSource,
    state: &str,
    fields: &[(String, FieldSpec)],
) -> Result<ExtractReport, String> {
    let session = ExtractSession::open(source).await?;
    let report = run_in(&session, state, fields).await;
    session.close().await;
    report.map_err(|error| error.to_string())
}

async fn run_in(
    session: &ExtractSession,
    state: &str,
    fields: &[(String, FieldSpec)],
) -> Result<ExtractReport, ExtractError> {
    let started = std::time::Instant::now();
    let (located, locate_model) = session.locate(state, fields).await?;
    let asked: std::collections::HashSet<&str> =
        fields.iter().map(|(name, _)| name.as_str()).collect();
    let candidates: Vec<&Located> = located
        .iter()
        .filter(|span| asked.contains(span.field.as_str()))
        .collect();
    let mut stats = ExtractStats::default();
    let mut used: Vec<String> = Vec::new();
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
            models: vec![locate_model],
            stats,
        });
    }

    // Pass 2 — segments; only the ones carrying a candidate cost a call,
    // bounded by the budget.
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
        let mut head_list: Vec<HeadQuestion> = Vec::new();
        for span in spans {
            if !head_list.iter().any(|head| head.id == span.field) {
                let spec = fields
                    .iter()
                    .find(|(name, _)| name == &span.field)
                    .map(|(_, spec)| spec)
                    .expect("candidates are filtered to asked fields");
                head_list.push(HeadQuestion {
                    id: span.field.clone(),
                    question: format!("Does this text state {}?", spec.description(&span.field)),
                });
            }
        }
        let read = take_chars(&segment.text, MAX_SEGMENT_CHARS);
        let (verdicts, provider, chain) =
            session.verdicts(read, &head_list, None, "verify").await?;
        stats.verify_calls += 1;
        record_model(&mut used, &provider);
        for name in chain {
            record_model(&mut used, &name);
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
                // A veto only removes a claim; the field's absence is
                // named downstream, never filled.
                stats.vetoed += 1;
                continue;
            }
            // Contested: one adjudication across every provider the lane
            // has (bounded), and the stronger evidence wins; still
            // unsettled, the span ships marked.
            let (p_final, contested) = if adjudications < MAX_ADJUDICATIONS {
                adjudications += 1;
                let (second, second_provider, second_chain) =
                    session.strongest(read, &head_list).await?;
                for name in std::iter::once(second_provider).chain(second_chain) {
                    record_model(&mut used, &name);
                }
                let second_p = second
                    .iter()
                    .find(|(name, _)| name == &span.field)
                    .map(|(_, p)| *p)
                    .unwrap_or(0.0);
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
    let mut models = vec![locate_model];
    models.extend(used);

    // Pass 4's other half — assemble the answers: every requested field,
    // best survivor first; the absence named when nothing survived.
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

/// The answering provider's heads map out of an [`ExtractSession`]
/// `answers()` reply. The reply is provider-keyed (the adjudication
/// needs every provider's read), so a caller asking ONE question reads
/// the entry of the provider that answered — falling back to the first
/// map only when the reply names a provider it did not answer with.
/// serde_json's maps are sorted, so "the first entry" is never
/// "the provider".
pub fn provider_map(answers: &serde_json::Value, provider: &str) -> serde_json::Value {
    answers
        .get(provider)
        .or_else(|| answers.as_object().and_then(|map| map.values().next()))
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

/// Record a provider or model name the lane walked — once, and never
/// an empty one. An escalated answer can come back with no single
/// provider named; the empty string is not a model the report should
/// list as if it were.
fn record_model(used: &mut Vec<String>, name: &str) {
    if name.is_empty() || used.iter().any(|seen| seen == name) {
        return;
    }
    used.push(name.to_string());
}

/// p(true) from a noul answer entry, across the family's shapes:
/// glide/drex carry the probability itself (`noul: p`, 1.0 = certainly
/// true, 0.0 = certainly false); jev may carry a boolean with a
/// confidence; a yes/no string is tolerated the way sys1's parser
/// tolerates one. A missing confidence on a bare boolean is certainty —
/// the model said so without hedging.
pub fn p_true(entry: &serde_json::Value) -> Option<f64> {
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
pub fn take_chars(text: &str, cap: usize) -> &str {
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

    // The transport pins (the locate/verify wire, the floors, the
    // absence) live in `tests/integration.rs`: they drive the mock-mcp
    // lane, and the mock binary is built for the integration tests, not
    // the lib. These are the pure halves.

    #[test]
    fn provider_map_reads_the_answering_provider_not_the_sorted_first() {
        // The reply is provider-keyed and serde_json sorts the keys, so
        // "drex" lands before "glide" whatever the routing chose. A
        // caller asking one question must read the provider that
        // answered — reading the first map is how a claim pass read 0.0
        // for every head while the lane answered perfectly.
        let nested = serde_json::json!({
            "drex": {"alpha": {"noul": 0.2}},
            "glide": {"alpha": {"noul": 0.93}},
        });
        assert_eq!(provider_map(&nested, "glide")["alpha"]["noul"], 0.93);
        assert_eq!(provider_map(&nested, "drex")["alpha"]["noul"], 0.2);
        // A reply naming a provider it did not answer with falls back to
        // the only map there is.
        assert_eq!(provider_map(&nested, "liquid")["alpha"]["noul"], 0.2);
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
}
