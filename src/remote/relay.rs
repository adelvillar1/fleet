//! The outbound relay: Fleet dials out, so a phone on cellular can reach
//! it with nothing installed on the network.
//!
//! ## Why outbound
//!
//! A laptop is behind a NAT. An inbound listener needs a port forward, a
//! public address, or a hole punch, and none of those are things an
//! owner should have to arrange to read a session from a cafe. So Fleet
//! dials out instead: it holds an HTTP long-poll against a relay, and
//! the relay shuttles frames between that poll and whatever the phone
//! asks for. The phone's requests are HTTP; Fleet's answers are HTTP;
//! the relay never parses either.
//!
//! ```text
//!   phone ──HTTPS──► relay ──frame──► Fleet's long-poll ──► the remote
//!          ◄──HTTP──        ◄──frame──   (one task, one socket)    listener's
//!                                                               handler
//! ```
//!
//! ## The protocol, and why it is this shape
//!
//! Three verbs. `join` registers the device session the pairing window
//! minted. `poll` is a long-held GET that returns frames — each one a
//! phone's HTTP request, whole, with an id. `frame` is a POST carrying
//! one answer back. That is the whole contract: a relay is a map of
//! pending request ids and two queues, and the reference implementation
//! is a hundred lines of anything.
//!
//! The page is Fleet's own, served through the relay rather than copied
//! into it: the phone asks the relay for `/`, the relay forwards the
//! request to Fleet as a frame, and Fleet answers with the same page the
//! desktop gets. There is no second copy of the face to keep in sync.
//!
//! ## The threat model, stated plainly
//!
//! The relay sees metadata — the paths a phone asked for, the frame
//! ids, the sizes — and ciphertext for everything content-bearing.
//! The frame key comes from an X25519 exchange the relay only watched:
//! the device holds a per-launch scalar, the phone holds its own, and
//! neither public key alone derives anything. A relay (and the TLS
//! edge in front of it) that records every byte it sees still cannot
//! open a frame. What the relay can do is refuse to carry them, which
//! is availability, not confidentiality. The browser-forced install
//! fetches — the shell page, its assets, the manifest, the worker,
//! the pairing page and its redemption — are plaintext frames of
//! public code, by necessity: a browser navigation cannot seal.

use std::sync::Arc;
use std::time::Duration;

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::Aes256Gcm;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// How long an HTTP call to the relay may take. A poll is held for
/// [`POLL_SECS`], so the ceiling has to be comfortably past it: a
/// timeout shorter than the hold is a client that abandons a poll the
/// relay was about to answer.
const POLL_SECS: u64 = 25;

/// How long an HTTP call to the relay may take.
const CALL_TIMEOUT: Duration = Duration::from_secs(POLL_SECS + 15);

/// What the relay is told at `join`: the device session and the secret
/// that proves it. Both come from the pairing window, so a relay can
/// only ever be joined by a device that paired.
#[derive(Debug, Serialize)]
pub struct Join {
    pub sid: String,
    pub hash: String,
}

/// One phone request, forwarded by the relay as a frame. Whole and
/// unparsed: the relay does not know what a route means, and neither
/// does this module — the frame goes to the remote listener's handler,
/// which already knows.
///
/// `enc` is the seal marker the phone's fetch wrapper sets: a body
/// that arrived sealed, which this side opens before the handler sees
/// it and re-seals the answer against. Absent means the frame rides as
/// it is — the install set, the handshake, the redemption.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Frame {
    pub id: String,
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enc: Option<u8>,
}

/// One answer, posted back to the relay against the frame's id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub id: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enc: Option<u8>,
}

/// Whether a frame's path is one whose content must be sealed: the API
/// and the stream. The handshake and the redemption are `/api/` paths
/// that cannot be — the handshake is what produces the key, and the
/// redemption answers a phone that has no key yet.
fn is_sealed_path(path: &str) -> bool {
    let bare = path.split('?').next().unwrap_or(path);
    if bare == "/api/relay/handshake" || bare == "/api/pair" {
        return false;
    }
    bare == "/events" || bare.starts_with("/api/")
}

/// The frame key and the seal. The key is HKDF-SHA-256 over the X25519
/// shared secret, salted with the device session id and bound to the
/// fixed info string, so a re-handshake on a new launch derives a
/// different key even from the same peer scalar. The AEAD is
/// AES-256-GCM; the nonce rides the body, and the frame id (for an
/// answer) or the request path (for a request) is the additional data,
/// so a body cannot be replayed onto another frame.
pub struct Sealer {
    key: [u8; 32],
}

impl Sealer {
    /// Derive the seal from the raw X25519 shared secret and the
    /// session id both endpoints already know.
    pub fn from_shared(shared: &[u8], session: &str) -> Sealer {
        let hk = Hkdf::<Sha256>::new(Some(session.as_bytes()), shared);
        let mut key = [0u8; 32];
        hk.expand(b"fleet relay frame v1", &mut key)
            .expect("a 32-byte expand from a 32-byte secret never fails");
        Sealer { key }
    }

    /// Seal a body. The output is base64(nonce ‖ ciphertext ‖ tag).
    pub fn seal(&self, aad: &str, body: &str) -> Result<String, String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|error| format!("the seal could not build its cipher: {error}"))?;
        let mut nonce = [0u8; 12];
        use rand::RngCore;
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let sealed = cipher
            .encrypt(
                (&nonce).into(),
                Payload {
                    msg: body.as_bytes(),
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| "the seal refused a body".to_string())?;
        let mut out = Vec::with_capacity(12 + sealed.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&sealed);
        Ok(B64.encode(&out))
    }

    /// Open a body that [`seal`](Sealer::seal) produced.
    pub fn open(&self, aad: &str, body: &str) -> Result<String, String> {
        let raw = B64
            .decode(body)
            .map_err(|_| "a sealed body was not base64".to_string())?;
        if raw.len() < 13 {
            return Err("a sealed body was too short to carry a nonce".to_string());
        }
        let (nonce, sealed) = raw.split_at(12);
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|error| format!("the seal could not build its cipher: {error}"))?;
        let plain = cipher
            .decrypt(
                nonce.into(),
                Payload {
                    msg: sealed,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| "a sealed frame did not open under this seal".to_string())?;
        String::from_utf8(plain)
            .map_err(|_| "a sealed frame opened to bytes that were not text".to_string())
    }
}

/// The device's half of the exchange: a per-launch X25519 scalar, and
/// the seal that exists once a phone's public key has arrived. Shared
/// between the serve loop (which opens and seals frames) and the
/// handler (which answers the handshake), which is why it lives behind
/// a mutex rather than in either alone.
pub struct RelayKeys {
    scalar: x25519_dalek::StaticSecret,
    public: x25519_dalek::PublicKey,
    session: String,
    sealer: Option<Sealer>,
}

impl RelayKeys {
    /// A fresh exchange for one launch: a random scalar, its public
    /// half, and no seal yet.
    pub fn mint(session: &str) -> RelayKeys {
        let scalar = x25519_dalek::StaticSecret::random_from_rng(rand::rngs::OsRng);
        let public = x25519_dalek::PublicKey::from(&scalar);
        RelayKeys {
            scalar,
            public,
            session: session.to_string(),
            sealer: None,
        }
    }

    /// The public half, base64, for the handshake's answer.
    pub fn public_b64(&self) -> String {
        B64.encode(self.public.as_bytes())
    }

    /// The generation stamp: the first eight hex of the session id. A
    /// phone that boots compares what it stored against this — a
    /// different value means the device relaunched and the exchange
    /// must run again.
    pub fn gen(&self) -> String {
        self.session.chars().take(8).collect()
    }

    /// The session id, which the phone needs as the HKDF salt. It is
    /// not a secret — the relay learns it at join.
    pub fn session(&self) -> &str {
        &self.session
    }

    /// Establish the seal from the phone's public key. Idempotent per
    /// peer: the same public key derives the same seal, so a phone
    /// that repeats its handshake does not rotate anything.
    pub fn establish(&mut self, peer_b64: &str) -> Result<(), String> {
        let raw = B64
            .decode(peer_b64)
            .map_err(|_| "the handshake's public key was not base64".to_string())?;
        let bytes: [u8; 32] = raw
            .try_into()
            .map_err(|_| "the handshake's public key was not 32 bytes".to_string())?;
        let peer = x25519_dalek::PublicKey::from(bytes);
        let shared = self.scalar.diffie_hellman(&peer);
        if !shared.was_contributory() {
            return Err("the handshake's public key contributed nothing".to_string());
        }
        self.sealer = Some(Sealer::from_shared(shared.as_bytes(), &self.session));
        Ok(())
    }

    /// Whether a seal exists — the phone has handed its public key over
    /// at least once this launch.
    pub fn established(&self) -> bool {
        self.sealer.is_some()
    }

    /// Seal with the established key.
    pub fn seal(&self, aad: &str, body: &str) -> Result<String, String> {
        self.sealer
            .as_ref()
            .ok_or_else(|| "no seal is established".to_string())?
            .seal(aad, body)
    }

    /// Open with the established key.
    pub fn open(&self, aad: &str, body: &str) -> Result<String, String> {
        self.sealer
            .as_ref()
            .ok_or_else(|| "no seal is established".to_string())?
            .open(aad, body)
    }
}

/// The relay client: the address, the device session, and the secret.
/// One per face; the session outlives any single poll.
pub struct Client {
    base_url: String,
    sid: String,
    hash: String,
}

impl Client {
    /// A client for one relay and one device session.
    pub fn new(base_url: &str, sid: String, hash: String) -> Client {
        Client {
            base_url: base_url.trim_end_matches('/').to_string(),
            sid,
            hash,
        }
    }

    /// The relay's base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Register this device session with the relay. Idempotent: joining
    /// twice is the same session, which is what a reconnect wants.
    pub fn join(&self) -> Result<(), String> {
        let url = format!("{}/join", self.base_url);
        ureq::post(&url)
            .timeout(CALL_TIMEOUT)
            .send_json(Join {
                sid: self.sid.clone(),
                hash: self.hash.clone(),
            })
            .map_err(|error| format!("the relay refused the join: {error}"))?;
        Ok(())
    }

    /// Hold one poll and return the frames that arrived. An empty vec is
    /// the normal answer to a poll that timed out at the relay's
    /// ceiling: the caller polls again.
    pub fn poll(&self) -> Result<Vec<Frame>, String> {
        let url = format!("{}/poll?sid={}", self.base_url, self.sid);
        let response = ureq::get(&url)
            .timeout(CALL_TIMEOUT)
            .call()
            .map_err(|error| format!("the relay poll failed: {error}"))?;
        let frames: Vec<Frame> = response
            .into_json()
            .map_err(|error| format!("the relay's frames were not readable: {error}"))?;
        Ok(frames)
    }

    /// Post one answer back against its frame's id.
    pub fn answer(&self, answer: &Answer) -> Result<(), String> {
        let url = format!("{}/frame?sid={}", self.base_url, self.sid);
        ureq::post(&url)
            .timeout(CALL_TIMEOUT)
            .send_json(answer)
            .map_err(|error| format!("the relay refused the answer: {error}"))?;
        Ok(())
    }
}

/// Where the relay client's answers come from: the remote listener's own
/// handler, which already knows how to answer a request against the
/// bridge. The relay module hands it a frame and takes back an answer,
/// so there is one implementation of "what a request means" and the
/// relay is not a second one.
pub trait Handler: Send + Sync {
    /// Answer one forwarded request. The boxed future is what keeps the
    /// trait object-safe: the handler is chosen at run time, so the trait
    /// has to be usable as `dyn Handler`.
    fn answer(
        &self,
        frame: &Frame,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Answer> + Send + '_>>;
}

/// Serves the relay's frames until the process stops: one poll loop on
/// its own task, each frame answered by the handler and posted back.
///
/// The loop is deliberately single-threaded in its polling — one poll at
/// a time, because the relay holds one device session and a second
/// concurrent poll would race the first for the same frames.
pub async fn serve(
    client: Arc<Client>,
    handler: Arc<dyn Handler>,
    shutdown: tokio::sync::watch::Receiver<bool>,
    keys: Arc<std::sync::Mutex<RelayKeys>>,
) {
    serve_with_ready(client, handler, shutdown, keys, None).await
}

/// `serve`, carrying the lifecycle signal KI-351 named: `ready` fires
/// after the first successful join — the moment the loop can answer —
/// so a test waits on the relay's own word instead of a wall clock.
pub async fn serve_with_ready(
    client: Arc<Client>,
    handler: Arc<dyn Handler>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    keys: Arc<std::sync::Mutex<RelayKeys>>,
    mut ready: Option<tokio::sync::oneshot::Sender<()>>,
) {
    // A join refused at startup once ended the loop outright while a
    // failed poll inside it rejoined — two policies for one condition.
    // One policy now, the loop's own: a transient join failure pauses
    // two seconds and tries again, and only the shutdown ends the
    // relay. KI-351 was this split — one load-induced join error killed
    // the loop and the test polled an answer that could never come.
    loop {
        match client.join() {
            Ok(()) => {
                eprintln!("fleet relay — joined {}", client.base_url());
                if let Some(ready) = ready.take() {
                    let _ = ready.send(());
                }
                break;
            }
            Err(error) => eprintln!("fleet relay: {error}; rejoining"),
        }
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_ok() && *shutdown.borrow_and_update() {
                    return;
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
        }
    }
    loop {
        if *shutdown.borrow_and_update() {
            return;
        }
        // The poll is blocking HTTP on a dedicated thread: `ureq` is a
        // blocking client, and the bridge already spawns threads for
        // exactly this shape of work.
        let polling = {
            let client = Arc::clone(&client);
            tokio::task::spawn_blocking(move || client.poll())
        };
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_ok() && *shutdown.borrow_and_update() {
                    return;
                }
            }
            polled = polling => match polled {
                Ok(Ok(frames)) => {
                    for mut frame in frames {
                        // The seal boundary lives here, at the client's
                        // edge: a sealed frame is opened before the
                        // handler sees it and its answer is sealed on
                        // the way back, so the handler — and everything
                        // behind it — keeps reading plaintext. A
                        // content-bearing frame that arrived without
                        // the seal is refused by name, because a
                        // plaintext task text crossing the relay is
                        // the one thing this module exists to prevent.
                        let sealed = frame.enc == Some(1);
                        let answer = if !sealed && is_sealed_path(&frame.path) {
                            Answer {
                                id: frame.id.clone(),
                                status: 400,
                                headers: vec![(
                                    "content-type".to_string(),
                                    "application/json".to_string(),
                                )],
                                body: serde_json::json!({
                                    "error": "this relay seals its content — reload the face so the page seals its frames"
                                })
                                .to_string(),
                                enc: None,
                            }
                        } else {
                            if sealed {
                                // The guard binds to a value, not a
                                // match scrutinee: a temporary held to
                                // the end of the statement is a guard
                                // held across the awaits below, and a
                                // non-Send future tokio refuses to
                                // spawn.
                                let opened =
                                    keys.lock().expect("the keys lock").open(&frame.path, &frame.body);
                                match opened {
                                    Ok(plain) => frame.body = plain,
                                    Err(error) => {
                                        let refused = Answer {
                                            id: frame.id.clone(),
                                            status: 400,
                                            headers: vec![(
                                                "content-type".to_string(),
                                                "application/json".to_string(),
                                            )],
                                            body: serde_json::json!({
                                                "error": format!(
                                                    "a sealed frame did not open: {error} — the exchange may be stale; reload the face"
                                                )
                                            })
                                            .to_string(),
                                            enc: None,
                                        };
                                        let posting = {
                                            let client = Arc::clone(&client);
                                            let refused = refused.clone();
                                            tokio::task::spawn_blocking(move || client.answer(&refused))
                                        };
                                        if let Ok(Err(error)) = posting.await {
                                            eprintln!("fleet relay: {error}");
                                        }
                                        continue;
                                    }
                                }
                            }
                            let mut answer = handler.answer(&frame).await;
                            if sealed {
                                let sealed_body =
                                    keys.lock().expect("the keys lock").seal(&frame.id, &answer.body);
                                match sealed_body {
                                    Ok(body) => {
                                        answer.body = body;
                                        answer.enc = Some(1);
                                    }
                                    Err(error) => {
                                        eprintln!("fleet relay: {error}");
                                        answer.body = serde_json::json!({
                                            "error": "the answer could not be sealed"
                                        })
                                        .to_string();
                                        answer.status = 500;
                                        answer.enc = None;
                                    }
                                }
                            }
                            answer
                        };
                        let posting = {
                            let client = Arc::clone(&client);
                            let answer = answer.clone();
                            tokio::task::spawn_blocking(move || client.answer(&answer))
                        };
                        if let Ok(Err(error)) = posting.await {
                            eprintln!("fleet relay: {error}");
                        }
                    }
                }
                Ok(Err(error)) => {
                    eprintln!("fleet relay: {error}");
                    // A failed poll is a relay that went away: wait a
                    // moment rather than hammering it, then rejoin.
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    let _ = client.join();
                }
                Err(_) => return,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A relay in a test: a tiny HTTP server speaking the three verbs,
    /// holding the frames a test posts to it and handing them out on the
    /// next poll.
    struct FakeRelay {
        frames: std::sync::Mutex<Vec<Frame>>,
        answers: std::sync::Mutex<Vec<Answer>>,
        /// The capturing half: every body this relay has ever handed to
        /// a poll or received as an answer, verbatim. The seal's proof
        /// is pinned against these bytes, the way the audit pins
        /// what-the-model-saw claims.
        capture: std::sync::Mutex<Vec<String>>,
        joined: std::sync::atomic::AtomicUsize,
        /// How many polls are in flight right now, and the most that
        /// have ever been: the loop holds one at a time, and a loop that
        /// does not wait for its poll leaks one per iteration.
        in_flight: std::sync::atomic::AtomicUsize,
        most_in_flight: std::sync::atomic::AtomicUsize,
    }

    impl FakeRelay {
        fn new() -> Arc<FakeRelay> {
            Arc::new(FakeRelay {
                frames: std::sync::Mutex::new(Vec::new()),
                answers: std::sync::Mutex::new(Vec::new()),
                capture: std::sync::Mutex::new(Vec::new()),
                joined: std::sync::atomic::AtomicUsize::new(0),
                in_flight: std::sync::atomic::AtomicUsize::new(0),
                most_in_flight: std::sync::atomic::AtomicUsize::new(0),
            })
        }

        fn record(&self, body: &str) {
            self.capture
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .push(body.to_string());
        }

        /// Queue one frame the next poll will hand out.
        fn offer(&self, frame: Frame) {
            self.frames
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .push(frame);
        }
    }

    /// A handler that answers every frame with a fixed body, so the test
    /// can assert the relay got the answer back.
    struct Echo;

    impl Handler for Echo {
        fn answer(
            &self,
            frame: &Frame,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Answer> + Send + '_>> {
            // Owned copies, so the future does not borrow the frame the
            // caller still holds.
            let id = frame.id.clone();
            let path = frame.path.clone();
            Box::pin(async move {
                Answer {
                    id,
                    status: 200,
                    headers: vec![("content-type".to_string(), "application/json".to_string())],
                    body: format!("{{\"path\":\"{path}\"}}"),
                    enc: None,
                }
            })
        }
    }

    fn frame(id: &str, path: &str) -> Frame {
        Frame {
            id: id.to_string(),
            method: "GET".to_string(),
            path: path.to_string(),
            headers: vec![("host".to_string(), "127.0.0.1".to_string())],
            body: String::new(),
            enc: None,
        }
    }

    /// The keys a serve loop runs under: minted per test, one exchange
    /// per launch.
    fn keys_for(session: &str) -> Arc<std::sync::Mutex<RelayKeys>> {
        Arc::new(std::sync::Mutex::new(RelayKeys::mint(session)))
    }

    /// Starts the fake relay on an OS-picked port and returns its base
    /// URL. The server answers `join`, hands out one queued frame per
    /// `poll` (empty after that), and records every `frame` it is sent.
    async fn start(relay: Arc<FakeRelay>) -> String {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("a port");
        let port = listener.local_addr().expect("an address").port();
        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                let relay = Arc::clone(&relay);
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut head = Vec::new();
                    let mut chunk = [0u8; 1024];
                    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                        let read = stream.read(&mut chunk).await.unwrap_or(0);
                        if read == 0 {
                            return;
                        }
                        head.extend_from_slice(&chunk[..read]);
                    }
                    let text = String::from_utf8_lossy(&head).to_string();
                    let (status, body) = if text.starts_with("POST /join") {
                        relay
                            .joined
                            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        ("200 OK", "{}\n".to_string())
                    } else if text.starts_with("GET /poll") {
                        let flight = relay
                            .in_flight
                            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                            + 1;
                        relay
                            .most_in_flight
                            .fetch_max(flight, std::sync::atomic::Ordering::SeqCst);
                        // Hold the poll the way the real relay does: the
                        // client's own timeout is what ends it.
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        relay
                            .in_flight
                            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                        let frames = relay
                            .frames
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner());
                        let queued = frames.clone();
                        drop(frames);
                        for frame in &queued {
                            relay.record(&frame.body);
                        }
                        relay
                            .frames
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner())
                            .clear();
                        (
                            "200 OK",
                            format!("{}\n", serde_json::to_string(&queued).unwrap()),
                        )
                    } else if text.starts_with("POST /frame") {
                        let body = text
                            .split("\r\n\r\n")
                            .nth(1)
                            .unwrap_or_default()
                            .to_string();
                        if let Ok(answer) = serde_json::from_str::<Answer>(body.trim()) {
                            relay.record(&answer.body);
                            relay
                                .answers
                                .lock()
                                .unwrap_or_else(|poison| poison.into_inner())
                                .push(answer);
                        }
                        ("200 OK", "{}\n".to_string())
                    } else {
                        ("404 Not Found", "{}\n".to_string())
                    };
                    let reply = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(reply.as_bytes()).await;
                });
            }
        });
        format!("http://127.0.0.1:{port}")
    }

    /// The serve loop holds one poll at a time. A loop that does not
    /// wait — a dropped shutdown sender answering `changed()` at once,
    /// say — leaks a poll task per iteration: the blocking pool
    /// saturates at 512 threads and the relay sees hundreds of
    /// connections at once.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_serve_loop_holds_one_poll_at_a_time() {
        let relay = FakeRelay::new();
        let base = start(Arc::clone(&relay)).await;
        let client = Arc::new(Client::new(
            &base,
            "sid-rate".to_string(),
            "hash".to_string(),
        ));
        let (shutdown, shutdown_rx) = tokio::sync::watch::channel(false);
        let _serving = tokio::spawn(serve(
            Arc::clone(&client),
            Arc::new(Echo),
            shutdown_rx,
            keys_for("sid-rate"),
        ));
        // The sender stays alive for the loop's life: `let _ =` would
        // drop it at once.
        let _ = shutdown;
        // The loop's own counters are the observable state: a bounded
        // poll gives a loaded machine the time the first poll needs,
        // where the fixed 1.5 s wait red under a parallel suite
        // (KI-288). The leak bound stays: a dropped sender answers
        // `changed()` at once and drives the in-flight count up within
        // a short soak, so the soak can be short.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            let seen = relay.joined.load(std::sync::atomic::Ordering::SeqCst);
            let most = relay
                .most_in_flight
                .load(std::sync::atomic::Ordering::SeqCst);
            if seen >= 1 && most >= 1 {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the loop never reached the relay under the deadline: joined={seen}, most={most}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        let most = relay
            .most_in_flight
            .load(std::sync::atomic::Ordering::SeqCst);
        let seen = relay.joined.load(std::sync::atomic::Ordering::SeqCst);
        assert!(seen >= 1, "the loop never joined the relay: {seen}");
        assert!(most >= 1, "no poll ever reached the relay: {most}");
        assert!(
            most <= 2,
            "the loop leaked polls: {most} in flight at once (a dropped sender answers changed() at once)"
        );
    }

    /// A poll returns the frames the relay is holding, and the answers go
    /// back against their ids — the round trip a phone's request makes.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_poll_hands_out_frames_and_the_answers_go_back() {
        let relay = FakeRelay::new();
        let base = start(Arc::clone(&relay)).await;
        let client = Arc::new(Client::new(
            &base,
            "sid-1".to_string(),
            "hash-1".to_string(),
        ));
        client.join().expect("the join lands");

        relay.offer(frame("f1", "/api/state"));
        relay.offer(frame("f2", "/api/sessions"));
        let frames = client.poll().expect("the poll lands");
        assert_eq!(frames.len(), 2, "{frames:?}");
        assert_eq!(frames[0].id, "f1");
        assert_eq!(frames[0].path, "/api/state");
        // The second poll is empty: the frames were handed out once.
        assert!(client.poll().expect("the poll lands").is_empty());

        for frame in &frames {
            let answer = Answer {
                id: frame.id.clone(),
                status: 200,
                headers: Vec::new(),
                body: "ok".to_string(),
                enc: None,
            };
            client.answer(&answer).expect("the answer lands");
        }
        let answers = relay
            .answers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone();
        assert_eq!(answers.len(), 2, "{answers:?}");
        assert_eq!(answers[0].id, "f1");
        assert_eq!(answers[1].id, "f2");
    }

    /// The serve loop polls until it is told to stop, answering what
    /// arrives with the handler's answer.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_serve_loop_answers_until_it_is_told_to_stop() {
        let relay = FakeRelay::new();
        let base = start(Arc::clone(&relay)).await;
        let client = Arc::new(Client::new(
            &base,
            "sid-2".to_string(),
            "hash-2".to_string(),
        ));
        // A shell path, not an API one: the loop's unsealed refusal is
        // its own test below, and this test pins the passthrough.
        relay.offer(frame("g1", "/state"));
        let (shutdown, shutdown_rx) = tokio::sync::watch::channel(false);
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let serving = tokio::spawn(serve_with_ready(
            Arc::clone(&client),
            Arc::new(Echo),
            shutdown_rx,
            keys_for("sid-2"),
            Some(ready_tx),
        ));
        // KI-351's named action: the relay's own lifecycle word, not a
        // clock. Readiness fires after the join the loop answers from —
        // a join whose one-shot refusal at startup used to kill the
        // loop outright, which the retry inside `serve_with_ready` now
        // absorbs — and the two deadlines below reach only a relay that
        // never comes up or never polls, where the old wait's two
        // seconds red under a loaded suite with a correct relay slow.
        tokio::time::timeout(Duration::from_secs(60), ready_rx)
            .await
            .expect("the relay never announced itself within 60 s")
            .expect("the readiness channel closed before the join answered");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            if !relay
                .answers
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .is_empty()
            {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the joined relay never answered the offered frame within 60 s"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let answers = relay
            .answers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone();
        assert_eq!(answers.len(), 1, "{answers:?}");
        assert_eq!(answers[0].id, "g1");
        assert_eq!(answers[0].status, 200);
        assert_eq!(answers[0].body, r#"{"path":"/state"}"#);
        // And it stops when told.
        let _ = shutdown.send(true);
        let _ = serving.await;
    }

    /// The seal's paths are the content paths: the API and the stream
    /// are sealed, the handshake and the redemption cannot be (the
    /// handshake is what produces the key; the redemption answers a
    /// phone with no key yet), and the shell is public code.
    #[test]
    fn the_seal_paths_are_the_content_paths() {
        assert!(is_sealed_path("/api/state"));
        assert!(is_sealed_path("/api/run"));
        assert!(is_sealed_path("/events"));
        assert!(!is_sealed_path("/api/relay/handshake"));
        assert!(!is_sealed_path("/api/pair"));
        assert!(!is_sealed_path("/"));
        assert!(!is_sealed_path("/app.js?v=abc"));
    }

    /// A key pair round-trips, binds its bodies to the context they
    /// were sealed for, and refuses a body bound for another frame.
    #[test]
    fn the_seal_round_trips_and_binds_its_context() {
        let mut device = RelayKeys::mint("sid-seal");
        let peer = x25519_dalek::StaticSecret::random_from_rng(rand::rngs::OsRng);
        let peer_public = x25519_dalek::PublicKey::from(&peer);
        let device_public_bytes: [u8; 32] = B64
            .decode(device.public_b64())
            .expect("the public is base64")
            .try_into()
            .expect("the public is 32 bytes");
        let device_public = x25519_dalek::PublicKey::from(device_public_bytes);
        let phone = Sealer::from_shared(peer.diffie_hellman(&device_public).as_bytes(), "sid-seal");
        device
            .establish(&B64.encode(peer_public.as_bytes()))
            .expect("the exchange lands");

        let task = "summarize the fleet repo quietly";
        let sealed = phone.seal("/api/run", task).expect("the seal lands");
        assert!(
            !sealed.contains("summarize"),
            "the ciphertext leaks the text: {sealed}"
        );
        let opened = device.open("/api/run", &sealed).expect("the open lands");
        assert_eq!(opened, task);

        // The answer goes back sealed against the frame's id, and the
        // phone opens it under the same binding. A body bound for
        // another frame does not open here.
        let answer = device
            .seal("f-1", "the summary text")
            .expect("the answer's seal lands");
        assert_eq!(
            phone.open("f-1", &answer).expect("the answer opens"),
            "the summary text"
        );
        assert!(phone.open("f-2", &answer).is_err());
    }

    /// The exchange refuses a public key that is not one: not base64,
    /// not 32 bytes, or one that contributes nothing (the all-zero
    /// key, whose shared secret every X25519 implementation pins out).
    #[test]
    fn the_exchange_refuses_a_public_key_that_is_not_one() {
        let mut keys = RelayKeys::mint("sid-bad");
        assert!(keys.establish("!! not base64 !!").is_err());
        assert!(keys.establish(&B64.encode([7u8; 12])).is_err());
        assert!(keys.establish(&B64.encode([0u8; 32])).is_err());
        assert!(!keys.established());
        let peer = x25519_dalek::StaticSecret::random_from_rng(rand::rngs::OsRng);
        keys.establish(&B64.encode(x25519_dalek::PublicKey::from(&peer).as_bytes()))
            .expect("a real peer establishes");
        assert!(keys.established());
    }

    /// A content-bearing frame that arrives without the seal is
    /// refused by name: a plaintext task text crossing the relay is
    /// the one thing this module exists to prevent.
    #[tokio::test(flavor = "multi_thread")]
    async fn an_unsealed_api_frame_is_refused_by_name() {
        let relay = FakeRelay::new();
        let base = start(Arc::clone(&relay)).await;
        let client = Arc::new(Client::new(
            &base,
            "sid-refuse".to_string(),
            "hash".to_string(),
        ));
        relay.offer(frame("r1", "/api/state"));
        let (shutdown, shutdown_rx) = tokio::sync::watch::channel(false);
        let serving = tokio::spawn(serve(
            Arc::clone(&client),
            Arc::new(Echo),
            shutdown_rx,
            keys_for("sid-refuse"),
        ));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            // The lock lives inside the block and dies with it: a guard
            // that reached the await below would hold the fake relay's
            // mutex across a sleep, which the lint names and the suite
            // feels under a parallel run.
            let refused = {
                let answers = relay.answers.lock().unwrap_or_else(|p| p.into_inner());
                answers
                    .first()
                    .map(|answer| (answer.status, answer.body.clone(), answer.enc))
            };
            if let Some((status, body, enc)) = refused {
                assert_eq!(status, 400, "{body:?}");
                assert!(body.contains("seals"), "{body:?}");
                assert_eq!(enc, None, "{body:?}");
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the refusal never reached the relay"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let _ = shutdown.send(true);
        let _ = serving.await;
    }

    /// The end-to-end shape through the loop: a sealed request opens
    /// for the handler, the answer comes back sealed, and the bytes
    /// the relay captured — pinned, the way the audit pins
    /// what-the-model-saw — carry neither the task's text nor the
    /// answer's.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_sealed_frame_opens_and_its_answer_comes_back_sealed() {
        let relay = FakeRelay::new();
        let base = start(Arc::clone(&relay)).await;
        let client = Arc::new(Client::new(
            &base,
            "sid-e2e".to_string(),
            "hash".to_string(),
        ));

        // The two halves of the exchange: the device's keys, and the
        // phone's own scalar against the device's public. The relay
        // sees only the public halves.
        let mut device = RelayKeys::mint("sid-e2e");
        let peer = x25519_dalek::StaticSecret::random_from_rng(rand::rngs::OsRng);
        let device_public_bytes: [u8; 32] = B64
            .decode(device.public_b64())
            .expect("the public is base64")
            .try_into()
            .expect("the public is 32 bytes");
        let device_public = x25519_dalek::PublicKey::from(device_public_bytes);
        let phone = Sealer::from_shared(peer.diffie_hellman(&device_public).as_bytes(), "sid-e2e");
        device
            .establish(&B64.encode(x25519_dalek::PublicKey::from(&peer).as_bytes()))
            .expect("the exchange lands");
        let keys = Arc::new(std::sync::Mutex::new(device));

        let task = "summarize the fleet repo quietly";
        let sealed_body = phone.seal("/api/run", task).expect("the seal lands");
        let mut offered = frame("s1", "/api/run");
        offered.method = "POST".to_string();
        offered.body = sealed_body;
        offered.enc = Some(1);
        relay.offer(offered);

        let (shutdown, shutdown_rx) = tokio::sync::watch::channel(false);
        let serving = tokio::spawn(serve(
            Arc::clone(&client),
            Arc::new(Echo),
            shutdown_rx,
            Arc::clone(&keys),
        ));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            let landed = {
                let answers = relay.answers.lock().unwrap_or_else(|p| p.into_inner());
                answers
                    .first()
                    .map(|answer| (answer.status, answer.enc, answer.body.clone()))
            };
            if let Some((status, enc, body)) = landed {
                assert_eq!(status, 200, "{body:?}");
                assert_eq!(enc, Some(1), "{body:?}");
                let opened = phone
                    .open("s1", &body)
                    .expect("the answer opens for the phone");
                assert_eq!(opened, r#"{"path":"/api/run"}"#);
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the sealed round trip never landed"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let _ = shutdown.send(true);
        let _ = serving.await;

        // The capture is the proof: every byte the relay held, and
        // none of them says what the task said.
        let held = relay
            .capture
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .join(" ");
        assert!(
            !held.contains("summarize"),
            "the relay's capture leaks the task text: {held}"
        );
        assert!(!held.is_empty(), "the capture recorded nothing");
    }
}
