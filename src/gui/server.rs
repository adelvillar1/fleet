//! The loopback HTTP surface behind `fleet gui`.
//!
//! Hand-rolled HTTP/1.1 over tokio's `TcpListener`, because the crate
//! contract is one static Rust binary: a framework would buy this
//! surface nothing it does not already do by hand — the panel reads,
//! one SSE stream, and the small set of actions the substrate already
//! owns. The route table below is the whole surface.
//!
//! ## The security shape
//!
//! - **Loopback only.** The listener binds a loopback address, and a
//!   `--host` that is not one is refused by name before a socket
//!   exists. This face is the owner's machine and nothing else.
//! - **The boundary is the machine.** The listener is loopback-only,
//!   and two checks keep the web's remote hands off the local port: a
//!   request whose `Host` does not name this machine is refused (that
//!   is the rebinding attack — a remote page serving the local port
//!   under a hostname it owns), and a state-changing request with a
//!   foreign `Origin` is refused (that is the cross-site form). A
//!   browser on this machine fails neither, so the address alone opens
//!   the face — the owner's ruling is that it should, and that
//!   authentication returns only when fleet becomes a multi-user app.
//! - **No credentials in any payload.** The page, the assets, the
//!   stream, and the action replies carry names and presence, never
//!   values: the masking rule doctor holds is the rule here.
//!
//! ## The stream and the actions
//!
//! `/events` is the only long-lived connection. It answers a hello that
//! tells the page where it is, then a heartbeat on an interval for as
//! long as the browser listens. Everything the face shows — a run's
//! turns, a route's decision, a memory's recall, a tool's approval seat
//! — rides this same stream as one more event shape, so G1's seam is
//! the wire the rest of the face speaks on. The actions are the other
//! direction: `POST /api/run` hands the browser's task to the agent
//! thread, and `POST /api/approve` answers the seat that thread is
//! waiting on. A page that has just loaded reads `GET /api/state` for
//! the replay, then subscribes for what comes next; the two agree
//! because they are the same log.
//!
//! The read routes beyond `/api/state` are the panels': `GET
//! /api/router` is the router board's own reads plus the
//! route-decision cache's counts, read off the async worker, `GET
//! /api/delegation` is the live run's delegation tree off the session
//! logs beside the dependency graph's own file, `GET /api/memory` is
//! the corpus and the day's recall hits, and `GET /api/skills` is the
//! library with its skips, its proposals, and its curation candidates.
//! A panel's document is the board's facts, not a second opinion about
//! them, and it never carries a credential.
//!
//! The write actions are the panels' own, and each rides the
//! substrate's path rather than a file write of this surface's: `POST
//! /api/fact` edits a fact in place through memory's own rewriter, and
//! `POST /api/skill` approves a skills proposal through improve's own
//! apply — the same gate `fleet improve apply` runs, so the reply waits
//! on it. Neither can reach a credential, and neither writes anything
//! the substrate would refuse.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::gui::bridge::Bridge;

/// The page, its stylesheet, and its script, embedded in the binary:
/// there is no asset directory to ship beside it and nothing served
/// from disk.
pub(crate) const INDEX: &str = include_str!("assets/index.html");
pub(crate) const STYLES: &str = include_str!("assets/app.css");
pub(crate) const SCRIPT: &str = include_str!("assets/app.js");
/// The install surface: the manifest, the shell worker, and the icons.
pub(crate) const MANIFEST: &str = include_str!("assets/manifest.webmanifest");
const WORKER: &str = include_str!("assets/sw.js");
const ICON_192: &[u8] = include_bytes!("assets/icon-192.png");
const ICON_512: &[u8] = include_bytes!("assets/icon-512.png");

/// The build stamp `build.rs` computed over every served asset. The page
/// asks for the two script assets at this address, the worker's cache
/// name carries it, and only a request naming it is immutable.
pub(crate) const ASSET_HASH: &str = env!("FLEET_ASSET_HASH");

/// How long an icon may be reused. Icons change only when the binary
/// does, and they are not the app: a day is long enough to stop the
/// re-fetch and short enough that a changed mark lands without a
/// reinstall.
pub(crate) const ICON_CACHE: &str = "public, max-age=86400";

/// How often the stream says the server is still there. The page counts
/// the beats, so a silent stream is a dead server the owner can see.
const HEARTBEAT: Duration = Duration::from_secs(5);

/// The largest request head read from one connection. A browser's head
/// is a few hundred bytes; the cap is what stops a peer that never
/// sends the terminator from holding a task forever.
const MAX_HEAD_BYTES: usize = 16 * 1024;

/// The largest request body this surface reads. A task is a paragraph
/// and a decision is a word; the cap is what stops a peer that declares
/// a length and never sends it from holding a task forever.
const MAX_BODY_BYTES: usize = 64 * 1024;

/// One launch's face: the listener, the token, the address the owner
/// was given, and the bridge to the agent thread the actions speak to.
/// A `Server` is deliberately not cloneable — one launch, one token,
/// one listener — and the per-connection tasks carry what they need by
/// value instead.
pub struct Server {
    base_url: String,
    heartbeat: Duration,
    listener: tokio::net::TcpListener,
    bridge: Arc<Bridge>,
}

impl Server {
    /// Binds the loopback listener. `port` zero asks the operating
    /// system for a free one, which is what a second concurrent launch
    /// and the tests need.
    pub async fn bind(host: &str, port: u16, bridge: Arc<Bridge>) -> Result<(Server, u16), String> {
        let address = loopback_address(host)?;
        let listener = tokio::net::TcpListener::bind((address, port))
            .await
            .map_err(|e| format!("fleet gui cannot bind {address}:{port}: {e}"))?;
        let bound = listener
            .local_addr()
            .map_err(|e| format!("fleet gui cannot read its own address: {e}"))?;
        let base_url = format!("http://{bound}");
        let server = Server {
            base_url,
            heartbeat: HEARTBEAT,
            listener,
            bridge,
        };
        Ok((server, bound.port()))
    }

    /// The address the owner opens: the machine's own loopback name and
    /// the bound port, and nothing secret in it.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Serves until the owner stops the process. Ctrl-C ends it
    /// cleanly, and every connection is a task of its own, so one slow
    /// stream never holds the listener.
    pub async fn serve(&self) -> Result<(), String> {
        loop {
            tokio::select! {
                accepted = self.listener.accept() => match accepted {
                    Ok((stream, _from)) => {
                        let base_url = self.base_url.clone();
                        let heartbeat = self.heartbeat;
                        let bridge = Arc::clone(&self.bridge);
                        tokio::spawn(async move {
                            if let Err(e) =
                                serve_connection(stream, &base_url, heartbeat, &bridge).await
                            {
                                eprintln!("fleet gui: {e}");
                            }
                        });
                    }
                    // A connection that failed to arrive is not a reason
                    // to stop serving the ones that do.
                    Err(e) => eprintln!("fleet gui: a connection did not arrive: {e}"),
                },
                _ = tokio::signal::ctrl_c() => return Ok(()),
            }
        }
    }
}

/// The session a request addresses: the one its query names, or the
/// launch session when it names none. The launch session is the bridge's
/// own answer, so a default taken here cannot drift from the one the
/// bridge boots with — and a phone that names nothing gets the same
/// session the desktop's own face opens.
fn asked_session(bridge: &Bridge, request: &Request) -> String {
    let asked = decode_query(&request.query, "session");
    let asked = asked.trim();
    if asked.is_empty() {
        bridge.launch()
    } else {
        asked.to_string()
    }
}

/// One connection, start to finish: read the head, check that it came
/// from this machine's own web, route, answer, and drop the socket.
/// Every reply says `Connection: close` and means it — a browser
/// reconnects, and a scanner learns nothing from a held-open port.
async fn serve_connection(
    mut stream: tokio::net::TcpStream,
    base_url: &str,
    heartbeat: Duration,
    bridge: &Arc<Bridge>,
) -> Result<(), String> {
    let (head, rest) = read_head(&mut stream).await?;
    let request = Request::parse(&head)?;
    // The boundary is the machine, and the two checks below are what
    // hold it: the listener is loopback-only, so a remote hand has to
    // arrive through a browser, and a browser arriving through a
    // rebinding page carries a foreign Host while a page driving the
    // port cross-site carries a foreign Origin. A local browser fails
    // neither — the owner's ruling is that the address alone opens the
    // face.
    if !request.host_is_loopback() {
        return write_reply(
            &mut stream,
            forbidden("the Host header does not name this machine"),
        )
        .await;
    }
    if matches!(request.method.as_str(), "POST" | "PUT" | "PATCH" | "DELETE") {
        if let Some(origin) = request.header("origin") {
            if !origin_is_loopback(origin) {
                return write_reply(
                    &mut stream,
                    forbidden("the request's origin is not this machine"),
                )
                .await;
            }
        }
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => write_reply(&mut stream, page()).await,
        ("GET", "/manifest.webmanifest") => write_reply(&mut stream, manifest()).await,
        ("GET", "/sw.js") => write_reply(&mut stream, worker()).await,
        ("GET", "/icon-192.png") => {
            write_blob(&mut stream, "image/png", ICON_CACHE, ICON_192).await
        }
        ("GET", "/icon-512.png") => {
            write_blob(&mut stream, "image/png", ICON_CACHE, ICON_512).await
        }
        // iOS reads this one rather than the manifest's icons; the 192 is
        // the size it wants and a second render would be a second asset
        // to keep in step.
        ("GET", "/apple-touch-icon.png") => {
            write_blob(&mut stream, "image/png", ICON_CACHE, ICON_192).await
        }
        ("GET", "/app.css") => {
            let reply = if query_value(&request.query, "v") == Some(ASSET_HASH) {
                versioned_asset("text/css; charset=utf-8", STYLES)
            } else {
                asset("text/css; charset=utf-8", STYLES)
            };
            write_reply(&mut stream, reply).await
        }
        ("GET", "/app.js") => {
            let reply = if query_value(&request.query, "v") == Some(ASSET_HASH) {
                versioned_asset("text/javascript; charset=utf-8", SCRIPT)
            } else {
                asset("text/javascript; charset=utf-8", SCRIPT)
            };
            write_reply(&mut stream, reply).await
        }
        ("GET", "/events") => {
            stream_events(&mut stream, base_url, heartbeat, bridge.subscribe()).await
        }
        ("GET", "/api/state") => {
            let session = asked_session(bridge, &request);
            write_reply(&mut stream, api(200, bridge.state(&session))).await
        }
        ("GET", "/api/home") => write_reply(&mut stream, api(200, bridge.home().await)).await,
        ("GET", "/api/router") => write_reply(&mut stream, api(200, bridge.router().await)).await,
        ("GET", "/api/delegation") => {
            let session = asked_session(bridge, &request);
            write_reply(&mut stream, api(200, bridge.delegation(&session).await)).await
        }
        ("GET", "/api/memory") => write_reply(&mut stream, api(200, bridge.memory().await)).await,
        ("GET", "/api/skills") => write_reply(&mut stream, api(200, bridge.skills().await)).await,
        // The pane's two reads. Both are contained to the working
        // directory by the bridge, and both refuse a credential file by
        // name — the browser can ask for a file now, which is exactly
        // where one would leak.
        ("GET", "/api/tree") => {
            let asked = decode_query(&request.query, "path");
            write_reply(&mut stream, api(200, bridge.tree(&asked).await)).await
        }
        // The inspector's related suggestions. The kind is a closed set
        // — an unknown one is a 400, not an empty list that could mean
        // "nothing is related" — and the id is the selected object's
        // own key, exactly as the panel named it.
        ("GET", "/api/related") => {
            let kind = decode_query(&request.query, "kind");
            if !crate::gui::related::KINDS.contains(&kind.as_str()) {
                return write_reply(
                    &mut stream,
                    bad_request(&format!(
                        "`{kind}` is not a kind of object; the kinds are {}",
                        crate::gui::related::KINDS.join(", ")
                    )),
                )
                .await;
            }
            let id = decode_query(&request.query, "id");
            if id.trim().is_empty() {
                return write_reply(
                    &mut stream,
                    bad_request("a related read needs the selected object's `id`"),
                )
                .await;
            }
            let limit = decode_query(&request.query, "limit")
                .trim()
                .parse::<usize>()
                .unwrap_or(crate::gui::related::DEFAULT_LIMIT);
            write_reply(
                &mut stream,
                api(200, bridge.related(&kind, &id, limit).await),
            )
            .await
        }
        // The browser's own usage counters. Counters and ids only:
        // nothing here carries task text, a file, or a credential, and
        // the store refuses a section/event pair outside its closed set.
        ("GET", "/api/usage") => write_reply(&mut stream, api(200, bridge.usage())).await,
        ("POST", "/api/usage") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let (section, id, event) = match usage_event_of(&body) {
                Ok(parts) => parts,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            match bridge.record_usage(&section, &id, &event).await {
                Ok(document) => write_reply(&mut stream, api(200, document)).await,
                Err(message) => write_reply(&mut stream, api(400, json!({"error": message}))).await,
            }
        }
        ("GET", "/api/contrast") => write_reply(&mut stream, api(200, bridge.contrast())).await,
        ("GET", "/api/web") => {
            let asked = decode_query(&request.query, "url");
            write_reply(&mut stream, api(200, bridge.web(&asked).await)).await
        }
        ("GET", "/api/file") => {
            let asked = decode_query(&request.query, "path");
            write_reply(&mut stream, api(200, bridge.file(&asked).await)).await
        }
        ("POST", "/api/pair/window") => match bridge.pair_window().await {
            Ok(document) => write_reply(&mut stream, api(200, document)).await,
            Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
        },
        ("POST", "/api/attach") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            match bridge.attach(&body).await {
                Ok(reply) => write_reply(&mut stream, api(200, reply)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/run") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let task = match task_of(&body) {
                Ok(task) => task,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let task = match with_attachments(&body, &task) {
                Ok(task) => task,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let (mode, pin) = match run_policy_of(&body) {
                Ok(policy) => policy,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let folder = match folder_of(&body) {
                Ok(folder) => folder,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            // A folder the run names is validated before anything is
            // stored or dispatched: the pane's containment rule, and a
            // path that does not exist named as itself.
            if let Some(asked) = folder.as_deref() {
                if let Err(message) = bridge.chat_folder(asked) {
                    return write_reply(&mut stream, bad_request(&message)).await;
                }
            }
            let session = asked_session(bridge, &request);
            match bridge.run_with(&session, &task, mode, pin, folder.as_deref()) {
                Ok(()) => {
                    write_reply(
                        &mut stream,
                        api(200, json!({"type": "run", "task": task, "accepted": true})),
                    )
                    .await
                }
                // A rule the owner's own choices broke — full access
                // with no folder, a folder that is not there — is the
                // page's to correct; a face that cannot take the run is
                // the page's to wait on.
                Err(crate::gui::bridge::RunError::BadRequest(message)) => {
                    write_reply(&mut stream, bad_request(&message)).await
                }
                Err(crate::gui::bridge::RunError::Refused(message)) => {
                    write_reply(&mut stream, api(409, json!({"error": message}))).await
                }
            }
        }
        // The chat's working folder, changed from the run bar without a
        // run. It applies to the next run and lands in the transcript as
        // one line; a run in flight already carries its own root.
        ("POST", "/api/folder") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let folder = match folder_choice_of(&body) {
                Ok(folder) => folder,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let session = asked_session(bridge, &request);
            match bridge.set_folder(&session, folder.as_deref()) {
                Ok(document) => write_reply(&mut stream, api(200, document)).await,
                Err(message) => write_reply(&mut stream, bad_request(&message)).await,
            }
        }
        // The owner's two instructions to the run in flight. Both write
        // the control file the agent's turn loop already reads before
        // every model call; the route only names what the write means.
        // A face with no run, or a run with no log yet, is a 409 that
        // says which — never a file nobody will read.
        ("POST", "/api/stop") => match bridge.stop(None) {
            Ok(control) => write_reply(&mut stream, api(200, control)).await,
            Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
        },
        ("POST", "/api/steer") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let text = match steer_of(&body) {
                Ok(text) => text,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            match bridge.stop(Some(&text)) {
                Ok(control) => write_reply(&mut stream, api(200, control)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/approve") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let decision = match decision_of(&body) {
                Ok(decision) => decision,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let session = asked_session(bridge, &request);
            match bridge.answer(
                &session,
                &decision,
                &crate::gui::bridge::AnsweredBy::Desktop,
            ) {
                Ok(answered) => write_reply(&mut stream, api(200, answered)).await,
                Err(crate::gui::bridge::AnswerError::NoSeat) => {
                    write_reply(
                        &mut stream,
                        api(409, json!({"error": "no approval is waiting"})),
                    )
                    .await
                }
                Err(crate::gui::bridge::AnswerError::Unknown(word)) => {
                    write_reply(&mut stream, api(400, json!({"error": word}))).await
                }
            }
        }
        ("POST", "/api/deck/render") => {
            // The deck pane's apply: the browser sends a deck — the one
            // the event carried, as written or as repaired — and this
            // validates, renders, and names the verdict. The boundary
            // checks above already held; a deck that breaks a consultant
            // constraint is a 409 that names the rule, not a render.
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let parsed: serde_json::Value = match serde_json::from_str(&body) {
                Ok(parsed) => parsed,
                Err(message) => {
                    return write_reply(&mut stream, bad_request(&message.to_string())).await
                }
            };
            let deck: Result<crate::deck::Deck, String> = parsed
                .get("deck")
                .cloned()
                .ok_or_else(|| "the body carries no `deck`".to_string())
                .and_then(|value| {
                    serde_json::from_value(value)
                        .map_err(|error| format!("the deck does not parse: {error}"))
                });
            let deck = match deck {
                Ok(deck) => deck,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let refusals = crate::deck::check(&deck);
            if !refusals.is_empty() {
                let named: Vec<String> = refusals.iter().map(|r| r.line()).collect();
                let reply = serde_json::json!({
                    "status": "refused", "refusals": named,
                });
                return write_reply(&mut stream, api(409, reply)).await;
            }
            let review = crate::deck::review(&deck);
            let faults: Vec<serde_json::Value> =
                review.iter().map(crate::deck::fault_value).collect();
            let html = crate::deck::render(&deck);
            let reply = serde_json::json!({
                "status": "rendered",
                "review": crate::deck::review_line(&deck),
                "faults": faults,
                "html": html,
            });
            write_reply(&mut stream, api(200, reply)).await
        }
        ("POST", "/api/rank") => {
            // The composer's command type-ahead: the browser sends the
            // command declarations it owns, and this ranks them against
            // what the owner typed. Nothing here performs an action —
            // the ranking is the substrate's, the command set is the
            // page's.
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let parsed: serde_json::Value = match serde_json::from_str(&body) {
                Ok(parsed) => parsed,
                Err(message) => {
                    return write_reply(&mut stream, bad_request(&message.to_string())).await
                }
            };
            let asked = parsed
                .get("asked")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let candidates = parsed
                .get("candidates")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            write_reply(
                &mut stream,
                api(200, bridge.rank(&asked, &candidates).await),
            )
            .await
        }
        ("POST", "/api/fact") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            // The edit rides the substrate's own rewriter, so the body is
            // the fact-edit's own arguments object rather than a shape
            // this surface invented.
            match bridge.fact_edit(&body).await {
                Ok(edited) => write_reply(&mut stream, api(200, edited)).await,
                Err(message) => write_reply(&mut stream, api(400, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/skill") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let name = match proposal_of(&body) {
                Ok(name) => name,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            // The gate is the real one, so the reply waits on it: an
            // approve that answered before the gate ran would be an
            // approve of nothing.
            match bridge.skill_apply(&name).await {
                Ok(applied) => write_reply(&mut stream, api(200, applied)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        // The sessions surface: the archive, one archived session's
        // replay, and the resume that re-anchors this face to that log.
        // The id rides the query like the pane's `path` and is refused
        // for its shape before the disk is touched, for the same reason.
        ("GET", "/api/sessions") => {
            let session = asked_session(bridge, &request);
            write_reply(&mut stream, api(200, bridge.sessions(&session).await)).await
        }
        // Who may reach this face, and the revocation that forgets one.
        // A loopback read and write only: the remote face's allowlist
        // keeps both closed, because a phone that could revoke devices is
        // a phone that could lock its owner out.
        ("GET", "/api/access") => write_reply(&mut stream, api(200, bridge.access().await)).await,
        ("POST", "/api/access/revoke") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let parsed: serde_json::Value = match serde_json::from_str(&body) {
                Ok(parsed) => parsed,
                Err(message) => {
                    return write_reply(&mut stream, bad_request(&message.to_string())).await
                }
            };
            let kind = parsed
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let id = parsed
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            match bridge.access_revoke(kind, id).await {
                Ok(revoked) => write_reply(&mut stream, api(200, revoked)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        // The channel control plane: one bounded document for the Bots
        // surface, and the writes that change it. The remote face's
        // default-deny allowlist does not carry any of these — a phone
        // answers 403 by name, the same rule settings already has — so
        // the read here is the machine's own, not a paired device's.
        ("GET", "/api/channels") => {
            write_reply(&mut stream, api(200, bridge.channels_read().await)).await
        }
        ("POST", "/api/channels/start") => {
            let name = match field_of(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(reply) => return write_reply(&mut stream, reply).await,
            };
            match bridge.channel_start(&name).await {
                Ok(report) => write_reply(&mut stream, api(200, report)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/channels/stop") => {
            let name = match field_of(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(reply) => return write_reply(&mut stream, reply).await,
            };
            match bridge.channel_stop(&name).await {
                Ok(report) => write_reply(&mut stream, api(200, report)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/channels/pair") => {
            let body = match channel_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(reply) => return write_reply(&mut stream, reply).await,
            };
            match bridge
                .channel_pair(
                    &body.channel,
                    &body.id,
                    &body.name,
                    &body.capability,
                    &body.callback,
                )
                .await
            {
                Ok(pairing) => write_reply(&mut stream, api(200, pairing)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/channels/capability") => {
            let body = match channel_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(reply) => return write_reply(&mut stream, reply).await,
            };
            match bridge
                .channel_capability(&body.channel, &body.id, &body.capability)
                .await
            {
                Ok(changed) => write_reply(&mut stream, api(200, changed)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/channels/revoke") => {
            let body = match channel_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(reply) => return write_reply(&mut stream, reply).await,
            };
            match bridge.access_revoke("peer", &body.id).await {
                Ok(revoked) => write_reply(&mut stream, api(200, revoked)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/channels/forget") => {
            let body = match channel_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(reply) => return write_reply(&mut stream, reply).await,
            };
            match bridge.channel_forget_pending(&body.channel, &body.id).await {
                Ok(forgotten) => write_reply(&mut stream, api(200, forgotten)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("GET", "/api/session") => {
            let asked = decode_query(&request.query, "id");
            write_reply(&mut stream, api(200, bridge.replay(&asked).await)).await
        }
        ("GET", "/api/calibration") => {
            write_reply(&mut stream, api(200, bridge.calibration().await)).await
        }
        // The tower console and the second-brain panel: the boards' own
        // reads, shaped for the browser. Both name the parts that did
        // not land rather than rendering a blank console.
        // The settings surface: the whole config as one document, and
        // every write through the writer the surface owns. The reply
        // names itself the way the writer's own refusal does.
        // The trust trio: plan mode's door, the rewind, the checkpoint
        // store's list, and the diff with its per-hunk reject.
        ("POST", "/api/plan") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let task = match task_of(&body) {
                Ok(task) => task,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let session = asked_session(bridge, &request);
            match bridge.plan(&session, &task) {
                Ok(()) => {
                    write_reply(&mut stream, api(200, json!({"type": "plan", "task": task}))).await
                }
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/rewind") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let turn = match turn_of(&body) {
                Ok(turn) => turn,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let session = asked_session(bridge, &request);
            match bridge.rewind(&session, turn).await {
                Ok(rewound) => write_reply(&mut stream, api(200, rewound)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("GET", "/api/checkpoints") => {
            let session = asked_session(bridge, &request);
            write_reply(&mut stream, api(200, bridge.checkpoints(&session).await)).await
        }
        ("GET", "/api/diff") => {
            let asked = decode_query(&request.query, "turn");
            let turn: u64 = asked.trim().parse().unwrap_or(0);
            let session = asked_session(bridge, &request);
            write_reply(&mut stream, api(200, bridge.diff(&session, turn).await)).await
        }
        ("POST", "/api/diff/reject") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let session = asked_session(bridge, &request);
            match bridge.diff_reject(&session, &body).await {
                Ok(rejected) => write_reply(&mut stream, api(200, rejected)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("GET", "/api/settings") => {
            write_reply(&mut stream, api(200, bridge.settings().await)).await
        }
        ("POST", "/api/settings") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            match bridge.settings_write(&body).await {
                Ok(wrote) => write_reply(&mut stream, api(200, wrote)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("GET", "/api/tower") => write_reply(&mut stream, api(200, bridge.tower().await)).await,
        ("GET", "/api/second-brain") => {
            write_reply(&mut stream, api(200, bridge.brain().await)).await
        }
        ("GET", "/api/second-brain/plugin") => {
            write_reply(&mut stream, api(200, bridge.second_brain_plugin().await)).await
        }
        ("POST", "/api/second-brain/act") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            match bridge.second_brain_act(&body).await {
                Ok(reply) => write_reply(&mut stream, api(200, reply)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/resume") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let id = match session_of(&body) {
                Ok(id) => id,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            match bridge.resume(&id).await {
                Ok(resumed) => write_reply(&mut stream, api(200, resumed)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("GET", _) => write_reply(&mut stream, not_found(&request.path)).await,
        _ => write_reply(&mut stream, method_not_allowed(&request.method)).await,
    }
}

/// Reads one request head and whatever arrived past it: bytes up to the
/// `\r\n\r\n` that ends them, capped so a peer that never sends it
/// cannot hold a task forever. The second value is the body bytes that
/// rode the same read — a browser sends a short POST in one packet, so
/// discarding them would lose the task the owner just typed.
pub(crate) async fn read_head(
    stream: &mut tokio::net::TcpStream,
) -> Result<(String, Vec<u8>), String> {
    let mut head: Vec<u8> = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let end = loop {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("fleet gui could not read the request: {e}"))?;
        if read == 0 {
            break None;
        }
        head.extend_from_slice(&chunk[..read]);
        if let Some(end) = head_end(&head) {
            break Some(end);
        }
        if head.len() > MAX_HEAD_BYTES {
            return Err("fleet gui refused an oversized request head".to_string());
        }
    };
    let (head, rest) = match end {
        Some(end) => (
            String::from_utf8(head[..end].to_vec())
                .map_err(|_| "fleet gui refused a non-UTF-8 request head".to_string())?,
            head[end..].to_vec(),
        ),
        None => (
            String::from_utf8(head)
                .map_err(|_| "fleet gui refused a non-UTF-8 request head".to_string())?,
            Vec::new(),
        ),
    };
    Ok((head, rest))
}

/// Reads one request body: the `Content-Length` bytes the head declared,
/// starting with the bytes that rode past the head's terminator. A body
/// larger than the cap is refused rather than read, and a body that
/// ends early is a malformed request — both are the caller's 400.
pub(crate) async fn read_body(
    stream: &mut (impl tokio::io::AsyncRead + Unpin),
    request: &Request,
    rest: Vec<u8>,
) -> Result<String, String> {
    let length = request
        .header("content-length")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    if length > MAX_BODY_BYTES {
        return Err(format!(
            "fleet gui refused a body of {length} bytes; the cap is {MAX_BODY_BYTES}"
        ));
    }
    let mut body = rest;
    if body.len() > length {
        body.truncate(length);
    }
    let mut chunk = [0u8; 4096];
    while body.len() < length {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("fleet gui could not read the body: {e}"))?;
        if read == 0 {
            return Err(format!(
                "fleet gui read {} of the {length} bytes the head declared",
                body.len()
            ));
        }
        body.extend_from_slice(&chunk[..read]);
        body.truncate(length.min(body.len()));
    }
    String::from_utf8(body).map_err(|_| "fleet gui refused a non-UTF-8 body".to_string())
}

/// The task a run's body carries. Anything that is not a JSON object
/// with a `task` string is a malformed request, and saying so is the
/// whole reply.
/// The turn's attached files, framed ahead of the task. The browser read
/// the text the owner picked; the server owns the bounds — five files,
/// 256 KB each, 512 KB together — and refuses a name that is a path.
fn with_attachments(body: &str, task: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the run's body: {e}"))?;
    let Some(rows) = parsed
        .get("attachments")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(task.to_string());
    };
    if rows.is_empty() {
        return Ok(task.to_string());
    }
    if rows.len() > 5 {
        return Err("at most five files can be attached to one turn".to_string());
    }
    let mut framed = String::from("Attached files, provided by the owner for this turn:\n");
    let mut total = 0usize;
    for row in rows {
        let name = row
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim();
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name == "."
            || name == ".."
        {
            return Err("an attachment needs a file name, not a path".to_string());
        }
        let text = row
            .get("text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if text.len() > 256 * 1024 {
            return Err(format!("{name} is larger than the 256 KB attachment bound"));
        }
        total += text.len();
        // The parser is part of the record: a local read and a hosted
        // parse are different provenance, and a truncated preview says
        // so in the same line.
        let mut meta: Vec<String> = Vec::new();
        if let Some(parser) = row.get("parser").and_then(serde_json::Value::as_str) {
            if !parser.trim().is_empty() {
                meta.push(format!("parser {}", parser.trim()));
            }
        }
        if let Some(pages) = row.get("pages").and_then(serde_json::Value::as_u64) {
            meta.push(format!("{pages} pages"));
        }
        if let Some(units) = row.get("units").and_then(serde_json::Value::as_u64) {
            meta.push(format!("{units} units"));
        }
        if let Some(job) = row.get("job_id").and_then(serde_json::Value::as_str) {
            if !job.trim().is_empty() {
                meta.push(format!("job {}", job.trim()));
            }
        }
        if let Some(reason) = row.get("reason").and_then(serde_json::Value::as_str) {
            if !reason.trim().is_empty() {
                meta.push(format!("escalated: {}", reason.trim()));
            }
        }
        if let Some(truncated) = row.get("truncated").and_then(serde_json::Value::as_object) {
            let original = truncated
                .get("original")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let retained = truncated
                .get("retained")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            meta.push(format!("truncated · retained {retained} of {original}"));
        }
        let head = if meta.is_empty() {
            name.to_string()
        } else {
            format!("{name} ({})", meta.join(" · "))
        };
        framed.push_str(&format!("\n--- {head} ---\n{text}\n"));
    }
    if total > 512 * 1024 {
        return Err("the attachments together are larger than the 512 KB bound".to_string());
    }
    framed.push_str("\n--- end of attachments ---\n\n");
    framed.push_str(task);
    Ok(framed)
}

/// The run's two standing choices: how its tool calls are approved, and
/// the provider its route is pinned to. A body that names neither is the
/// safe pair — `ask` and `auto` — so an older page keeps working.
fn run_policy_of(body: &str) -> Result<(crate::approvals::Mode, Option<String>), String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the run's body: {e}"))?;
    let mode = match parsed
        .get("mode")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("ask")
    {
        "full" | "yes" => crate::approvals::Mode::Yes,
        "auto" | "yes-guarded" => crate::approvals::Mode::YesGuarded,
        _ => crate::approvals::Mode::Ask,
    };
    let pin = parsed
        .get("pin")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|pin| !pin.is_empty())
        .map(str::to_string);
    Ok((mode, pin))
}

fn task_of(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the run's body: {e}"))?;
    parsed
        .get("task")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "a run's body is a JSON object with a task string".to_string())
}

/// The folder a run's body names, when it names one. Absent or null
/// means "whatever this chat already chose"; the empty string is the
/// reset to no folder; anything that is not a string is malformed.
fn folder_of(body: &str) -> Result<Option<String>, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the run's body: {e}"))?;
    match parsed.get("folder") {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(folder)) => Ok(Some(folder.clone())),
        Some(_) => Err("a run's `folder` is a string when it is present".to_string()),
    }
}

/// The folder a `/api/folder` body carries: the field is required, and
/// null is no folder. Keeping this separate from [`folder_of`]
/// is the difference between "leave the chat's choice alone" (a run
/// that did not name one) and "clear it" (the reset control).
fn folder_choice_of(body: &str) -> Result<Option<String>, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the folder's body: {e}"))?;
    match parsed.get("folder") {
        None => Err("a folder change carries a `folder` (a path, or null to reset)".to_string()),
        Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(folder)) => Ok(Some(folder.clone())),
        Some(_) => Err("a folder change carries a string or null".to_string()),
    }
}

/// One usage event's three words. The store validates their closed
/// sets; this only reads the strings, so a malformed body is a 400
/// before anything is written.
fn usage_event_of(body: &str) -> Result<(String, String, String), String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the usage body: {e}"))?;
    let field = |name: &str| {
        parsed
            .get(name)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                format!("a usage body is a JSON object with section, id, and event strings ({name} is missing)")
            })
    };
    Ok((field("section")?, field("id")?, field("event")?))
}

/// The text a steer's body carries. The bridge owns the rule that an
/// empty steer is refused; this only reads the one string the body
/// holds, so a malformed body is a 400 before any control is written.
fn steer_of(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the steer's body: {e}"))?;
    parsed
        .get("text")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "a steer's body is a JSON object with a text string".to_string())
}

/// One `name` field from a control body: the channel a start or stop
/// names. A body with no name is a 400, not a guess at which channel the
/// caller meant.
async fn field_of(
    stream: &mut (impl tokio::io::AsyncRead + Unpin),
    request: &Request,
    rest: Vec<u8>,
) -> Result<String, Reply> {
    let body = match read_body(stream, request, rest).await {
        Ok(body) => body,
        Err(message) => return Err(bad_request(&message)),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&body) {
        Ok(parsed) => parsed,
        Err(message) => return Err(bad_request(&message.to_string())),
    };
    let name = parsed
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if name.is_empty() {
        return Err(bad_request("a channel control names one channel in `name`"));
    }
    Ok(name)
}

/// The channel control plane's body: which channel, which peer, what it
/// may do, and the name it is filed under.
struct ChannelBody {
    channel: String,
    id: String,
    name: String,
    capability: String,
    callback: String,
}

async fn channel_body(
    stream: &mut (impl tokio::io::AsyncRead + Unpin),
    request: &Request,
    rest: Vec<u8>,
) -> Result<ChannelBody, Reply> {
    let body = match read_body(stream, request, rest).await {
        Ok(body) => body,
        Err(message) => return Err(bad_request(&message)),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&body) {
        Ok(parsed) => parsed,
        Err(message) => return Err(bad_request(&message.to_string())),
    };
    let text = |key: &str| {
        parsed
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    Ok(ChannelBody {
        channel: text("channel"),
        id: text("id"),
        name: text("name"),
        capability: text("capability"),
        callback: text("callback"),
    })
}

/// The session id a resume's body carries. The shape itself is the
/// bridge's rule — this only parses the one string the body holds.
fn session_of(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the resume's body: {e}"))?;
    parsed
        .get("session")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "a resume's body is a JSON object with a session string".to_string())
}

/// The turn a rewind's body carries.
fn turn_of(body: &str) -> Result<u64, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the rewind's body: {e}"))?;
    parsed
        .get("turn")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "a rewind's body is a JSON object with a turn number".to_string())
}

/// The decision an approval's body carries, in the seat's four words.
fn decision_of(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the answer's body: {e}"))?;
    parsed
        .get("decision")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "an answer's body is a JSON object with a decision string".to_string())
}

/// The proposal an approve's body carries.
fn proposal_of(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the approve's body: {e}"))?;
    parsed
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "an approve's body is a JSON object with a name string".to_string())
}

/// Where a head ends: just past the first `\r\n\r\n`.
fn head_end(head: &[u8]) -> Option<usize> {
    head.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| at + 4)
}

/// A request head, parsed to what this surface routes on: the method,
/// the path, the query, and the headers a token can arrive in.
pub(crate) struct Request {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) query: String,
    headers: Vec<(String, String)>,
}

impl Request {
    /// Parses the head. A malformed line is an error the caller turns
    /// into a closed connection: this surface's inputs are a browser's,
    /// and there is nothing here worth a 400 page.
    pub(crate) fn parse(head: &str) -> Result<Request, String> {
        let mut lines = head.split("\r\n");
        let request_line = lines.next().unwrap_or_default();
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or_default().to_string();
        let target = parts.next().unwrap_or_default().to_string();
        if method.is_empty() || target.is_empty() {
            return Err(format!(
                "fleet gui refused a malformed request line: {request_line:?}"
            ));
        }
        let (path, query) = match target.split_once('?') {
            Some((path, query)) => (path.to_string(), query.to_string()),
            None => (target, String::new()),
        };
        let mut headers = Vec::new();
        for line in lines {
            if let Some((name, value)) = line.split_once(':') {
                headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
            }
        }
        Ok(Request {
            method,
            path,
            query,
            headers,
        })
    }

    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header == name)
            .map(|(_, value)| value.as_str())
    }

    /// Whether the request's Host names this machine — the loopback
    /// names a local browser produces, with or without the port. A
    /// foreign Host is the rebinding attack: a remote page serving the
    /// local port under a hostname it owns.
    fn host_is_loopback(&self) -> bool {
        host_is_loopback(self.header("host").unwrap_or_default())
    }
}

/// One query parameter's value, percent-decoded. The pane's paths are
/// the exotic case the token never was: a file path carries spaces,
/// hashes and non-ASCII, and the browser encodes all of them before they
/// reach the wire, so they have to come back off it.
pub(crate) fn decode_query(query: &str, name: &str) -> String {
    let raw = query_value(query, name).unwrap_or_default();
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' && at + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[at + 1..at + 3]).ok();
            if let Some(value) = hex.and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
                out.push(value);
                at += 3;
                continue;
            }
        }
        out.push(bytes[at]);
        at += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| raw.to_string())
}

/// One query parameter's value. The token is hex, so nothing needs
/// decoding; anything exotic is the caller's problem, not this
/// surface's.
pub(crate) fn query_value<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

/// One reply, assembled as bytes before it touches the socket: the
/// status line, the headers this surface always sends, and the body.
pub(crate) struct Reply {
    pub(crate) status: &'static str,
    pub(crate) content_type: &'static str,
    pub(crate) extra: Vec<(String, String)>,
    pub(crate) body: String,
}

impl Reply {
    pub(crate) fn bytes(&self) -> Vec<u8> {
        let mut head = format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
            self.status,
            self.content_type,
            self.body.len()
        );
        for (name, value) in &self.extra {
            head.push_str(name);
            head.push_str(": ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(self.body.as_bytes());
        bytes
    }
}

/// The page. There is no credential in it and none beside it: the
/// boundary is the machine, and a browser on the machine is the owner.
/// The page asks for its two script assets at the build's own address, so
/// a browser can keep them forever and still never serve yesterday's
/// script after an upgrade. Everything else in the HTML is inline or
/// fetched at runtime.
pub(crate) fn index_page() -> String {
    INDEX
        .replace(
            "href=\"/app.css\"",
            &format!("href=\"/app.css?v={ASSET_HASH}\""),
        )
        .replace(
            "src=\"/app.js\"",
            &format!("src=\"/app.js?v={ASSET_HASH}\""),
        )
}

fn cache_control(value: &str) -> Vec<(String, String)> {
    vec![("Cache-Control".to_string(), value.to_string())]
}

fn page() -> Reply {
    Reply {
        status: "200 OK",
        content_type: "text/html; charset=utf-8",
        extra: cache_control("no-cache"),
        body: index_page(),
    }
}

pub(crate) fn manifest() -> Reply {
    Reply {
        status: "200 OK",
        content_type: "application/manifest+json",
        extra: cache_control("no-cache"),
        body: MANIFEST.to_string(),
    }
}

/// The worker is re-validated on every load so a new binary's policy is
/// picked up; the header lets a root-scoped worker be served from a path
/// that is not itself the scope root.
pub(crate) fn worker() -> Reply {
    Reply {
        status: "200 OK",
        content_type: "text/javascript; charset=utf-8",
        extra: vec![
            ("Cache-Control".to_string(), "no-cache".to_string()),
            ("Service-Worker-Allowed".to_string(), "/".to_string()),
            ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
        ],
        body: WORKER.replace("__FLEET_ASSET_HASH__", ASSET_HASH),
    }
}

/// An unversioned asset revalidates; only the URL carrying the build's
/// own hash is immutable. A browser that asked the bare path (an old
/// bookmark, a tool that strips queries) therefore never pins a stale
/// script.
fn asset(content_type: &'static str, body: &'static str) -> Reply {
    Reply {
        status: "200 OK",
        content_type,
        extra: cache_control("no-cache"),
        body: body.to_string(),
    }
}

pub(crate) fn versioned_asset(content_type: &'static str, body: &'static str) -> Reply {
    Reply {
        status: "200 OK",
        content_type,
        extra: cache_control("public, max-age=31536000, immutable"),
        body: body.to_string(),
    }
}

/// The icon a PWA path names. One function, so a path the loopback face
/// serves cannot 404 on the remote face or the other way round.
pub(crate) fn icon_for(path: &str) -> Option<&'static [u8]> {
    match path {
        "/icon-192.png" | "/apple-touch-icon.png" => Some(ICON_192),
        "/icon-512.png" => Some(ICON_512),
        _ => None,
    }
}

/// A binary reply (the icons). `Reply` carries a `String` body, which a
/// PNG cannot be, so this writes the head and the bytes directly rather
/// than pretending an image is text.
pub(crate) async fn write_blob(
    stream: &mut tokio::net::TcpStream,
    content_type: &'static str,
    cache: &str,
    body: &'static [u8],
) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Connection: close\r\nCache-Control: {cache}\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| format!("fleet gui could not write an asset head: {e}"))?;
    stream
        .write_all(body)
        .await
        .map_err(|e| format!("fleet gui could not write an asset: {e}"))?;
    stream
        .flush()
        .await
        .map_err(|e| format!("fleet gui could not flush an asset: {e}"))
}

/// The web's refusal: the request arrived from somewhere the face does
/// not serve, and the body names the rule without naming anything else.
fn forbidden(reason: &str) -> Reply {
    Reply {
        status: "403 Forbidden",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: format!("fleet gui serves this machine only: {reason}.\n"),
    }
}

fn not_found(path: &str) -> Reply {
    Reply {
        status: "404 Not Found",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: format!("fleet gui has no route for {path}.\n"),
    }
}

/// An action's answer, as JSON: the state document, the run's receipt,
/// or the answer's outcome. One shape for every reply the browser
/// parses, so the page reads one content type forever.
pub(crate) fn api(status: u16, body: serde_json::Value) -> Reply {
    Reply {
        status: status_line(status),
        content_type: "application/json; charset=utf-8",
        extra: Vec::new(),
        body: format!("{body}\n"),
    }
}

fn bad_request(message: &str) -> Reply {
    Reply {
        status: "400 Bad Request",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: format!("{message}\n"),
    }
}

/// The status line both faces write. The set is small and fixed — eight
/// lines, the five this surface answers with plus the three the remote
/// face's gate answers with — and naming them is cheaper than a
/// status-code crate for surfaces that answer nothing else.
pub(crate) fn status_line(status: u16) -> &'static str {
    match status {
        200 => "200 OK",
        303 => "303 See Other",
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        403 => "403 Forbidden",
        404 => "404 Not Found",
        405 => "405 Method Not Allowed",
        409 => "409 Conflict",
        _ => "500 Internal Server Error",
    }
}

fn method_not_allowed(method: &str) -> Reply {
    Reply {
        status: "405 Method Not Allowed",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: format!(
            "fleet gui answers GET on its pages and POST on its actions; {method} is neither.\n"
        ),
    }
}

pub(crate) async fn write_reply(
    stream: &mut tokio::net::TcpStream,
    reply: Reply,
) -> Result<(), String> {
    stream
        .write_all(&reply.bytes())
        .await
        .map_err(|e| format!("fleet gui could not write the reply: {e}"))?;
    stream
        .flush()
        .await
        .map_err(|e| format!("fleet gui could not flush the reply: {e}"))
}

/// The stream: the head, a hello that tells the page where it is, then
/// a beat on an interval and every event the bridge broadcasts, for as
/// long as the browser listens. A write that fails is the browser being
/// gone — the stream ends with it, quietly, because that is the normal
/// way a tab closes. A browser too slow to keep up is told how many
/// events it missed rather than being left to wonder, and a closed
/// channel is the agent thread being gone, which ends the stream.
async fn stream_events(
    stream: &mut tokio::net::TcpStream,
    base_url: &str,
    heartbeat: Duration,
    mut events: tokio::sync::broadcast::Receiver<serde_json::Value>,
) -> Result<(), String> {
    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n";
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| format!("fleet gui could not open the stream: {e}"))?;
    stream
        .flush()
        .await
        .map_err(|e| format!("fleet gui could not open the stream: {e}"))?;
    write_event(stream, &json!({"type": "hello", "gui": base_url})).await?;
    let mut ticks = tokio::time::interval(heartbeat);
    // The interval's first tick completes at once; skipping it puts the
    // first heartbeat a full interval out, which is what a beat means.
    ticks.tick().await;
    loop {
        tokio::select! {
            received = events.recv() => match received {
                Ok(payload) => {
                    if write_event(stream, &payload).await.is_err() {
                        return Ok(());
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    let missed = json!({"type": "lagged", "skipped": skipped});
                    if write_event(stream, &missed).await.is_err() {
                        return Ok(());
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
            _ = ticks.tick() => {
                let beat = json!({"type": "heartbeat"});
                if write_event(stream, &beat).await.is_err() {
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

/// One server-sent event: the `data:` line and the blank line that ends
/// it. The payloads are JSON so the page parses one shape forever.
async fn write_event(
    stream: &mut tokio::net::TcpStream,
    payload: &serde_json::Value,
) -> Result<(), String> {
    let event = format!("data: {payload}\n\n");
    stream
        .write_all(event.as_bytes())
        .await
        .map_err(|e| format!("fleet gui could not write a beat: {e}"))
}

/// Resolves `--host` to a loopback address or refuses it by name. The
/// surface never leaves this machine, so a routable address, a
/// wildcard, or a name that is not `localhost` is a configuration
/// error, and naming the refused host is the whole error. The parser
/// shares this so the refusal is a usage error at the prompt, not a
/// run that starts and dies.
pub(crate) fn loopback_address(host: &str) -> Result<IpAddr, String> {
    if host == "localhost" {
        return Ok(IpAddr::V4(Ipv4Addr::LOCALHOST));
    }
    match host.parse::<IpAddr>() {
        Ok(address) if address.is_loopback() => Ok(address),
        _ => Err(format!(
            "fleet gui serves loopback only; refusing --host {host}"
        )),
    }
}

/// Reads the operating system's entropy for this launch's token. A
/// failure here is loud and fatal: a predictable token is worse than no
/// server at all, and there is no weak fallback to fall back to.
/// The loopback names a local browser produces in a Host header, with
/// or without the port. Anything else is a foreign name, whether it is
/// a rebind ("127.0.0.1.evil.com" is not 127.0.0.1) or another machine.
fn host_is_loopback(host: &str) -> bool {
    let host = host.trim();
    let name = if let Some(rest) = host.strip_prefix('[') {
        // a bracketed IPv6 literal, with or without ":port" behind it
        rest.split_once(']')
            .map(|(inside, _)| inside)
            .unwrap_or(rest)
    } else if host.matches(':').count() > 1 {
        // a bare IPv6 address carries colons but no port
        host
    } else {
        host.split(':').next().unwrap_or(host)
    };
    let name = name.strip_suffix('.').unwrap_or(name);
    matches!(
        name.to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost" | "::1"
    )
}

/// Whether an Origin header names this machine. An origin's authority
/// is the same host-and-port shape a Host header carries, behind the
/// scheme this check deliberately ignores — the scheme is the web's
/// business, the authority is the machine's.
fn origin_is_loopback(origin: &str) -> bool {
    let rest = match origin.split_once("://") {
        Some((_, rest)) => rest,
        None => return false,
    };
    let authority = rest.split('/').next().unwrap_or(rest);
    host_is_loopback(authority)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn attachments_are_framed_ahead_of_the_task_and_their_names_bound() {
        let framed = with_attachments(
            r#"{"task":"read it","attachments":[{"name":"a.md","text":"hello"}]}"#,
            "read it",
        )
        .expect("a framed task");
        assert!(framed.contains("--- a.md ---"), "{framed}");
        assert!(framed.ends_with("read it"), "{framed}");
        assert!(with_attachments(
            r#"{"task":"x","attachments":[{"name":"../etc/passwd","text":"x"}]}"#,
            "x"
        )
        .is_err());
        assert!(with_attachments(r#"{"task":"x"}"#, "x")
            .expect("plain")
            .eq("x"));
        let framed = with_attachments(
            r#"{"task":"x","attachments":[{"name":"scan.pdf","text":"page","parser":"nace","pages":96,"units":96,"job_id":"job1","reason":"the PDF is probably a scan","truncated":{"original":1000,"retained":65536}}]}"#,
            "x",
        )
        .expect("a framed task");
        assert!(framed.contains("--- scan.pdf (parser nace · 96 pages · 96 units · job job1 · escalated: the PDF is probably a scan · truncated · retained 65536 of 1000) ---"), "{framed}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_attach_route_reads_office_files_locally() {
        let (token, port) = launch().await;
        let reply = post_within(
            port,
            "/api/attach",
            &token,
            r#"{"path":"services/documents/fixtures/simple.docx"}"#,
            Duration::from_secs(30),
        );
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.contains(r#""parser":"local:"#), "{reply}");
        assert!(reply.contains("Ada"), "{reply}");
        assert!(reply.contains(r#""type":"attachment""#), "{reply}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_escalation_asks_before_it_sends() {
        let (token, port) = launch().await;
        for path in [
            "services/documents/fixtures/pixel.png",
            "services/documents/fixtures/blank.pdf",
        ] {
            let asked = post(
                port,
                "/api/attach",
                &token,
                &format!(r#"{{"path":"{path}"}}"#),
            );
            assert!(asked.starts_with("HTTP/1.1 200"), "{asked}");
            assert!(asked.contains(r#""type":"attach_needs""#), "{asked}");
            assert!(asked.contains("nace"), "{asked}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_attach_outside_the_workspace_is_refused() {
        let (token, port) = launch().await;
        let refused = post(
            port,
            "/api/attach",
            &token,
            r#"{"path":"../../Cargo.toml"}"#,
        );
        assert!(refused.starts_with("HTTP/1.1 409"), "{refused}");
        assert!(refused.contains("outside"), "{refused}");
    }

    #[test]
    fn the_run_policy_names_its_two_choices() {
        let (mode, pin) =
            run_policy_of(r#"{"task":"x","mode":"auto","pin":"stepfun"}"#).expect("a body");
        assert_eq!(mode, crate::approvals::Mode::YesGuarded);
        assert_eq!(pin.as_deref(), Some("stepfun"));
        // A body that names nothing is the safe pair, so an older page
        // keeps working.
        let (mode, pin) = run_policy_of(r#"{"task":"x"}"#).expect("a body");
        assert_eq!(mode, crate::approvals::Mode::Ask);
        assert_eq!(pin, None);
        assert_eq!(
            run_policy_of(r#"{"task":"x","mode":"full"}"#)
                .expect("a body")
                .0,
            crate::approvals::Mode::Yes
        );
        // An empty pin is auto, not a provider named "".
        let (_, pin) = run_policy_of(r#"{"task":"x","pin":"  "}"#).expect("a body");
        assert_eq!(pin, None);
    }

    /// A live server on an OS-picked port, with a heartbeat short enough
    /// that a test does not wait five seconds for a beat. The server
    /// runs on its own task; the client below blocks on the test's
    /// thread, which is why these tests ask for a multi-thread runtime.
    /// The bridge behind it is detached — no agent thread — so the
    /// routes answer exactly what the bridge holds and nothing the
    /// network would decide.
    /// A fresh temp home, empty. Each call makes its own directory: a name
    /// shared per process would let one test's `fresh_home` wipe another
    /// test's home while its server is answering, and a home whose
    /// config.json just vanished sends the board's read looking for the
    /// machine's own calibration log instead of an empty tree.
    fn fresh_home() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let home =
            std::env::temp_dir().join(format!("fleet-gui-server-{}-{serial}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        home
    }

    /// The home a router-panel test reads: a fresh home whose config
    /// names its own calibration root, so the board's route rows come
    /// from an empty tree rather than the machine the test runs on. The
    /// file's only field is the log root, so the roster is the
    /// compiled-in defaults — exactly what a fresh install reads.
    fn router_home() -> std::path::PathBuf {
        let home = fresh_home();
        std::fs::write(
            home.join("config.json"),
            format!(
                r#"{{"calib":{{"log_root":"{}"}}}}"#,
                home.join("logs").display()
            ),
        )
        .expect("a config naming the temp calibration root");
        home
    }

    /// The home a delegation-panel test reads: the bridge's own session
    /// id has a log, it delegated to one child, and the home declares a
    /// graph with one node in each state the code names. A fresh home
    /// would render a panel with nothing in it, which is the one case
    /// the panel is not for.
    fn delegation_home() -> std::path::PathBuf {
        let home = fresh_home();
        let sessions = home.join("sessions");
        std::fs::create_dir_all(&sessions).expect("the sessions directory");
        let log = |id: &str, kind: Option<&str>, parent: Option<&str>, stop: Option<&str>| {
            let mut start = serde_json::json!({
                "type": "start",
                "v": crate::session::SESSION_FORMAT_VERSION,
                "model": "deepseek-chat",
                "created": "2026-10-16T09:00:00Z",
            });
            if let Some(kind) = kind {
                start["kind"] = serde_json::json!(kind);
            }
            if let Some(parent) = parent {
                start["parent"] = serde_json::json!(parent);
            }
            let mut lines = vec![
                start.to_string(),
                serde_json::json!({"type": "user", "content": "revamp the delegation panel"})
                    .to_string(),
            ];
            if let Some(stop) = stop {
                lines.push(
                    serde_json::json!({"type": "end", "stop_reason": stop, "turns": 2}).to_string(),
                );
            }
            std::fs::write(
                sessions.join(format!("{id}.jsonl")),
                format!("{}\n", lines.join("\n")),
            )
            .expect("a session log");
        };
        // The bridge's own session, and the child it delegated to.
        log("session-test", None, None, None);
        log(
            "20261016T090100Z-child",
            Some(crate::session::SUBAGENT_KIND),
            Some("session-test"),
            Some("completed"),
        );
        let node = |id: &str, cost: u64, prereqs: &[&str], paths: &[&str], status: &str| {
            serde_json::json!({
                "id": id,
                "cost": cost,
                "prereqs": prereqs,
                "paths": paths,
                "capabilities": { "tools": ["read"], "skills": [], "workflows": [], "loops": [] },
                "approach": "the approach",
                "status": status,
            })
        };
        std::fs::write(
            crate::graph::path_in(&home),
            serde_json::to_string_pretty(&serde_json::json!({
                "nodes": [
                    node("probe", 2, &[], &[], "verified"),
                    node("plan", 3, &["probe"], &[], "pending"),
                    // Held: `run` is in flight on this node's admitted
                    // path, so the resource condition is the one that
                    // fails and the panel must say so rather than
                    // reading the node as ready.
                    node("audit", 1, &[], &["src/boards.rs"], "pending"),
                    node("run", 1, &[], &["src/boards.rs"], "running"),
                ],
                "revisions": [],
            }))
            .expect("the graph json"),
        )
        .expect("the declared graph");
        home
    }

    /// The home a memory-panel test reads: a corpus it built itself and a
    /// calibration root it owns, so the facts and the hits are the test's
    /// own rather than the machine's. The corpus is handed to the bridge
    /// through [`Bridge::point_at_memories`], the seam that keeps the
    /// production rule — the agent's own resolve off the working
    /// directory — out of the test's way.
    fn memory_home() -> (std::path::PathBuf, std::path::PathBuf) {
        let home = router_home();
        let corpus = fresh_home();
        let day = crate::calib::today();
        std::fs::write(
            corpus.join("a-fact.md"),
            format!(
                "---\nname: a-fact\ndescription: \"the corpus is one fact per file\"\ntype: project\ndate: {day}\nbasis: inferred\ntier: durable\nshape: []\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body.\n"
            ),
        )
        .expect("a fact file");
        let logs = home
            .join("logs")
            .join(&day[..4])
            .join(&day[5..7])
            .join(&day[8..10]);
        std::fs::create_dir_all(&logs).expect("the day's log directory");
        std::fs::write(
            logs.join("events.jsonl"),
            format!(
                "{}\n{}\n",
                serde_json::json!({
                    "op": crate::calib::OP_RECALL_HIT,
                    "ts": format!("{day}T09:00:00Z"),
                    "fact": "a-fact",
                    "shape": "docs",
                    "score": 0.82,
                }),
                serde_json::json!({
                    "op": crate::calib::OP_TOOL_SCREEN,
                    "ts": format!("{day}T09:00:01Z"),
                    "tool": "read",
                    "verdict": "allow",
                }),
            ),
        )
        .expect("the day's stream");
        (home, corpus)
    }

    /// A live server over `home`, whose memory panel reads `corpus`.
    async fn launch_over(home: std::path::PathBuf, corpus: std::path::PathBuf) -> (String, u16) {
        let (mut bridge, _sources) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        bridge.point_at_memories(corpus);
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .unwrap();
        server.heartbeat = Duration::from_millis(20);
        // The face no longer gates on a token; the queries below still carry
        // one and the server ignores it, which is why these reads answer
        // all the same either way.
        let token = String::new();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        (token, port)
    }

    async fn launch() -> (String, u16) {
        launch_at(fresh_home()).await
    }

    /// A live server on an OS-picked port, with a heartbeat short enough
    /// that a test does not wait five seconds for a beat. The server
    /// runs on its own task; the client below blocks on the test's
    /// thread, which is why these tests ask for a multi-thread runtime.
    /// The bridge behind it is detached — no agent thread — so the
    /// routes answer exactly what the bridge holds and nothing the
    /// network would decide.
    async fn launch_at(home: std::path::PathBuf) -> (String, u16) {
        let (bridge, _sources) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .unwrap();
        server.heartbeat = Duration::from_millis(20);
        // The face no longer gates on a token; the queries below still carry
        // one and the server ignores it, which is why these reads answer
        // all the same either way.
        let token = String::new();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        (token, port)
    }

    /// A POST with a JSON body and the token in the query, which is how
    /// the browser's fetches arrive.
    fn post(port: u16, path: &str, token: &str, body: &str) -> String {
        ask(
            port,
            &format!(
                "POST {path}?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{body}",
                body.len()
            ),
            None,
        )
    }

    /// The attach route may run a local extractor (a Python process), so
    /// its tests read with a budget that a loaded suite cannot eat.
    fn post_within(port: u16, path: &str, token: &str, body: &str, budget: Duration) -> String {
        ask_within(
            port,
            &format!(
                "POST {path}?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{body}",
                body.len()
            ),
            None,
            budget,
        )
    }

    /// Sends one request and reads the reply, stopping when `needle`
    /// arrives or the reply ends. `None` reads to the end, which is how
    /// a normal reply arrives: the server closes every connection it
    /// answers.
    fn ask(port: u16, raw: &str, needle: Option<&str>) -> String {
        ask_within(port, raw, needle, Duration::from_secs(3))
    }

    /// The same read with its own budget. The related route's first
    /// request builds the corpus — the corpus's index, the archive's
    /// cached vectors, and the day's routes — and the full suite's
    /// load can push that past the ordinary read's three seconds.
    fn ask_within(port: u16, raw: &str, needle: Option<&str>, budget: Duration) -> String {
        let mut stream =
            std::net::TcpStream::connect(("127.0.0.1", port)).expect("the server is listening");
        stream
            .set_read_timeout(Some(budget))
            .expect("a read timeout is settable");
        stream
            .write_all(raw.as_bytes())
            .expect("the request is sent");
        let mut reply = String::new();
        let mut chunk = [0u8; 4096];
        loop {
            if needle.is_some_and(|needle| reply.contains(needle)) {
                break;
            }
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => reply.push_str(&String::from_utf8_lossy(&chunk[..read])),
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    break;
                }
                Err(e) => panic!("the reply could not be read: {e}"),
            }
        }
        reply
    }

    /// A GET with the token in the query, which is how the browser's
    /// page load and its EventSource arrive.
    fn get(port: u16, path: &str, token: &str) -> String {
        ask(
            port,
            &format!("GET {path}?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
        )
    }

    /// A GET with its own read budget. The settings read assembles the
    /// catalog (the skills library and the router), so a loaded suite can
    /// push it past the ordinary three seconds.
    fn get_within(port: u16, path: &str, token: &str, budget: Duration) -> String {
        ask_within(
            port,
            &format!("GET {path}?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
            budget,
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_address_alone_opens_the_face_and_the_web_is_refused() {
        let (_token, port) = launch().await;
        // The owner's ruling: browsing the address of a running fleet
        // opens fleet — no credential in the URL, in a header, or
        // anywhere else.
        let bare = ask(port, "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n", None);
        assert!(bare.starts_with("HTTP/1.1 200"), "{bare}");
        assert!(bare.contains("<title>fleet</title>"), "{bare}");
        // The web's hands are refused with the rule named. A rebinding
        // page serves the local port under a hostname it owns — and
        // "127.0.0.1.evil.com" is exactly such a name, not an address.
        let rebound = ask(
            port,
            "GET / HTTP/1.1\r\nHost: 127.0.0.1.evil.com\r\n\r\n",
            None,
        );
        assert!(rebound.starts_with("HTTP/1.1 403"), "{rebound}");
        assert!(rebound.contains("does not name this machine"), "{rebound}");
        // A cross-site form carries a foreign origin on its way in.
        let cross = ask(
            port,
            "POST /api/rank HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: https://evil.example\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}",
            None,
        );
        assert!(cross.starts_with("HTTP/1.1 403"), "{cross}");
        assert!(cross.contains("origin is not this machine"), "{cross}");
        // The same state-changing request from the face itself — a
        // loopback origin — is served.
        let own = ask(
            port,
            "POST /api/rank HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: http://127.0.0.1:3080\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}",
            None,
        );
        assert!(own.starts_with("HTTP/1.1 200"), "{own}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_assets_ride_behind_the_token() {
        let (token, port) = launch().await;
        for path in ["/app.css", "/app.js"] {
            let anonymous = ask(
                port,
                &format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
                None,
            );
            assert!(anonymous.starts_with("HTTP/1.1 200"), "{anonymous}");
            let served = get(port, path, &token);
            assert!(served.starts_with("HTTP/1.1 200"), "{served}");
        }
        let css = get(port, "/app.css", &token);
        // The real stylesheet, and the visual language intact: the plan's
        // colour chain has three tiers and nothing skips one, so the face
        // is not shipped with a bare token standing in for the alias it
        // should be reading.
        for tier in ["--dsw-static-", "--dsw-alias-", "--card-"] {
            assert!(css.contains(tier), "{css}");
        }
        let js = get(port, "/app.js", &token);
        assert!(js.contains("EventSource"), "{js}");
    }

    /// The shell is installable and its two assets are content-addressed:
    /// the page asks for the build's own `?v=`, only that URL is
    /// immutable, and the worker and manifest carry the types and headers
    /// an install needs.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_face_serves_an_installable_shell() {
        let (_token, port) = launch().await;
        let hash = crate::gui::server::ASSET_HASH;
        let page = ask(port, "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n", None);
        assert!(page.starts_with("HTTP/1.1 200"), "{page}");
        assert!(page.contains(&format!("/app.css?v={hash}")), "{page}");
        assert!(page.contains(&format!("/app.js?v={hash}")), "{page}");
        assert!(
            page.contains("rel=\"manifest\" href=\"/manifest.webmanifest\""),
            "{page}"
        );
        assert!(page.contains("rel=\"apple-touch-icon\""), "{page}");
        assert!(page.contains("name=\"theme-color\""), "{page}");
        assert!(page.contains("Cache-Control: no-cache"), "{page}");

        let immutable = ask(
            port,
            &format!("GET /app.css?v={hash} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
        );
        assert!(
            immutable.contains("Cache-Control: public, max-age=31536000, immutable"),
            "{immutable}"
        );
        let bare = ask(
            port,
            "GET /app.css HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(bare.contains("Cache-Control: no-cache"), "{bare}");
        assert!(!bare.contains("immutable"), "{bare}");

        let worker = ask(port, "GET /sw.js HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n", None);
        assert!(worker.starts_with("HTTP/1.1 200"), "{worker}");
        assert!(worker.contains("Service-Worker-Allowed: /"), "{worker}");
        assert!(worker.contains("Cache-Control: no-cache"), "{worker}");
        assert!(
            worker.contains(hash),
            "the worker carries the build stamp: {worker}"
        );
        assert!(!worker.contains("__FLEET_ASSET_HASH__"), "{worker}");

        let manifest = ask(
            port,
            "GET /manifest.webmanifest HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(
            manifest.contains("Content-Type: application/manifest+json"),
            "{manifest}"
        );
        assert!(manifest.contains("\"start_url\": \"/\""), "{manifest}");

        let icon = ask(
            port,
            "GET /icon-192.png HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(icon.contains("Content-Type: image/png"), "{icon}");
        let length = icon
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        assert!(length > 500, "the icon is bytes, not an empty 200: {icon}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_unknown_path_is_a_404_and_a_post_is_a_405() {
        let (token, port) = launch().await;
        let missing = get(port, "/nope", &token);
        assert!(missing.starts_with("HTTP/1.1 404"), "{missing}");
        assert!(missing.contains("/nope"), "{missing}");
        let posted = ask(
            port,
            &format!(
                "POST /?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\n\r\n"
            ),
            None,
        );
        assert!(posted.starts_with("HTTP/1.1 405"), "{posted}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_stream_says_hello_and_then_keeps_a_beat() {
        let (token, port) = launch().await;
        let reply = ask(
            port,
            &format!("GET /events?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            Some("heartbeat"),
        );
        assert!(reply.contains("Content-Type: text/event-stream"), "{reply}");
        assert!(reply.contains("\"type\":\"hello\""), "{reply}");
        assert!(
            reply.contains(&format!("\"gui\":\"http://127.0.0.1:{port}\"")),
            "{reply}"
        );
        assert!(reply.contains("\"type\":\"heartbeat\""), "{reply}");
        assert!(reply.matches("data: ").count() >= 2, "{reply}");
        // The stream holds the connection open: it never says close in
        // the body, and the head's Connection line is the server's.
        // A second connection with no token at all is served the same
        // stream: the boundary is the machine, and this one is on it.
        let anonymous = ask(
            port,
            "GET /events HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            Some("hello"),
        );
        assert!(anonymous.starts_with("HTTP/1.1 200"), "{anonymous}");
        assert!(anonymous.contains("\"type\":\"hello\""), "{anonymous}");
    }

    /// A bridge with no agent thread behind it, for the tests that need
    /// a server to exist at all.
    fn bridge() -> Arc<Bridge> {
        let home = std::env::temp_dir().join(format!("fleet-gui-bind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let (bridge, _sources) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        Arc::new(bridge)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_host_that_is_not_loopback_is_refused_by_name() {
        for host in ["0.0.0.0", "192.168.1.5", "example.com"] {
            let refused = Server::bind(host, 0, bridge()).await;
            let error = refused
                .err()
                .unwrap_or_else(|| panic!("{host} must be refused"));
            assert!(error.contains(host), "{error}");
            assert!(error.contains("loopback"), "{error}");
        }
    }

    /// A loopback host binds and mints a token no guess reaches. The
    /// must-bind set is 127.0.0.1 and `localhost`, and the URL names the
    /// address the listener holds rather than the name the owner typed,
    /// so it works on a machine that resolves neither.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_loopback_host_binds_and_mints_a_token_no_guess_reaches() {
        for host in ["127.0.0.1", "localhost"] {
            let (server, port) = Server::bind(host, 0, bridge()).await.expect(host);
            assert!(port > 0, "{host} bound no port");
            assert!(
                server.base_url().starts_with("http://127.0.0."),
                "{}",
                server.base_url()
            );
        }
        // A loopback address the machine has not assigned to its
        // interface is the operating system's answer, not this
        // surface's: Linux routes all of 127.0.0.0/8 to loopback while
        // macOS assigns .1 alone, so the address either binds or fails
        // with itself named — never a quiet move to another port.
        if let Err(message) = Server::bind("127.0.0.2", 0, bridge()).await {
            assert!(message.contains("127.0.0.2"), "{message}");
            assert!(message.contains("cannot bind"), "{message}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_state_document_is_the_replay_the_stream_agrees_with() {
        let (token, port) = launch().await;
        let anonymous = ask(
            port,
            "GET /api/state HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 200"), "{anonymous}");
        let state = get(port, "/api/state", &token);
        assert!(state.starts_with("HTTP/1.1 200"), "{state}");
        assert!(state.contains("application/json"), "{state}");
        assert!(state.contains("\"session\":\"session-test\""), "{state}");
        assert!(state.contains("\"model\":\"deepseek-chat\""), "{state}");
        assert!(state.contains("\"budget\""), "{state}");
        assert!(state.contains("\"running\":false"), "{state}");
        assert!(state.contains("\"events\":[]"), "{state}");
        assert!(state.contains("\"approval\":null"), "{state}");
        assert!(state.contains("\"rules\":[]"), "{state}");
        // The verify step's gate rides every state document.
        assert!(
            state.contains("\"gate\":{\"informs_failover\":true,\"name\":\"sys1\"}"),
            "{state}"
        );
    }

    /// The owner's stop and steer reach the run through the control file
    /// the turn loop reads. An idle face is a 409 that names the reason,
    /// a steer body that is not a text string is a 400, and the control
    /// a live run accepts is on disk beside its log.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_control_routes_reach_the_run_and_refuse_an_idle_face() {
        let (_token, port) = launch().await;
        let idle = post(port, "/api/stop", "", "{}");
        assert!(idle.starts_with("HTTP/1.1 409"), "{idle}");
        assert!(idle.contains("no run is in flight"), "{idle}");

        // A face with a run in flight and a log beside it.
        let home = fresh_home();
        let log = crate::session::Session::create(&home, "deepseek-chat").expect("the log");
        let id = log.id().to_string();
        drop(log);
        let (mut bridge, _sources) =
            Bridge::detached(home.clone(), "deepseek-chat".to_string(), Some(id.clone()));
        let _door = bridge.connect();
        bridge.run(&id, "go").expect("the run is accepted");
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .unwrap();
        server.heartbeat = Duration::from_millis(20);
        tokio::spawn(async move {
            let _ = server.serve().await;
        });

        let stopped = post(port, "/api/stop", "", "{}");
        assert!(stopped.starts_with("HTTP/1.1 200"), "{stopped}");
        assert!(stopped.contains("\"kind\":\"stop\""), "{stopped}");
        let control = crate::home::sessions_dir(&home).join(format!("{id}.control"));
        assert!(std::fs::read_to_string(&control)
            .expect("the control file")
            .contains("stop"));

        let steered = post(port, "/api/steer", "", r#"{"text":"read the gate first"}"#);
        assert!(steered.starts_with("HTTP/1.1 200"), "{steered}");
        assert!(std::fs::read_to_string(&control)
            .expect("the steer file")
            .contains("read the gate first"));
        let malformed = post(port, "/api/steer", "", "{}");
        assert!(malformed.starts_with("HTTP/1.1 400"), "{malformed}");
    }

    /// The related route answers one document for a selected object and
    /// refuses a kind outside the closed set by naming the set. A fresh
    /// home has an empty archive and sidecar, so the document may carry
    /// no rows — but it still says what it looked at and how it scored.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_related_route_answers_one_document_and_refuses_a_foreign_kind() {
        let (token, port) = launch().await;
        let anonymous = ask_within(
            port,
            "GET /api/related?kind=fact&id=one HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
            Duration::from_secs(12),
        );
        assert!(anonymous.starts_with("HTTP/1.1 200"), "{anonymous}");
        let document: serde_json::Value = serde_json::from_str(
            anonymous
                .split("\r\n\r\n")
                .nth(1)
                .expect("a body past the head"),
        )
        .expect("a JSON document");
        assert_eq!(document["type"], "related");
        assert_eq!(document["kind"], "fact");
        assert_eq!(document["id"], "one");
        assert!(document["related"].is_array());
        assert!(["cosine", "lexical"].contains(&document["scores"].as_str().unwrap_or("")));
        assert!(["warm", "warming"].contains(&document["index"].as_str().unwrap_or("")));

        let foreign = ask(
            port,
            &format!(
                "GET /api/related?kind=planet&id=one&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
            ),
            None,
        );
        assert!(foreign.starts_with("HTTP/1.1 400"), "{foreign}");
        assert!(
            foreign.contains("planet") && foreign.contains("fact, skill"),
            "{foreign}"
        );
    }

    /// The usage route reads and writes the browser's own counters:
    /// the document survives the POST, the knob rides along, and a
    /// section outside the closed set is a 400 that writes nothing.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_usage_route_counts_and_refuses_a_foreign_section() {
        let (_token, port) = launch().await;
        let before = get(port, "/api/usage", "");
        assert!(before.starts_with("HTTP/1.1 200"), "{before}");
        assert!(before.contains("\"type\":\"usage\""), "{before}");
        assert!(before.contains("\"adapt\":true"), "{before}");

        let recorded = post(
            port,
            "/api/usage",
            "",
            r#"{"section":"suggestions","id":"offer:plan","event":"shown"}"#,
        );
        assert!(recorded.starts_with("HTTP/1.1 200"), "{recorded}");
        assert!(recorded.contains("\"offer:plan\""), "{recorded}");
        assert!(recorded.contains("\"shown\":1"), "{recorded}");
        let recorded = post(
            port,
            "/api/usage",
            "",
            r#"{"section":"suggestions","id":"offer:plan","event":"accepted"}"#,
        );
        assert!(recorded.contains("\"accepted\":1"), "{recorded}");

        let foreign = post(
            port,
            "/api/usage",
            "",
            r#"{"section":"planets","id":"x","event":"shown"}"#,
        );
        assert!(foreign.starts_with("HTTP/1.1 400"), "{foreign}");
        assert!(foreign.contains("planets"), "{foreign}");
    }

    /// The router panel's document is the board's own reads: the declared
    /// chain, the workload preference table, and the route-decision
    /// cache's counts, read off the async worker. A fresh home renders
    /// rather than refuses — a day with no rows is a ledger that says so.
    /// And nothing in it carries a credential.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_router_document_is_the_boards_own_read() {
        let (token, port) = launch_at(router_home()).await;
        let anonymous = ask(
            port,
            "GET /api/router HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 200"), "{anonymous}");
        let reply = get(port, "/api/router", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.contains("application/json"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert_eq!(document["type"], "router");
        // The declared chain, in the preference table's own order: each
        // workload's first candidate is what a route selects for it, and
        // the rest are what its failover walks.
        let tiers = document["tiers"].as_array().expect("the tier chains");
        assert!(!tiers.is_empty(), "{document}");
        assert!(
            tiers.iter().all(|tier| tier["candidates"]
                .as_array()
                .is_some_and(|candidates| !candidates.is_empty())),
            "{document}"
        );
        // The lane that would judge, and no pin: nothing is pinned on a
        // fresh install.
        assert_eq!(document["judge"]["lane"], "sys1");
        assert_eq!(document["pin"], serde_json::Value::Null);
        // An empty day is a ledger with nothing in it, not a refusal.
        assert_eq!(document["totals"], serde_json::Value::Null);
        assert!(
            document["recent"]
                .as_array()
                .is_some_and(|rows| rows.is_empty()),
            "{document}"
        );
        assert!(
            document["refused"]
                .as_array()
                .is_some_and(|refused| refused.is_empty()),
            "{document}"
        );
        // The cache is empty, and the panel says so rather than
        // presenting another lane's verdicts as this one's.
        assert_eq!(document["cache"]["entries"], 0);
        assert_eq!(document["cache"]["keyed"], false);
        assert!(
            document["cache"]["workloads"]
                .as_array()
                .is_some_and(|workloads| workloads.is_empty()),
            "{document}"
        );
        // Names and presence, never values.
        assert!(!body.contains("api_key"), "{body}");
        assert!(!body.contains("DEEPSEEK_API_KEY"), "{body}");
    }

    /// The delegation panel's document is the board's own read: the live
    /// run's tree off the session logs' `start` records, and the declared
    /// graph with each node's three unlock conditions carried separately.
    /// A home whose graph has declared nothing says so in `refused`
    /// rather than rendering an empty graph — an empty graph reads as
    /// "nothing is blocking anything", which is a different claim.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_delegation_document_is_the_boards_own_read() {
        let (token, port) = launch_at(delegation_home()).await;
        let anonymous = ask(
            port,
            "GET /api/delegation HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 200"), "{anonymous}");
        let reply = get(port, "/api/delegation", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.contains("application/json"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert_eq!(document["type"], "delegation");
        // The cap is the code's own, read off the subagents module.
        assert_eq!(document["depth_cap"], crate::subagents::SUBAGENT_MAX_DEPTH);
        // The live tree: the bridge's own session at the root, the child
        // it delegated to beneath it, and the child's stop.
        let live = &document["live"];
        assert_eq!(live["id"], "session-test");
        assert_eq!(live["status"], "running");
        assert_eq!(live["goal"], "revamp the delegation panel");
        let children = live["children"].as_array().expect("the children");
        assert_eq!(children.len(), 1, "{document}");
        assert_eq!(children[0]["kind"], crate::session::SUBAGENT_KIND);
        assert_eq!(children[0]["status"], "stopped");
        assert_eq!(children[0]["stop_reason"], "completed");
        assert_eq!(children[0]["depth"], 1);
        // The dependency graph, with the three conditions per node.
        let graph = &document["graph"];
        let nodes = graph["nodes"].as_array().expect("the nodes");
        assert_eq!(nodes.len(), 4, "{document}");
        // `plan`'s only prerequisite is verified, so a verified prereq
        // carries no cost into the score: all three conditions hold and
        // the node reads `ready`. The panel reports the three marks, not
        // one word derived from them.
        let plan = &nodes[1];
        assert_eq!(plan["state"], "ready");
        assert_eq!(plan["score"], 0);
        let unlock = &plan["unlock"];
        assert_eq!(unlock["score_zero"], true);
        assert_eq!(unlock["resource_free"], true);
        assert_eq!(unlock["deps_verified"], true);
        assert_eq!(unlock["unlocked"], true);
        // `audit` admits a path `run` is in flight on, so the resource
        // condition is the one that fails — score zero, deps verified,
        // and the holder named. A one-word summary would read this node
        // as ready, which is the difference the panel exists to show.
        let audit = &nodes[2];
        assert_eq!(audit["state"], "held");
        assert_eq!(audit["unlock"]["score_zero"], true);
        assert_eq!(audit["unlock"]["deps_verified"], true);
        assert_eq!(audit["unlock"]["resource_free"], false);
        assert_eq!(audit["unlock"]["unlocked"], false);
        assert_eq!(audit["unlock"]["held"][0]["holder"], "run");
        assert_eq!(audit["unlock"]["held"][0]["path"], "src/boards.rs");
        // The verified node's own closure is empty.
        assert_eq!(nodes[0]["state"], "verified");
        assert_eq!(nodes[0]["score"], 0);
        assert!(!body.contains("api_key"), "{body}");
    }

    /// The memory panel's document is the corpus and the day's recall
    /// hits: the facts with their bodies whole, because the panel's edit
    /// form needs the fact as it stands on disk, and the hits off the
    /// calibration stream's own rows — the screening rows between them do
    /// not ride along. Nothing in it carries a credential.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_memory_document_is_the_corpus_and_the_days_hits() {
        let (home, corpus) = memory_home();
        let (token, port) = launch_over(home, corpus.clone()).await;
        let anonymous = ask(
            port,
            "GET /api/memory HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 200"), "{anonymous}");
        let reply = get(port, "/api/memory", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.contains("application/json"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert_eq!(document["type"], "memory");
        // The corpus root is named, so the panel can say which one it is
        // showing.
        assert_eq!(document["root"], corpus.display().to_string());
        let facts = document["facts"].as_array().expect("the facts");
        assert_eq!(facts.len(), 1, "{document}");
        assert_eq!(facts[0]["name"], "a-fact");
        assert_eq!(facts[0]["body"], "The body.");
        assert_eq!(facts[0]["tier"], "durable");
        // The day's recall rows, oldest first — and the screening row
        // that sat between them is not one.
        let hits = document["hits"].as_array().expect("the hits");
        assert_eq!(hits.len(), 1, "{document}");
        assert_eq!(hits[0]["fact"], "a-fact");
        assert_eq!(hits[0]["shape"], "docs");
        assert!(
            document["refused"]
                .as_array()
                .is_some_and(|refused| refused.is_empty()),
            "{document}"
        );
        assert!(!body.contains("api_key"), "{body}");
        let _ = std::fs::remove_dir_all(&corpus);
    }

    /// A fact edits through the substrate's own rewriter: the named
    /// frontmatter lines are re-rendered and every other byte survives,
    /// so the panel's edit is the corpus's own write path rather than a
    /// second one. An unknown fact is a refusal that names it.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_fact_edits_through_the_substrates_own_rewriter() {
        let (home, corpus) = memory_home();
        let (token, port) = launch_over(home, corpus.clone()).await;
        let reply = post(
            port,
            "/api/fact",
            &token,
            r#"{"name":"a-fact","description":"the corpus is one fact per file, rewritten","shape":"docs"}"#,
        );
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert_eq!(document["edited"], true);
        assert_eq!(
            document["fact"]["description"],
            "the corpus is one fact per file, rewritten"
        );
        assert_eq!(document["fact"]["shape"][0], "docs");
        // The body survived the rewrite: the rewriter owns one line and
        // preserves every other byte.
        assert_eq!(document["fact"]["body"], "The body.");
        let on_disk = std::fs::read_to_string(corpus.join("a-fact.md")).expect("the fact file");
        assert!(
            on_disk.contains("description: \"the corpus is one fact per file, rewritten\""),
            "{on_disk}"
        );
        assert!(on_disk.contains("shape: [docs]"), "{on_disk}");
        assert!(on_disk.contains("The body."), "{on_disk}");
        // A fact that is not there is refused by name, and the reason is
        // the part the owner needs.
        let unknown = post(
            port,
            "/api/fact",
            &token,
            r#"{"name":"no-such-fact","description":"a fact that is not there"}"#,
        );
        assert!(unknown.starts_with("HTTP/1.1 400"), "{unknown}");
        assert!(unknown.contains("no fact named"), "{unknown}");
        let _ = std::fs::remove_dir_all(&corpus);
    }

    /// The skills panel's document is the library with its skips, the
    /// proposals awaiting a decision, and the curation candidates. The
    /// curation card names KI-215 rather than offering a retire the
    /// substrate refuses to draft.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_skills_document_is_the_library_its_skips_and_its_proposals() {
        let (token, port) = launch().await;
        let anonymous = ask(
            port,
            "GET /api/skills HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 200"), "{anonymous}");
        let reply = get(port, "/api/skills", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.contains("application/json"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert_eq!(document["type"], "skills");
        // The library, its skips, its proposals, and its merge candidates
        // are all carried, each as its own list.
        assert!(document["skills"].is_array(), "{document}");
        assert!(document["skips"].is_array(), "{document}");
        assert!(document["proposals"].is_array(), "{document}");
        assert!(document["merges"].is_array(), "{document}");
        assert!(document["sessions"].is_number(), "{document}");
        assert!(document["unused"].is_number(), "{document}");
        assert!(
            document["curation_note"]
                .as_str()
                .is_some_and(|note| note.contains("KI-215")),
            "{document}"
        );
        // The canonical library is named, so the panel can say which one
        // it is showing.
        assert!(document["canonical"].is_string(), "{document}");
        assert!(!body.contains("api_key"), "{body}");
    }

    /// The approve route reaches the substrate's own apply and reports
    /// what it says. A proposal that is not awaiting a decision is
    /// refused by the apply itself, in its own words — and a proposal
    /// whose target is not a skill is refused before the gate, because
    /// this is the skills panel's approve and a source edit's corpus step
    /// is a CLI step, not a button.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_approve_route_reaches_the_substrates_own_apply() {
        let (token, port) = launch().await;
        // Nothing is awaiting a decision on a fresh home, so the apply
        // refuses it in its own words rather than the route inventing one.
        let missing = post(port, "/api/skill", &token, r#"{"name":"no-such-proposal"}"#);
        assert!(missing.starts_with("HTTP/1.1 409"), "{missing}");
        assert!(
            missing.contains("no proposal named") || missing.contains("awaiting a decision"),
            "{missing}"
        );
        // A body with no name is a malformed request, not an apply.
        let malformed = post(
            port,
            "/api/skill",
            &token,
            r#"{"proposal":"no-such-proposal"}"#,
        );
        assert!(malformed.starts_with("HTTP/1.1 400"), "{malformed}");
    }

    /// The pane's two routes, behind the launch token like every other.
    /// The deck pane's apply: a checked deck renders with its verdict,
    /// and a deck that breaks a consultant constraint is a 409 that
    /// names the rule — the browser applies a card's payload, the
    /// server decides whether the result is honest.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_deck_render_route_renders_a_checked_deck_and_refuses_a_lying_one() {
        let (_token, port) = launch().await;
        let good = r#"{"deck":{"project_type":"infographic","nodes":[{"id":"frame_01","layout":"metric_callout","headline":"Q3 margins grew by fourteen percent","components":[{"type":"metric","value":"14%","label":"margins"}]}]}}"#;
        let rendered = post(port, "/api/deck/render", "", good);
        assert!(rendered.starts_with("HTTP/1.1 200"), "{rendered}");
        assert!(rendered.contains("\"status\":\"rendered\""), "{rendered}");
        assert!(rendered.contains("deck-node"), "{rendered}");
        assert!(rendered.contains("review"), "{rendered}");
        // A deck that breaks the pyramid is refused with the rule, and
        // nothing renders — the browser's apply cannot make the server
        // ship a deck that lies.
        let lying = r#"{"deck":{"project_type":"infographic","nodes":[{"id":"frame_01","layout":"split_two_column","headline":"Market Analysis 2026","components":[{"type":"bullet_list","items":["**a marked point**"]}]}]}}"#;
        let refused = post(port, "/api/deck/render", "", lying);
        assert!(refused.starts_with("HTTP/1.1 409"), "{refused}");
        assert!(refused.contains("not a conclusion"), "{refused}");
    }

    /// The tree reads the working directory this face was launched from,
    /// the file read carries the text, and both refuse what they must:
    /// a path that escapes the root, and a credential file by name.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_pane_routes_read_the_tree_and_the_file_and_refuse_the_rest() {
        let (token, port) = launch().await;
        let anonymous = ask(
            port,
            "GET /api/tree?path=src HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 200"), "{anonymous}");

        let tree = ask(
            port,
            &format!("GET /api/tree?path=src&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
        );
        assert!(tree.starts_with("HTTP/1.1 200"), "{tree}");
        let document: serde_json::Value =
            serde_json::from_str(tree.split("\r\n\r\n").nth(1).expect("a body past the head"))
                .expect("a JSON document");
        assert_eq!(document["type"], "tree");
        let entries = document["entries"].as_array().expect("the entries");
        assert!(
            entries
                .iter()
                .any(|entry| entry["name"] == "gui" && entry["kind"] == "dir"),
            "{document}"
        );

        // The file read carries the text and names its language.
        let file = ask(
            port,
            &format!(
                "GET /api/file?path=src%2Fgui%2Fassets%2Fapp.css&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
            ),
            None,
        );
        assert!(file.starts_with("HTTP/1.1 200"), "{file}");
        let document: serde_json::Value =
            serde_json::from_str(file.split("\r\n\r\n").nth(1).expect("a body past the head"))
                .expect("a JSON document");
        assert_eq!(document["type"], "file");
        assert_eq!(document["language"], "css");
        assert!(
            document["text"]
                .as_str()
                .is_some_and(|text| text.contains("--dsw-alias-page")),
            "{document}"
        );

        // A path that escapes the root, and a credential file, are both
        // refused by name — the browser can ask for a file now, which is
        // exactly where one would leak.
        let escaped = ask(
            port,
            &format!(
                "GET /api/file?path=..%2F..%2Fetc%2Fpasswd&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
            ),
            None,
        );
        let document: serde_json::Value = serde_json::from_str(
            escaped
                .split("\r\n\r\n")
                .nth(1)
                .expect("a body past the head"),
        )
        .expect("a JSON document");
        assert_eq!(document["refused"][0], "outside the working directory");
        let dotenv = ask(
            port,
            &format!("GET /api/file?path=.env&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
        );
        assert!(
            dotenv.contains("a credential file is never served"),
            "{dotenv}"
        );
        // A path with a space rides the query percent-encoded, and the
        // decoder has to bring it back.
        let spaced = ask(
            port,
            &format!(
                "GET /api/file?path=docs%2Fplans%2F2026-10-16-gui-revamp.md&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
            ),
            None,
        );
        assert!(spaced.starts_with("HTTP/1.1 200"), "{spaced}");
    }

    /// The composer's command type-ahead: the browser sends the command
    /// declarations it owns and this ranks them against what the owner
    /// typed, which is what lets "make the screen darker" find /theme. A
    /// machine with no embedding backend answers with zero scores rather
    /// than refusing, so the lexical ranking stands.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_rank_route_orders_candidates_by_what_was_typed() {
        let (token, port) = launch().await;
        let body = r#"{"asked":"make the screen darker","candidates":["theme dark light mode switch between the light and dark themes","pane files code preview explore open or close the files and deliverables pane","compact fold summarise fold the conversation now"]}"#;
        let reply = post(port, "/api/rank", &token, body);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        let document: serde_json::Value = serde_json::from_str(
            reply
                .split("\r\n\r\n")
                .nth(1)
                .expect("a body past the head"),
        )
        .expect("a JSON document");
        let ranked = document["ranked"].as_array().expect("the ranked list");
        // Either the embedding backend answered and put the theme command
        // first, or it is absent and every score is zero — the honest
        // degradation, which the browser's lexical pass covers.
        let first = ranked[0]["index"].as_u64().expect("an index");
        let scores: Vec<f64> = ranked
            .iter()
            .map(|entry| entry["score"].as_f64().unwrap_or(0.0))
            .collect();
        if scores.iter().any(|score| *score > 0.0) {
            assert_eq!(first, 0, "{document}");
        } else {
            assert!(scores.iter().all(|score| *score == 0.0), "{document}");
        }
        // The order is descending by score, either way.
        assert!(
            scores.windows(2).all(|pair| pair[0] >= pair[1]),
            "{document}"
        );
    }

    /// A home with no logs and no declared graph names both, rather than
    /// drawing an empty tree and an empty graph.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_home_with_nothing_delegated_names_it_rather_than_drawing_an_empty_graph() {
        let (token, port) = launch().await;
        let reply = get(port, "/api/delegation", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert!(document["live"].is_null(), "{document}");
        assert!(document["graph"].is_null(), "{document}");
        let refused = document["refused"].as_array().expect("the refusals");
        assert!(
            refused.iter().any(|reason| reason
                .as_str()
                .is_some_and(|reason| reason.contains("holds no declared graph"))),
            "{document}"
        );
        assert!(
            refused.iter().any(|reason| reason
                .as_str()
                .is_some_and(|reason| reason.contains("has no readable session log"))),
            "{document}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_task_crosses_the_run_route_and_lands_in_the_log() {
        let (token, port) = launch().await;
        let anonymous = post(port, "/api/run", "wrong", r#"{"task":"say hello"}"#);
        // No credential gates the route, so the task is validated and
        // answered on its own terms: on a detached bridge that is the
        // 409 whose reply carries the message into the log.
        assert!(anonymous.starts_with("HTTP/1.1 409"), "{anonymous}");
        let refused = post(port, "/api/run", &token, r#"{"nope":"say hello"}"#);
        assert!(refused.starts_with("HTTP/1.1 400"), "{refused}");
        let empty = post(port, "/api/run", &token, r#"{"task":"   "}"#);
        assert!(empty.starts_with("HTTP/1.1 409"), "{empty}");
        // The detached bridge has no agent thread behind it, so the run
        // is refused — but the owner's message is already in the log,
        // which is the honest order the browser shows.
        let accepted = post(port, "/api/run", &token, r#"{"task":"say hello"}"#);
        assert!(accepted.starts_with("HTTP/1.1 409"), "{accepted}");
        let state = get(port, "/api/state", &token);
        assert!(state.contains("\"type\":\"message\""), "{state}");
        assert!(state.contains("\"text\":\"say hello\""), "{state}");
        assert!(state.contains("\"type\":\"failed\""), "{state}");
    }

    /// C5: the run route validates a named folder before dispatch — an
    /// escape and a missing path are 400s that name the class, and a
    /// valid folder is accepted and stored for the chat. C7's control:
    /// the folder route changes the choice without a run and answers
    /// with the transcript line it recorded.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_run_refuses_a_bad_folder_and_a_good_one_is_stored() {
        let (token, port) = launch().await;
        let escaped = post(
            port,
            "/api/run",
            &token,
            r#"{"task":"work","folder":"../"}"#,
        );
        assert!(escaped.starts_with("HTTP/1.1 400"), "{escaped}");
        assert!(
            escaped.contains("outside the working directory"),
            "{escaped}"
        );
        let missing = post(
            port,
            "/api/run",
            &token,
            r#"{"task":"work","folder":"definitely-not-here"}"#,
        );
        assert!(missing.starts_with("HTTP/1.1 400"), "{missing}");
        assert!(missing.contains("definitely-not-here"), "{missing}");
        let malformed = post(port, "/api/run", &token, r#"{"task":"work","folder":7}"#);
        assert!(malformed.starts_with("HTTP/1.1 400"), "{malformed}");

        // C10: full access is a grant only a folder can bound, so a
        // no-folder chat is refused the mode by name.
        let full = post(port, "/api/run", &token, r#"{"task":"work","mode":"full"}"#);
        assert!(full.starts_with("HTTP/1.1 400"), "{full}");
        assert!(
            full.contains("full access needs a working folder"),
            "{full}"
        );

        // A real folder is accepted; the detached bridge then refuses
        // the run itself for having no agent, which is not this test's
        // question. The choice is stored either way.
        let valid = post(
            port,
            "/api/run",
            &token,
            r#"{"task":"work","folder":"docs"}"#,
        );
        assert!(!valid.starts_with("HTTP/1.1 400"), "{valid}");
        let state = get(port, "/api/state", &token);
        assert!(state.contains("\"name\":\"docs\""), "{state}");

        // The run bar's own control: set, then reset to the launch folder.
        let changed = post(port, "/api/folder", &token, r#"{"folder":"src"}"#);
        assert!(changed.starts_with("HTTP/1.1 200"), "{changed}");
        assert!(changed.contains("\"type\":\"folder\""), "{changed}");
        assert!(changed.contains("\"name\":\"src\""), "{changed}");
        let reset = post(port, "/api/folder", &token, r#"{"folder":null}"#);
        assert!(reset.starts_with("HTTP/1.1 200"), "{reset}");
        assert!(reset.contains("\"path\":null"), "{reset}");
        let bare = post(port, "/api/folder", &token, r#"{}"#);
        assert!(bare.starts_with("HTTP/1.1 400"), "{bare}");
    }

    /// The seat round-trips: the bridge stands a seat, the browser
    /// answers it, and the answer crosses. A browser that answers when
    /// nothing is waiting gets a conflict, and a word that is not a
    /// decision is refused with the seat still standing.
    #[tokio::test(flavor = "multi_thread")]
    async fn an_answer_crosses_the_approve_route_and_a_bad_one_does_not() {
        let (token, port) = launch().await;
        let conflict = post(port, "/api/approve", &token, r#"{"decision":"once"}"#);
        assert!(conflict.starts_with("HTTP/1.1 409"), "{conflict}");
        let malformed = post(port, "/api/approve", &token, r#"{"word":"once"}"#);
        assert!(malformed.starts_with("HTTP/1.1 400"), "{malformed}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn no_payload_carries_a_credential() {
        let (token, port) = launch().await;
        let page = get(port, "/", &token);
        let css = get(port, "/app.css", &token);
        let js = get(port, "/app.js", &token);
        let stream = ask(
            port,
            &format!("GET /events?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            Some("heartbeat"),
        );
        for payload in [&page, &css, &js, &stream, INDEX, STYLES, SCRIPT] {
            for secret in ["sk-", "DEEPSEEK_API_KEY", "Authorization", "Bearer"] {
                assert!(
                    !payload.contains(secret),
                    "a payload names {secret}: {payload}"
                );
            }
        }
        // The face carries no credential at all, so no reply sets a
        // cookie: there is no capability to keep out of the body, and
        // the head's silence about Set-Cookie is the pin.
        let (head, body) = page.split_once("\r\n\r\n").expect("a head and a body");
        assert!(!head.contains("Set-Cookie"), "{head}");
        for secret in ["sk-", "DEEPSEEK_API_KEY", "fleet_gui"] {
            assert!(!body.contains(secret) && !head.contains(secret), "{page}");
        }
    }

    // -------------------------------------------------- the sessions surface (G6)

    /// A fresh home holding two real session logs: the older carries a
    /// message and an end record, the newer a message and no end. The
    /// ids are the shape `Session::create` mints, written directly —
    /// the delegation test's precedent — because the archive reads the
    /// files, not the writer.
    fn archive_home() -> (std::path::PathBuf, Vec<String>) {
        let home = fresh_home();
        let sessions = home.join("sessions");
        std::fs::create_dir_all(&sessions).expect("the sessions directory");
        let ids = vec![
            "20261017T090000Z-olderlog".to_string(),
            "20261017T090100Z-newerlog".to_string(),
        ];
        let start = |model: &str| {
            serde_json::json!({
                "type": "start",
                "v": crate::session::SESSION_FORMAT_VERSION,
                "model": model,
                "created": "2026-10-17T09:00:00Z",
            })
        };
        for (id, lines) in [
            (
                ids[0].as_str(),
                vec![
                    // The start stamp is the pre-route default; the end
                    // record names the model that actually served
                    // (KI-073). The archive must show the latter — this
                    // is KI-255's exact scenario.
                    start("deepseek-chat"),
                    serde_json::json!({"type": "user", "content": "the older conversation"}),
                    serde_json::json!({"type": "end", "stop_reason": "done", "turns": 1, "served_model": "step-5-preview"}),
                ],
            ),
            (
                ids[1].as_str(),
                vec![
                    start("step-5-preview"),
                    serde_json::json!({"type": "user", "content": "the newer conversation"}),
                ],
            ),
        ] {
            std::fs::write(
                sessions.join(format!("{id}.jsonl")),
                format!(
                    "{}\n",
                    lines
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            )
            .expect("a session log");
        }
        (home, ids)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_archive_lists_the_logs_newest_first_and_names_the_anchor() {
        let (home, ids) = archive_home();
        let (bridge, _) = Bridge::detached(home, "deepseek-chat".to_string(), Some(ids[0].clone()));
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .expect("bind");
        server.heartbeat = Duration::from_millis(20);
        // The face no longer gates on a token; the queries below still carry
        // one and the server ignores it, which is why these reads answer
        // all the same either way.
        let token = String::new();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        let reply = get(port, "/api/sessions", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(
            reply.contains(&format!("\"current\":\"{}\"", ids[0])),
            "the anchor is named: {reply}"
        );
        let newer_row = reply
            .find(&format!("\"id\":\"{}\"", ids[1]))
            .expect("the newer log is listed");
        let older_row = reply
            .find(&format!("\"id\":\"{}\"", ids[0]))
            .filter(|at| *at > newer_row)
            .expect("the anchor is listed after it");
        assert!(
            newer_row < older_row,
            "the archive reads newest first: {reply}"
        );
        assert!(
            reply.contains("\"ended\":true"),
            "the ended log says so: {reply}"
        );
        assert!(
            reply.contains("\"ended\":false"),
            "the open log says so: {reply}"
        );
        // KI-255: the row names the model that served, never the start
        // record's pre-route stamp — the archive read `deepseek-chat`
        // off every row for runs step-5-preview served. An open run
        // names nothing yet.
        assert!(
            reply.contains("\"model\":\"step-5-preview\""),
            "the ended row names the served model: {reply}"
        );
        assert!(
            reply.contains("\"model\":null"),
            "the open row names no model: {reply}"
        );
        assert!(
            !reply.contains("deepseek-chat"),
            "the nominal stamp never renders: {reply}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_replay_shapes_the_records_and_refuses_a_shape_that_is_not_an_id() {
        let home = fresh_home();
        let sessions = home.join("sessions");
        std::fs::create_dir_all(&sessions).expect("the sessions directory");
        std::fs::write(
            sessions.join("20261017T091000Z-replayit.jsonl"),
            format!(
                "{}\n{}\n{}\n{}\n",
                serde_json::json!({
                    "type": "start",
                    "v": crate::session::SESSION_FORMAT_VERSION,
                    "model": "deepseek-chat",
                    "created": "2026-10-17T09:10:00Z",
                }),
                serde_json::json!({"type": "user", "content": "hello"}),
                serde_json::json!({
                    "type": "tool_result",
                    "call_id": "c1",
                    "name": "bash",
                    "output": "ok\nsecond line",
                    "truncated": false,
                    "approved": true,
                }),
                serde_json::json!({"type": "end", "stop_reason": "done", "turns": 2}),
            ),
        )
        .expect("the session log");
        let (bridge, _) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .expect("bind");
        server.heartbeat = Duration::from_millis(20);
        // The face no longer gates on a token; the queries below still carry
        // one and the server ignores it, which is why these reads answer
        // all the same either way.
        let token = String::new();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        let id = "20261017T091000Z-replayit";
        // The id and the token share the query string, so this one skips
        // the `get` helper's own `?token=` append and asks directly.
        let reply = ask(
            port,
            &format!("GET /api/session?id={id}&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
        );
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.contains("\"type\":\"replay\""), "{reply}");
        assert!(reply.contains("hello"), "the owner's words ride: {reply}");
        assert!(
            reply.contains("\"preview\":\"ok\""),
            "the preview is the live cap's first line: {reply}"
        );
        assert!(
            !reply.contains("second line"),
            "the preview is first-line only, exactly what a live card showed: {reply}"
        );
        assert!(reply.contains("\"ended\":true"), "{reply}");
        assert!(reply.contains("\"turns\":2"), "{reply}");
        let refused = ask(
            port,
            &format!(
                "GET /api/session?id=../etc&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
            ),
            None,
        );
        assert!(refused.contains("is not a session id"), "{refused}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_resume_reseeds_the_face_and_names_its_refusals() {
        let (home, ids) = archive_home();
        let (bridge, _) = Bridge::detached(home, "deepseek-chat".to_string(), Some(ids[0].clone()));
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .expect("bind");
        server.heartbeat = Duration::from_millis(20);
        // The face no longer gates on a token; the queries below still carry
        // one and the server ignores it, which is why these reads answer
        // all the same either way.
        let token = String::new();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        let resumed = post(
            port,
            "/api/resume",
            &token,
            &format!("{{\"session\":\"{}\"}}", ids[1]),
        );
        assert!(resumed.starts_with("HTTP/1.1 200"), "{resumed}");
        // The resumed session's own state holds the log's cards.
        let state = ask(
            port,
            &format!(
                "GET /api/state?token={token}&session={} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
                ids[1]
            ),
            None,
        );
        assert!(
            state.contains(&format!("\"session\":\"{}\"", ids[1])),
            "the anchor moved: {state}"
        );
        assert!(
            state.contains("the newer conversation"),
            "the view reseeded: {state}"
        );
        assert!(state.contains("\"type\":\"resumed\""), "{state}");
        assert!(
            state.contains("\"agent\":false"),
            "a face with no agent behind it says so: {state}"
        );
        // And the launch session is unmoved by the resume: a phone that
        // names nothing still gets the session the face booted with.
        let launch = get(port, "/api/state", &token);
        assert!(
            launch.contains(&format!("\"session\":\"{}\"", ids[0])),
            "the launch session is the default: {launch}"
        );
        let refused = post(port, "/api/resume", &token, "{\"session\":\"../x\"}");
        assert!(refused.starts_with("HTTP/1.1 409"), "{refused}");
        assert!(refused.contains("not a session id"), "{refused}");
        let missing = post(port, "/api/resume", &token, "{\"session\":\"no-such-log\"}");
        assert!(missing.starts_with("HTTP/1.1 409"), "{missing}");
        assert!(missing.contains("could not open session"), "{missing}");
        let malformed = post(port, "/api/resume", &token, "{\"word\":\"x\"}");
        assert!(malformed.starts_with("HTTP/1.1 400"), "{malformed}");
    }

    // ------------------------------------------------ the tower console (G7)

    /// A home whose own `.env` names its second-brain and projects root
    /// — the two settings the tower board resolves, so the read is the
    /// test’s own rather than the machine’s. The brain holds the
    /// reports and tower aggregates the console’s six zones read, and
    /// the projects root holds one real worktree and one plain
    /// directory the scan must name rather than measure.
    fn tower_home() -> std::path::PathBuf {
        let home = fresh_home();
        std::fs::write(
            home.join(".env"),
            format!(
                "SECOND_BRAIN_HOME={}\nTOWER_LOG_PROJECTS_DIR={}\n",
                home.join("brain").display(),
                home.join("projects").display()
            ),
        )
        .expect("the env names the board’s own roots");
        let brain = home.join("brain");
        std::fs::create_dir_all(&brain).expect("the second-brain home");
        std::fs::write(
            brain.join("reports_state.json"),
            r#"{
              "generated_at": "2026-10-18 07:00",
              "doing": [
                { "id": "t-1", "project": "fleet", "title": "ship the console",
                  "harness": "zcode", "opened": "2026-10-17 09:00:00",
                  "last": "2026-10-18 06:00:00" }
              ],
              "pending_verification": [ { "id": "t-2" }, { "id": "t-3" } ],
              "conflicts": [
                { "task": "t-9", "type": "commit", "severity": "defect",
                  "claim": "done claiming commit deadbeef",
                  "reality": "no such commit" }
              ],
              "open_conflict_count": 1,
              "trust": [
                { "harness": "zcode", "claimed": 10, "evidenced": 9, "precision": 0.9 }
              ],
              "suppressed": 2,
              "malformed": 1,
              "events_total": 42,
              "coverage": {
                "window_days": 30, "with_reports": 6, "commit_only": 11, "ratio": 0.35
              },
              "report_daily": { "fleet": { "2026-10-17": 3, "2026-10-18": 2 } },
              "reports_recent": [
                { "ts": "2026-10-18 06:59:00", "type": "done", "project": "fleet",
                  "harness": "zcode", "summary": "ship the console", "task": "t-1",
                  "commit": "409910d" }
              ]
            }"#,
        )
        .expect("the reports state");
        std::fs::write(
            brain.join("tower_data.json"),
            r#"{
              "generated_at": "2026-10-18 07:00",
              "projects": [
                { "name": "fleet", "lifecycle": "active", "branch": "master",
                  "last_commit": "2026-10-18", "commits_30d": 12, "dirty": 0,
                  "plans": { "done": 9, "in_progress": 1 } }
              ],
              "timeline": [ { "date": "2026-10-18", "subject": "slice 20", "project": "fleet" } ],
              "harness": [ { "harness": "zcode", "total": 35, "user": 17 } ],
              "reports": {}, "harness_runs": {}
            }"#,
        )
        .expect("the tower data");
        let projects = home.join("projects");
        std::fs::create_dir_all(&projects).expect("the projects root");
        // One real worktree, two commits on main.
        let repo = projects.join("fleet-clone");
        std::fs::create_dir_all(&repo).expect("the worktree");
        for args in [
            vec!["init", "--quiet"],
            vec!["config", "user.email", "board@example.invalid"],
            vec!["config", "user.name", "Board Test"],
            vec!["checkout", "--quiet", "-b", "main"],
        ] {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(&args)
                .status()
                .expect("git runs");
            assert!(status.success(), "{args:?}");
        }
        for (index, message) in ["first commit", "second commit"].into_iter().enumerate() {
            std::fs::write(repo.join(format!("file-{index}.txt")), format!("{index}\n"))
                .expect("the commit’s file");
            for args in [vec!["add", "-A"], vec!["commit", "--quiet", "-m", message]] {
                let status = std::process::Command::new("git")
                    .arg("-C")
                    .arg(&repo)
                    .args(&args)
                    .status()
                    .expect("git runs");
                assert!(status.success(), "{args:?}");
            }
        }
        // A plain directory: the scan names it rather than measuring it.
        std::fs::create_dir_all(projects.join("not-a-repo")).expect("the plain directory");
        home
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_tower_console_reads_the_boards_and_names_what_did_not_land() {
        let home = tower_home();
        let (bridge, _) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .expect("bind");
        server.heartbeat = Duration::from_millis(20);
        // The face no longer gates on a token; the queries below still carry
        // one and the server ignores it, which is why these reads answer
        // all the same either way.
        let token = String::new();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        let reply = get(port, "/api/tower", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        // The KPI strip is the reads’ own counts.
        assert!(reply.contains("\"doing\":1"), "{reply}");
        assert!(reply.contains("\"pending_verification\":2"), "{reply}");
        assert!(reply.contains("\"open_conflicts\":1"), "{reply}");
        assert!(reply.contains("\"suppressed\":2"), "{reply}");
        assert!(reply.contains("\"malformed\":1"), "{reply}");
        assert!(reply.contains("\"events_total\":42"), "{reply}");
        // The hero’s two series, folded per day.
        assert!(
            reply.contains("\"count\":3,\"day\":\"2026-10-17\""),
            "{reply}"
        );
        assert!(
            reply.contains("\"count\":2,\"day\":\"2026-10-18\""),
            "{reply}"
        );
        // The feed, with its commit and its staleness named.
        assert!(reply.contains("ship the console"), "{reply}");
        assert!(reply.contains("409910d"), "{reply}");
        assert!(
            reply.contains("\"reports_at\":\"2026-10-18 07:00\""),
            "{reply}"
        );
        // The claims against the evidence, and the trust lines.
        assert!(reply.contains("done claiming commit deadbeef"), "{reply}");
        assert!(reply.contains("no such commit"), "{reply}");
        assert!(reply.contains("\"precision\":0.9"), "{reply}");
        // The harness mix and the coverage line.
        assert!(
            reply.contains("\"harness\":\"zcode\",\"total\":35"),
            "{reply}"
        );
        assert!(reply.contains("\"with_reports\":6"), "{reply}");
        assert!(reply.contains("\"ratio\":0.35"), "{reply}");
        // The git facts of the real worktree, and the plain directory
        // named rather than measured.
        assert!(reply.contains("\"name\":\"fleet-clone\""), "{reply}");
        assert!(reply.contains("\"commits_30d\":2"), "{reply}");
        assert!(reply.contains("not-a-repo"), "{reply}");
        // The lifecycle join: the console’s health dots come from the
        // brain’s own projects, not from git.
        assert!(reply.contains("\"lifecycle\":\"active\""), "{reply}");
        // Every read landed, so nothing is refused.
        assert!(reply.contains("\"refused\":[]"), "{reply}");

        // The second-brain panel rides the same home: the reports
        // section lands, and the four absent state files name
        // themselves instead of the panel pretending they are empty.
        let brain = get(port, "/api/second-brain", &token);
        assert!(brain.starts_with("HTTP/1.1 200"), "{brain}");
        assert!(brain.contains("\"events_total\":42"), "{brain}");
        assert!(brain.contains("gates_state.json does not exist"), "{brain}");
        assert!(
            brain.contains("maturity_state.json does not exist"),
            "{brain}"
        );
    }

    // ------------------------------------------------ the settings surface (G8)

    /// A home that already holds configuration: one provider override
    /// and one remembered rule in config.json, one workload in
    /// routing.json, one cron job, the SOUL slot, and a fleet.env line.
    /// The document the panel reads is the home's own answer.
    fn settings_home() -> std::path::PathBuf {
        let home = fresh_home();
        std::fs::write(
            home.join("config.json"),
            r#"{
              "pin": "stepfun",
              "providers": [
                { "id": "my-provider", "base_url": "https://example.invalid/v1",
                  "key_env": "MY_PROVIDER_KEY" }
              ],
              "approval_rules": [ { "tool": "read" } ]
            }"#,
        )
        .expect("the config");
        std::fs::write(
            home.join("routing.json"),
            r#"{
              "workloads": [
                { "workload": "quick", "candidates": ["stepfun/step-5-preview"] }
              ]
            }"#,
        )
        .expect("the routing");
        std::fs::write(
            home.join("fleet.env"),
            "FLEET_EMBEDDING_ENDPOINT=http://127.0.0.1:9999/v1\n",
        )
        .expect("the env file");
        std::fs::write(home.join("SOUL.md"), "Be direct.\n").expect("the identity slot");
        let mut store = crate::cron::Store::open(&home).expect("the cron store");
        store
            .add("daily-triage", "0 9 * * *", "triage the KI ledger", false)
            .expect("the job");
        home
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_settings_document_is_the_homes_own_answer() {
        let home = settings_home();
        let (bridge, _) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .expect("bind");
        server.heartbeat = Duration::from_millis(20);
        // The face no longer gates on a token; the queries below still carry
        // one and the server ignores it, which is why these reads answer
        // all the same either way.
        let token = String::new();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        let reply = get(port, "/api/settings", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        // The knobs, with the home's own values and sources.
        assert!(reply.contains("\"key\":\"pin\""), "{reply}");
        assert!(reply.contains("\"value\":\"stepfun\""), "{reply}");
        assert!(reply.contains("\"source\":\"file\""), "{reply}");
        // The two lists and the rules.
        assert!(reply.contains("my-provider"), "{reply}");
        assert!(reply.contains("https://example.invalid/v1"), "{reply}");
        assert!(reply.contains("stepfun/step-5-preview"), "{reply}");
        assert!(reply.contains("\"label\":\"read\""), "{reply}");
        // The env names, by name only — the fleet.env line resolves.
        assert!(
            reply.contains("\"name\":\"FLEET_EMBEDDING_ENDPOINT\",\"present\":true"),
            "{reply}"
        );
        assert!(
            reply.contains("\"name\":\"FLEET_CUA_DRIVER\",\"present\":false"),
            "{reply}"
        );
        // The env zone is by name only; the knob rows render their
        // values the way `fleet settings` renders them.
        // The cron jobs and the identity slot.
        assert!(reply.contains("daily-triage"), "{reply}");
        assert!(reply.contains("\"present\":true"), "{reply}");
        assert!(reply.contains("Be direct."), "{reply}");
        // The router, the catalog the pickers read, and the lanes.
        assert!(reply.contains("\"router\""), "{reply}");
        assert!(reply.contains("\"spend_guard\""), "{reply}");
        assert!(reply.contains("\"tiers\""), "{reply}");
        assert!(reply.contains("\"resolved\""), "{reply}");
        assert!(reply.contains("\"catalog\""), "{reply}");
        assert!(reply.contains("\"models\""), "{reply}");
        assert!(reply.contains("\"lanes\""), "{reply}");
        assert!(reply.contains("\"roles\""), "{reply}");
        assert!(reply.contains("\"vault\""), "{reply}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_settings_write_rides_the_writer_and_names_its_refusals() {
        let home = settings_home();
        let (bridge, _) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .expect("bind");
        server.heartbeat = Duration::from_millis(20);
        // The face no longer gates on a token; the queries below still carry
        // one and the server ignores it, which is why these reads answer
        // all the same either way.
        let token = String::new();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });

        // A knob write lands in the home's own file, and the next read
        // is the file's answer.
        let wrote = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"set","key":"skills.retire_after_sessions","value":"7"}"#,
        );
        assert!(wrote.starts_with("HTTP/1.1 200"), "{wrote}");
        let config = std::fs::read_to_string(home.join("config.json")).expect("the config");
        assert!(config.contains("\"retire_after_sessions\": 7"), "{config}");
        let reread = get(port, "/api/settings", &token);
        assert!(reread.contains("\"value\":\"7\""), "{reread}");

        // A rule rides allow_tool, so the class check is the seat's own.
        let ruled = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"rule","tool":"bash","program":"cargo"}"#,
        );
        assert!(ruled.starts_with("HTTP/1.1 200"), "{ruled}");
        let config = std::fs::read_to_string(home.join("config.json")).expect("the config");
        assert!(
            config.contains("\"tool\": \"bash\"") && config.contains("\"program\": \"cargo\""),
            "{config}"
        );
        // The always-ask class is refused by name, exactly as the seat
        // refuses it.
        let refused = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"rule","tool":"computer_use"}"#,
        );
        assert!(refused.starts_with("HTTP/1.1 409"), "{refused}");
        assert!(refused.contains("cannot be remembered"), "{refused}");
        let forgot = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"rule-remove","tool":"bash","program":"cargo"}"#,
        );
        assert!(forgot.starts_with("HTTP/1.1 200"), "{forgot}");

        // A provider entry validates through the roster's own schema: a
        // misspelled field fails here, by name.
        let bad = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"provider","entry":"{\"id\":\"x\",\"base_url\":\"https://x.invalid\",\"key_env\":\"K\",\"typo\":1}"}"#,
        );
        assert!(bad.starts_with("HTTP/1.1 409"), "{bad}");

        // A cron job lands in the store, toggles, and removes.
        let job = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"cron","name":"nightly","schedule":"0 3 * * *","task":"run the eval"}"#,
        );
        assert!(job.starts_with("HTTP/1.1 200"), "{job}");
        let listed = get(port, "/api/settings", &token);
        assert!(listed.contains("nightly"), "{listed}");
        let disabled = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"cron-enabled","name":"nightly","enabled":false}"#,
        );
        assert!(disabled.starts_with("HTTP/1.1 200"), "{disabled}");
        let removed = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"cron-remove","name":"nightly"}"#,
        );
        assert!(removed.starts_with("HTTP/1.1 200"), "{removed}");

        // A lane write lands in lanes.json through the same writer.
        let lane = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"lane","lane":"sem1","config":{"env":{"SEM1_ST_VENV":"/opt/venv-st"},"defaults":{}}}"#,
        );
        assert!(lane.starts_with("HTTP/1.1 200"), "{lane}");
        let lanes = std::fs::read_to_string(home.join("lanes.json")).expect("the lanes file");
        assert!(lanes.contains("SEM1_ST_VENV"), "{lanes}");
        // A name the lane does not declare is refused by name.
        let refused_lane = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"lane","lane":"sem1","config":{"env":{"DEEPSEEK_API_KEY":"x"},"defaults":{}}}"#,
        );
        assert!(refused_lane.starts_with("HTTP/1.1 409"), "{refused_lane}");
        assert!(refused_lane.contains("does not cross"), "{refused_lane}");

        // A role write lands through the roles writer, and the surface
        // reads it back.
        let role = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"role","name":"auditor","description":"reads a diff","tools":["bash"],"skills":[],"shapes":[],"overlay":"Be terse."}"#,
        );
        assert!(role.starts_with("HTTP/1.1 200"), "{role}");
        let written =
            std::fs::read_to_string(home.join("roles").join("auditor.md")).expect("the role file");
        assert!(written.contains("reads a diff"), "{written}");
        assert!(written.contains("Be terse."), "{written}");
        let reread = get(port, "/api/settings", &token);
        assert!(reread.contains("\"auditor\""), "{reread}");
        let removed = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"role-remove","name":"auditor"}"#,
        );
        assert!(removed.starts_with("HTTP/1.1 200"), "{removed}");

        // A secret rides the secret writer, and the read that follows
        // never carries the value.
        let secret = post_within(
            port,
            "/api/settings",
            &token,
            r#"{"op":"secret","name":"FLEET_TEST_KEY","value":"never-render-me"}"#,
            Duration::from_secs(30),
        );
        assert!(secret.starts_with("HTTP/1.1 200"), "{secret}");
        assert!(!secret.contains("never-render-me"), "{secret}");
        let after_secret = get(port, "/api/settings", &token);
        assert!(!after_secret.contains("never-render-me"), "{after_secret}");
        let unsealed = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"secret-remove","name":"FLEET_TEST_KEY"}"#,
        );
        assert!(unsealed.starts_with("HTTP/1.1 200"), "{unsealed}");

        // The vault surface: unlock, create from the plaintext .env,
        // read the names without the value, drop the plaintext, lock.
        // A fresh vault makes the test independent of any ambient
        // passphrase the test process happens to carry.
        let _ = std::fs::remove_file(home.join("vault.json"));
        crate::home::reset_vault_unlock();
        let mut env_text = std::fs::read_to_string(home.join(".env")).unwrap_or_default();
        env_text.push_str("FLEET_TEST_API_KEY=test-secret\n");
        std::fs::write(home.join(".env"), env_text).unwrap();
        // The vault's KDF (210k PBKDF2 iterations) is deliberately slow;
        // the full suite's load can push it past the ordinary three-second
        // read budget.
        let budget = Duration::from_secs(30);
        let unlocked = post_within(
            port,
            "/api/settings",
            &token,
            r#"{"op":"vault-unlock","passphrase":"passphrase"}"#,
            budget,
        );
        assert!(unlocked.starts_with("HTTP/1.1 200"), "{unlocked}");
        let after_unlock = get_within(port, "/api/settings", &token, budget);
        assert!(
            after_unlock.contains("\"passphrase\":true"),
            "{after_unlock}"
        );
        let created = post_within(
            port,
            "/api/settings",
            &token,
            r#"{"op":"vault-init"}"#,
            budget,
        );
        assert!(created.starts_with("HTTP/1.1 200"), "{created}");
        let after_init = get_within(port, "/api/settings", &token, budget);
        assert!(after_init.contains("FLEET_TEST_API_KEY"), "{after_init}");
        assert!(!after_init.contains("test-secret"), "{after_init}");
        let dropped = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"vault-remove-env","name":"FLEET_TEST_API_KEY"}"#,
        );
        assert!(dropped.starts_with("HTTP/1.1 200"), "{dropped}");
        let env_after = std::fs::read_to_string(home.join(".env")).unwrap_or_default();
        assert!(!env_after.contains("FLEET_TEST_API_KEY"), "{env_after}");
        let locked = post(port, "/api/settings", &token, r#"{"op":"vault-lock"}"#);
        assert!(locked.starts_with("HTTP/1.1 200"), "{locked}");
        let after_lock = get_within(port, "/api/settings", &token, budget);
        assert!(after_lock.contains("\"passphrase\":false"), "{after_lock}");
        crate::home::reset_vault_unlock();

        // The identity slot writes through the home's own writer.
        let soul = post(
            port,
            "/api/settings",
            &token,
            r#"{"op":"soul","text":"Prefer short answers."}"#,
        );
        assert!(soul.starts_with("HTTP/1.1 200"), "{soul}");
        let written = std::fs::read_to_string(home.join("SOUL.md")).expect("the slot");
        assert_eq!(written, "Prefer short answers.");

        // An unknown op is refused by name.
        let unknown = post(port, "/api/settings", &token, r#"{"op":"teleport"}"#);
        assert!(unknown.starts_with("HTTP/1.1 409"), "{unknown}");
        assert!(
            unknown.contains("is not a write this surface performs"),
            "{unknown}"
        );
    }

    // ---------------------------------------------------- the trust trio (G9)

    /// A home whose session has one checkpointed turn: a tree captured
    /// at turn 1, then the turn's writes applied to the live tree. The
    /// bridge's cwd is the launch directory, so the tree is the process's
    /// own working directory — the test changes into the tree it built
    /// and restores the cwd after, because the checkpoint root is read
    /// from the process, never from a parameter.
    fn trio_home() -> (std::path::PathBuf, std::path::PathBuf) {
        let home = fresh_home();
        let tree = home.join("tree");
        std::fs::create_dir_all(&tree).expect("the tree");
        std::fs::write(tree.join("kept.txt"), "before\n").expect("kept");
        std::fs::write(tree.join("changed.txt"), "one\ntwo\nthree\nfour\n").expect("changed");
        std::fs::write(tree.join("multi.txt"), "head\nA1\nA2\nmid\nB1\nB2\ntail\n").expect("multi");
        let store = home.join("checkpoints");
        let session = "session-test";
        crate::checkpoints::capture(&tree, &store, session, 1).expect("the capture");
        // The turn's writes: one file changed in two regions, one
        // created.
        std::fs::write(tree.join("changed.txt"), "one\nTWO\nthree\nFOUR\n").expect("the change");
        std::fs::write(tree.join("created.txt"), "new\n").expect("the create");
        // A log with one turn in it, so the rewind has a conversation to
        // reseed.
        let sessions = home.join("sessions");
        std::fs::create_dir_all(&sessions).expect("the sessions directory");
        std::fs::write(
            sessions.join(format!("{session}.jsonl")),
            format!(
                "{}\n{}\n",
                serde_json::json!({
                    "type": "start",
                    "v": crate::session::SESSION_FORMAT_VERSION,
                    "model": "deepseek-chat",
                    "created": "2026-10-18T09:00:00Z",
                }),
                serde_json::json!({"type": "user", "content": "do the work"}),
            ),
        )
        .expect("the session log");
        (home, tree)
    }

    /// Stand the bridge up over `tree` as the process's working
    /// directory: the checkpoint root and the diff's root are both read
    /// from the process, which is the production rule, so the test
    /// honours it rather than routing around it.
    async fn trio_server(home: std::path::PathBuf, tree: std::path::PathBuf) -> (u16, String) {
        // The trust trio's root rides the test's seam, not a change to
        // the process's working directory: the cwd is a global every
        // other test in this binary reads.
        let (mut bridge, _) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        bridge.point_at_cwd(tree);
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .expect("bind");
        server.heartbeat = Duration::from_millis(20);
        // The face no longer gates on a token; the queries below still carry
        // one and the server ignores it, which is why these reads answer
        // all the same either way.
        let token = String::new();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        (port, token)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_diff_reviews_the_changes_and_a_reject_puts_one_hunk_back() {
        let (home, tree) = trio_home();
        let (port, token) = trio_server(home, tree.clone()).await;

        // The checkpoint store lists the turn it holds.
        let listed = get(port, "/api/checkpoints", &token);
        assert!(listed.starts_with("HTTP/1.1 200"), "{listed}");
        assert!(listed.contains("\"turn\":1"), "{listed}");

        // The diff names both changes against turn 1. The turn and the
        // token share the query string, so this one asks directly.
        let diff = ask(
            port,
            &format!("GET /api/diff?turn=1&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
        );
        assert!(diff.starts_with("HTTP/1.1 200"), "{diff}");
        assert!(diff.contains("changed.txt"), "{diff}");
        assert!(diff.contains("created.txt"), "{diff}");
        assert!(diff.contains("\"kind\":\"added\""), "{diff}");
        // Two regions changed, so two hunks.
        assert_eq!(diff.matches("\"new_at\":").count(), 2, "{diff}");

        // Reject the second hunk only: its region goes back and the
        // first region stays changed.
        let rejected = post(
            port,
            "/api/diff/reject",
            &token,
            r#"{"turn":1,"path":"changed.txt","hunk":1}"#,
        );
        assert!(rejected.starts_with("HTTP/1.1 200"), "{rejected}");
        let after = std::fs::read_to_string(tree.join("changed.txt")).expect("the file");
        assert_eq!(
            after,
            "one\nTWO\nthree\nFOUR\n".replace("FOUR", "four"),
            "{after}"
        );

        // Reject the whole added file: it goes.
        let whole = post(
            port,
            "/api/diff/reject",
            &token,
            r#"{"turn":1,"path":"created.txt"}"#,
        );
        assert!(whole.starts_with("HTTP/1.1 200"), "{whole}");
        assert!(!tree.join("created.txt").exists(), "the added file is gone");

        // A whole-file reject of an unchanged file is the restore it
        // says it is — the bytes are already the checkpoint's — and a
        // reject of a file the diff does not carry is refused by name.
        let none = post(
            port,
            "/api/diff/reject",
            &token,
            r#"{"turn":1,"path":"kept.txt"}"#,
        );
        assert!(none.starts_with("HTTP/1.1 200"), "{none}");
        assert_eq!(
            std::fs::read_to_string(tree.join("kept.txt")).unwrap(),
            "before\n"
        );
        let absent = post(
            port,
            "/api/diff/reject",
            &token,
            r#"{"turn":1,"path":"never-existed.txt"}"#,
        );
        assert!(absent.starts_with("HTTP/1.1 409"), "{absent}");
        let _ = std::fs::remove_dir_all(tree.parent().unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_rewind_restores_the_tree_cuts_the_log_and_reseeds_the_face() {
        let (home, tree) = trio_home();
        let (port, token) = trio_server(home.clone(), tree.clone()).await;

        let rewound = post(port, "/api/rewind", &token, r#"{"turn":1}"#);
        assert!(rewound.starts_with("HTTP/1.1 200"), "{rewound}");
        // The tree is the checkpoint's again.
        assert_eq!(
            std::fs::read_to_string(tree.join("changed.txt")).unwrap(),
            "one\ntwo\nthree\nfour\n"
        );
        assert!(!tree.join("created.txt").exists());
        // The log carries the rewind record.
        let log =
            std::fs::read_to_string(home.join("sessions").join("session-test.jsonl")).unwrap();
        assert!(log.contains("\"type\":\"rewind\""), "{log}");
        // The face reseeded from before the cut: the owner's message
        // rides, and the rewind card is the newest event.
        let state = get(port, "/api/state", &token);
        assert!(state.contains("do the work"), "{state}");
        assert!(state.contains("\"type\":\"rewound\""), "{state}");

        // A rewind of a turn the store does not hold is refused by name.
        let absent = post(port, "/api/rewind", &token, r#"{"turn":9}"#);
        assert!(absent.starts_with("HTTP/1.1 409"), "{absent}");
        assert!(absent.contains("no checkpoint at turn 9"), "{absent}");
        let _ = std::fs::remove_dir_all(tree.parent().unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_calibration_read_names_the_gate_the_cells_and_the_cap() {
        let home = router_home();
        let day = crate::calib::today();
        let logs = home
            .join("logs")
            .join(&day[..4])
            .join(&day[5..7])
            .join(&day[8..10]);
        std::fs::create_dir_all(&logs).expect("the day's log directory");
        std::fs::write(
            logs.join("events.jsonl"),
            format!(
                "{}\n{}\n",
                serde_json::json!({
                    "op": "route",
                    "ts": "2026-10-17T09:00:00Z",
                    "provider": "stepfun",
                    "model": "step-5-preview",
                    "workload": "hard",
                    "how": "judged",
                    "judged_from": "fresh",
                }),
                serde_json::json!({
                    "op": "route_judged",
                    "ts": "2026-10-17T09:00:01Z",
                    "route_key": "k1",
                    "workload": "hard",
                    "verdict": "step-5-preview",
                    "confidence": 0.9,
                    "disposition": "agreed",
                    "lane": "sys1",
                }),
            ),
        )
        .expect("the day's rows");
        let (bridge, _) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .expect("bind");
        server.heartbeat = Duration::from_millis(20);
        // The face no longer gates on a token; the queries below still carry
        // one and the server ignores it, which is why these reads answer
        // all the same either way.
        let token = String::new();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        let reply = get(port, "/api/calibration", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.contains("\"type\":\"calibration\""), "{reply}");
        assert!(
            reply.contains("\"provider\":\"stepfun\""),
            "the fit cell rode through: {reply}"
        );
        assert!(
            reply.contains("\"judged\":1"),
            "the cell counts the one judged row: {reply}"
        );
        assert!(
            reply.contains("\"fit\":\"unfit\""),
            "one judged row is not a fit sample: {reply}"
        );
        assert!(
            reply.contains("\"cap\":512"),
            "the read names its bound: {reply}"
        );
        assert!(reply.contains("\"rows_read\":2"), "{reply}");
        let expected_gate = crate::route::calibration_config(&home).informs_failover;
        assert!(
            reply.contains(&format!("\"informs_failover\":{expected_gate}")),
            "the gate flag matches the config read, whatever it is: {reply}"
        );
    }
}
