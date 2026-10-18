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
//! In this increment the relay sees frame plaintext: a task's text, a
//! seat's summary, a session's transcript. The pairing secret gates the
//! session, so an unpaired relay cannot stitch a phone to a device, but
//! a relay that is compromised reads what crosses it. End-to-end
//! encryption — the HPKE-pinned key OpenMausBot's scanner carries — is a
//! later hardening increment, and this module does not pretend to be it.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// How long one poll is held before it is renewed. The relay drops a
/// poll at its own ceiling, so this stays under it: a poll the relay
/// dropped mid-wait is a poll that comes back empty and is retried.
const POLL_SECS: u64 = 25;

/// How long an HTTP call to the relay may take. A poll is held for
/// [`POLL_SECS`], so the ceiling has to be past it.
const CALL_TIMEOUT: Duration = Duration::from_secs(POLL_SECS + 10);

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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Frame {
    pub id: String,
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// One answer, posted back to the relay against the frame's id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub id: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
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
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    if let Err(error) = client.join() {
        eprintln!("fleet relay: {error}");
        return;
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
                    for frame in frames {
                        let answer = handler.answer(&frame).await;
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
        joined: std::sync::atomic::AtomicUsize,
    }

    impl FakeRelay {
        fn new() -> Arc<FakeRelay> {
            Arc::new(FakeRelay {
                frames: std::sync::Mutex::new(Vec::new()),
                answers: std::sync::Mutex::new(Vec::new()),
                joined: std::sync::atomic::AtomicUsize::new(0),
            })
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
        }
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
                        relay.joined.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        ("200 OK", "{}\n".to_string())
                    } else if text.starts_with("GET /poll") {
                        let frames = relay
                            .frames
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner());
                        let queued: Vec<Frame> = frames.clone();
                        drop(frames);
                        relay
                            .frames
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner())
                            .clear();
                        ("200 OK", format!("{}\n", serde_json::to_string(&queued).unwrap()))
                    } else if text.starts_with("POST /frame") {
                        let body = text
                            .split("\r\n\r\n")
                            .nth(1)
                            .unwrap_or_default()
                            .to_string();
                        if let Ok(answer) = serde_json::from_str::<Answer>(body.trim()) {
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

    /// A poll returns the frames the relay is holding, and the answers go
    /// back against their ids — the round trip a phone's request makes.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_poll_hands_out_frames_and_the_answers_go_back() {
        let relay = FakeRelay::new();
        let base = start(Arc::clone(&relay)).await;
        let client = Arc::new(Client::new(&base, "sid-1".to_string(), "hash-1".to_string()));
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
        let client = Arc::new(Client::new(&base, "sid-2".to_string(), "hash-2".to_string()));
        relay.offer(frame("g1", "/api/state"));
        let (shutdown, shutdown_rx) = tokio::sync::watch::channel(false);
        let serving = tokio::spawn(serve(
            Arc::clone(&client),
            Arc::new(Echo),
            shutdown_rx,
        ));
        // Wait for the answer to come back against the frame's id.
        for _ in 0..100 {
            if !relay
                .answers
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .is_empty()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let answers = relay
            .answers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone();
        assert_eq!(answers.len(), 1, "{answers:?}");
        assert_eq!(answers[0].id, "g1");
        assert_eq!(answers[0].status, 200);
        assert_eq!(answers[0].body, r#"{"path":"/api/state"}"#);
        // And it stops when told.
        let _ = shutdown.send(true);
        let _ = serving.await;
    }
}
