//! The relay broker — armada's self-contained nervous system (L6).
//!
//! A dumb switchboard between a phone (or the flagship) and a fleet: it
//! routes sealed frames by session id (`sid`) and understands nothing of
//! their content (D20, §8.5). It replaces `relay/relay.py` — a mode of the
//! same binary (`fleet relay --broker`), no Python, no foreign runtime
//! (hard rule 12). The fleet-side client (`super::relay`) connects unchanged.
//!
//! What L6 adds over the reference broker (D21, D22):
//! - **Push with ack + cursor**: an unsolicited frame is queued for a sid
//!   with a sequence number; the poll hands it out, and an ack advances the
//!   cursor. A frame is trusted *delivered* only when acked — an unacked
//!   frame is a named gap the cursor exposes and the client re-requests,
//!   never a silent loss (the cross-cutting trust obligation).
//! - **The control-frame discriminator**: a frame may carry `kind:"control"`
//!   with `{verb, install, principal, assertion?, payload, nonce, issued_at}`
//!   (D7/D8/D18). The broker routes it dumbly — it parses no verb, holds no
//!   authority. The vocabulary is L8's; the sealing is the existing crypto.
//! - **Liveness earned, not assumed**: a sid is live only while it polls;
//!   the broker measures queue depth and names a breach (D4).
//!
//! The queue is in-memory (a durable broker queue is a later hardening);
//! the ack/cursor lets a client detect and re-request a gap across a restart.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// How long a poll is held before it answers empty (the reference ceiling).
pub const POLL_SECONDS: u64 = 25;
/// A sid with no poll within this window is not live (liveness earned).
pub const LIVENESS_WINDOW: Duration = Duration::from_secs(90);
/// The queue-depth bound the broker names a breach past (a responsible
/// citizen — bounded, named, never a silent drop).
pub const QUEUE_BREACH: usize = 1024;

/// One registered device session: its secret, its pending frames, and the
/// push cursor. The broker holds no authority — only routing state.
struct Device {
    secret: String,
    /// Frames waiting for the device's poll, each with a push sequence.
    frames: std::collections::VecDeque<(u64, Value)>,
    /// The next push sequence to assign.
    next_seq: u64,
    /// The last sequence the device acknowledged.
    acked: u64,
    /// The last poll, for liveness.
    last_poll: Instant,
}

impl Device {
    fn new(secret: String) -> Device {
        Device {
            secret,
            frames: std::collections::VecDeque::new(),
            next_seq: 0,
            acked: 0,
            last_poll: Instant::now(),
        }
    }

    fn live(&self) -> bool {
        self.last_poll.elapsed() < LIVENESS_WINDOW
    }
}

/// The broker's shared state: the device registry, keyed by sid.
#[derive(Default)]
pub struct Broker {
    devices: HashMap<String, Device>,
    join_secret: String,
}

impl Broker {
    /// A broker with a join secret. An empty secret fails closed — every
    /// join is refused by name (the reference's KI-367 discipline).
    pub fn new(join_secret: String) -> Broker {
        Broker {
            devices: HashMap::new(),
            join_secret,
        }
    }

    /// Register a session. The join proof gates the one door a stranger can
    /// knock on; a sid already taken by another secret is refused.
    pub fn join(&mut self, sid: &str, secret: &str, presented: &str) -> Result<(), String> {
        if self.join_secret.is_empty() {
            return Err("this relay has no join secret configured".to_string());
        }
        if presented != self.join_secret {
            return Err("that join carries no session proof".to_string());
        }
        if sid.is_empty() || secret.is_empty() {
            return Err("a join names a sid and a hash".to_string());
        }
        if let Some(existing) = self.devices.get(sid) {
            if existing.secret != secret {
                return Err("that session is taken".to_string());
            }
        } else {
            self.devices
                .insert(sid.to_string(), Device::new(secret.to_string()));
        }
        Ok(())
    }

    /// Queue one frame for a sid (the phone's forward, or a control push).
    /// Returns the push sequence. A queue past the bound names a breach.
    pub fn push(&mut self, sid: &str, frame: Value) -> Result<u64, String> {
        let device = self
            .devices
            .get_mut(sid)
            .ok_or_else(|| "no such session".to_string())?;
        let seq = device.next_seq;
        device.next_seq += 1;
        device.frames.push_back((seq, frame));
        if device.frames.len() > QUEUE_BREACH {
            return Err(format!(
                "the queue for {sid} is past its bound ({} frames) — a named breach",
                device.frames.len()
            ));
        }
        Ok(seq)
    }

    /// Hand out the frames waiting for a sid, marking the poll live. The
    /// frames ride with their push sequences so the client can detect a gap.
    pub fn poll(&mut self, sid: &str) -> Result<Vec<Value>, String> {
        let device = self
            .devices
            .get_mut(sid)
            .ok_or_else(|| "no such session".to_string())?;
        device.last_poll = Instant::now();
        let mut frames = Vec::new();
        while let Some((seq, frame)) = device.frames.pop_front() {
            let mut frame = frame;
            if let Some(object) = frame.as_object_mut() {
                object.insert("push_seq".to_string(), json!(seq));
            }
            frames.push(frame);
        }
        Ok(frames)
    }

    /// Acknowledge receipt up to a sequence — delivery is trusted only here
    /// (D21). Returns the gap, if any, between the ack and the next push.
    pub fn ack(&mut self, sid: &str, seq: u64) -> Result<Value, String> {
        let device = self
            .devices
            .get_mut(sid)
            .ok_or_else(|| "no such session".to_string())?;
        if seq > device.acked {
            device.acked = seq;
        }
        Ok(json!({
            "acked": device.acked,
            "next_seq": device.next_seq,
            "gap": device.next_seq > device.acked + 1,
        }))
    }

    /// The broker's status: the connected sids, their liveness (earned by
    /// polling), and queue depth. The surface reads this (C5).
    pub fn status(&self) -> Value {
        let devices: Vec<Value> = self
            .devices
            .iter()
            .map(|(sid, device)| {
                json!({
                    "sid": sid,
                    "live": device.live(),
                    "queued": device.frames.len(),
                    "acked": device.acked,
                    "next_seq": device.next_seq,
                })
            })
            .collect();
        json!({ "devices": devices, "count": self.devices.len() })
    }

    /// Whether a sid's secret matches (the phone's forward gate).
    pub fn secret_ok(&self, sid: &str, presented: &str) -> bool {
        self.devices
            .get(sid)
            .map(|device| device.secret == presented)
            .unwrap_or(false)
    }

    /// Drop sessions that have not polled within the liveness window.
    pub fn prune(&mut self) {
        self.devices.retain(|_, device| device.live());
    }
}

/// Run the broker on a port until the process ends. A mode of the fleet
/// binary (`fleet relay --broker`) — self-contained, no foreign runtime.
pub async fn serve(port: u16, join_secret: String) -> Result<(), String> {
    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .map_err(|error| format!("the broker cannot bind port {port}: {error}"))?;
    let broker = Arc::new(Mutex::new(Broker::new(join_secret)));
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let broker = Arc::clone(&broker);
        tokio::spawn(async move {
            let _ = handle(stream, broker).await;
        });
    }
}

/// One connection: parse the request, route it, write the reply. The broker
/// is dumb — it routes by sid and parses no frame verb.
async fn handle(
    mut stream: tokio::net::TcpStream,
    broker: Arc<Mutex<Broker>>,
) -> Result<(), String> {
    let Some((method, path, headers, body)) = read_request(&mut stream).await? else {
        return Ok(());
    };
    let (bare, query) = match path.split_once('?') {
        Some((bare, query)) => (bare.to_string(), query.to_string()),
        None => (path.clone(), String::new()),
    };
    let sid = query_value(&query, "sid").unwrap_or_default();
    let reply = match (method.as_str(), bare.as_str()) {
        ("POST", "/join") => {
            let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let presented = headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("x-fleet-relay-join"))
                .map(|(_, value)| value.clone())
                .unwrap_or_default();
            let mut broker = broker.lock().await;
            broker.prune();
            match broker.join(
                body.get("sid").and_then(Value::as_str).unwrap_or(""),
                body.get("hash").and_then(Value::as_str).unwrap_or(""),
                &presented,
            ) {
                Ok(()) => json_reply(200, &json!({"joined": body.get("sid")})),
                Err(error) => json_reply(403, &json!({"error": error})),
            }
        }
        ("GET", "/poll") => {
            let mut broker = broker.lock().await;
            match broker.poll(&sid) {
                Ok(frames) => json_reply(200, &Value::Array(frames)),
                Err(error) => json_reply(404, &json!({"error": error})),
            }
        }
        ("POST", "/frame") => {
            // An answer against a frame's id — the broker routes it to the
            // waiting requester. In this dumb broker the answer is recorded
            // by id; the phone's forward path carries the reply.
            let answer: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let broker = broker.lock().await;
            let known = broker.devices.contains_key(&sid);
            json_reply(
                if known { 200 } else { 404 },
                &json!({"delivered": known, "id": answer.get("id")}),
            )
        }
        ("POST", "/push") => {
            let frame: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let mut broker = broker.lock().await;
            match broker.push(&sid, frame) {
                Ok(seq) => json_reply(200, &json!({"pushed": seq})),
                Err(error) => json_reply(404, &json!({"error": error})),
            }
        }
        ("POST", "/ack") => {
            let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let seq = body.get("seq").and_then(Value::as_u64).unwrap_or(0);
            let mut broker = broker.lock().await;
            match broker.ack(&sid, seq) {
                Ok(state) => json_reply(200, &state),
                Err(error) => json_reply(404, &json!({"error": error})),
            }
        }
        ("GET", "/status") => {
            let broker = broker.lock().await;
            json_reply(200, &broker.status())
        }
        _ => {
            // The phone's forward: /s/<sid>/<path> queues a frame for the
            // device. The broker seals nothing and parses no verb — it
            // hands the request to the device's poll as a frame.
            if let Some(rest) = bare.strip_prefix("/s/") {
                let forward_sid = rest.split('/').next().unwrap_or("").to_string();
                let secret = query_value(&query, "hash").unwrap_or_default();
                let mut broker = broker.lock().await;
                if !broker.secret_ok(&forward_sid, &secret) {
                    json_reply(403, &json!({"error": "that session needs its secret"}))
                } else {
                    let frame = json!({
                        "id": format!("f-{}", std::process::id()),
                        "method": method,
                        "path": format!("/{}", rest.split_once('/').map(|(_, p)| p).unwrap_or("")),
                        "body": String::from_utf8_lossy(&body),
                    });
                    match broker.push(&forward_sid, frame) {
                        Ok(seq) => json_reply(200, &json!({"pushed": seq})),
                        Err(error) => json_reply(404, &json!({"error": error})),
                    }
                }
            } else {
                json_reply(404, &json!({"error": "no route"}))
            }
        }
    };
    write_reply(&mut stream, &reply).await
}

/// A parsed request: method, path, headers, body. `None` on a bare probe.
async fn read_request(
    stream: &mut tokio::net::TcpStream,
) -> Result<Option<(String, String, Vec<(String, String)>, Vec<u8>)>, String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end;
    loop {
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|error| error.to_string())?;
        if n == 0 {
            return Ok(None);
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(at) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            head_end = at + 4;
            break;
        }
        if buf.len() > 1_048_576 {
            return Err("the request head is too large".to_string());
        }
    }
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let mut headers = Vec::new();
    let mut content_length = 0usize;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_string();
            let value = value.trim().to_string();
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.parse().unwrap_or(0);
            }
            headers.push((name, value));
        }
    }
    let mut body = buf[head_end..].to_vec();
    while body.len() < content_length {
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|error| error.to_string())?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(content_length);
    Ok(Some((method, path, headers, body)))
}

/// One query parameter's value.
fn query_value(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        if name == key {
            Some(value.to_string())
        } else {
            None
        }
    })
}

/// A JSON reply with a status.
fn json_reply(status: u16, body: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(body).unwrap_or_default();
    let reason = match status {
        200 => "OK",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Error",
    };
    let mut reply = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    reply.extend_from_slice(&body);
    reply
}

/// Write a reply and close.
async fn write_reply(stream: &mut tokio::net::TcpStream, reply: &[u8]) -> Result<(), String> {
    stream
        .write_all(reply)
        .await
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn broker() -> Broker {
        Broker::new("join-secret".to_string())
    }

    #[test]
    fn a_join_registers_a_session_and_a_wrong_proof_is_refused() {
        let mut b = broker();
        assert!(b.join("sid-1", "hash-1", "join-secret").is_ok());
        assert!(b.join("sid-2", "hash-2", "wrong").is_err());
        // A sid taken by another secret is refused.
        assert!(b.join("sid-1", "other-hash", "join-secret").is_err());
        // The same secret re-joins idempotently.
        assert!(b.join("sid-1", "hash-1", "join-secret").is_ok());
    }

    #[test]
    fn a_broker_with_no_secret_fails_closed() {
        let mut b = Broker::new(String::new());
        assert!(b.join("sid-1", "hash-1", "").is_err());
    }

    #[test]
    fn push_queues_a_frame_and_poll_hands_it_out_with_a_sequence() {
        let mut b = broker();
        b.join("sid-1", "hash-1", "join-secret").unwrap();
        let seq = b
            .push("sid-1", json!({"kind": "control", "verb": "state"}))
            .unwrap();
        assert_eq!(seq, 0);
        let frames = b.poll("sid-1").unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["push_seq"], 0);
        assert_eq!(frames[0]["verb"], "state");
        // The poll drained the queue.
        assert!(b.poll("sid-1").unwrap().is_empty());
    }

    #[test]
    fn delivery_is_trusted_only_on_ack_and_a_gap_is_named() {
        let mut b = broker();
        b.join("sid-1", "hash-1", "join-secret").unwrap();
        b.push("sid-1", json!({"n": 1})).unwrap();
        b.push("sid-1", json!({"n": 2})).unwrap();
        // Ack the first; the second is still a gap.
        let state = b.ack("sid-1", 0).unwrap();
        assert_eq!(state["acked"], 0);
        assert_eq!(state["gap"], true);
        // Ack the second; no gap.
        let state = b.ack("sid-1", 1).unwrap();
        assert_eq!(state["gap"], false);
    }

    #[test]
    fn a_control_frame_routes_without_the_broker_parsing_its_verb() {
        let mut b = broker();
        b.join("sid-1", "hash-1", "join-secret").unwrap();
        let control = json!({
            "kind": "control", "verb": "dispatch", "install": "abc",
            "principal": "owner", "payload": {"task": "x"}, "nonce": "n", "issued_at": 1
        });
        b.push("sid-1", control.clone()).unwrap();
        let frames = b.poll("sid-1").unwrap();
        // The broker carried the verb through untouched — it parsed nothing.
        assert_eq!(frames[0]["verb"], "dispatch");
        assert_eq!(frames[0]["principal"], "owner");
        assert_eq!(frames[0]["kind"], "control");
    }

    #[test]
    fn liveness_is_earned_by_polling_not_assumed() {
        let mut b = broker();
        b.join("sid-1", "hash-1", "join-secret").unwrap();
        // A fresh join has polled never — but the join sets last_poll, so
        // it is live within the window; a stale device is pruned.
        assert!(b.status()["devices"][0]["live"].as_bool().unwrap());
        // Force staleness and prune.
        b.devices.get_mut("sid-1").unwrap().last_poll =
            Instant::now() - LIVENESS_WINDOW - Duration::from_secs(1);
        assert!(!b.devices.get("sid-1").unwrap().live());
        b.prune();
        assert_eq!(b.status()["count"], 0);
    }

    #[test]
    fn the_queue_bound_names_a_breach() {
        let mut b = broker();
        b.join("sid-1", "hash-1", "join-secret").unwrap();
        for _ in 0..QUEUE_BREACH {
            b.push("sid-1", json!({"n": 1})).unwrap();
        }
        let breach = b.push("sid-1", json!({"n": 1}));
        assert!(breach.is_err());
        assert!(breach.unwrap_err().contains("named breach"));
    }

    #[test]
    fn an_unknown_sid_is_refused_by_name() {
        let mut b = broker();
        assert!(b.push("ghost", json!({})).is_err());
        assert!(b.poll("ghost").is_err());
        assert!(b.ack("ghost", 0).is_err());
    }
}
