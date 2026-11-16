//! Dictation: the microphone, transcribed by hex.
//!
//! The owner's idea (2026-11-16): speak into fleet. hex
//! (<https://github.com/anomalyco/hex>) is local, MIT, and ships for
//! macOS, Linux and Windows — the platform-agnostic transcriber.
//!
//! The split is hex's own (`docs/specs/local-transcription-service.md`):
//! **the host owns the microphone; hex owns the models and the
//! inference.** The embedded helper never opens the mic.
//!
//! The integration is the embedded service, not the CLI:
//!
//! 1. spend `hex service --embedded` as a child with piped stdin — the
//!    helper binds a loopback port and writes one JSON ready line
//!    (`{type, url, token, apiVersion, pid}`) in milliseconds;
//! 2. `POST {url}/transcriptions?model=<id>` with a WAV body and
//!    `Authorization: Bearer {token}` answers `{transcript, durationMs}`;
//! 3. the child's stdin is the lease: closing it is a clean shutdown.
//!
//! (`hex transcribe` — the CLI — is not used: in 3.4.0 on macOS it
//! refuses unless Hex.app is running, and it reads WAV only.)

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// The model hex recommends for English dictation; `--model` overrides it.
pub const DEFAULT_MODEL: &str = "parakeet_unified_en";

const READY_TIMEOUT: Duration = Duration::from_secs(10);
const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(180);

/// The service's ready line: the endpoint and the bearer token it minted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ready {
    pub url: String,
    pub token: String,
    pub pid: u32,
}

/// One clip's transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    pub text: String,
    pub duration_ms: Option<u64>,
}

/// One finished recording, ready to transcribe.
#[derive(Debug, Clone, PartialEq)]
pub struct Recording {
    pub wav: Vec<u8>,
    pub device: String,
    pub seconds: f64,
    pub samples: usize,
}

/// Parse the service's one ready line. Refusals name what went wrong: a
/// line that is not `ready`, an endpoint that is not loopback, an empty
/// token.
pub fn parse_ready(line: &str) -> Result<Ready, String> {
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|error| format!("the hex service's first line is not JSON: {error}"))?;
    if value.get("type").and_then(|one| one.as_str()) != Some("ready") {
        return Err(format!(
            "the hex service's first line is not `ready`: {}",
            line.trim()
        ));
    }
    let url = value
        .get("url")
        .and_then(|one| one.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    if !url.starts_with("http://127.0.0.1:") {
        return Err(format!(
            "the hex service named an endpoint that is not loopback: `{url}`"
        ));
    }
    let token = value
        .get("token")
        .and_then(|one| one.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    if token.is_empty() {
        return Err("the hex service's ready line carries no token".to_string());
    }
    let pid = value.get("pid").and_then(|one| one.as_u64()).unwrap_or(0) as u32;
    Ok(Ready { url, token, pid })
}

/// Parse a `POST /transcriptions` reply.
fn parse_transcript(body: &str) -> Result<Clip, String> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("the hex service's reply is not JSON: {error}"))?;
    let text = value
        .get("transcript")
        .and_then(|one| one.as_str())
        .ok_or_else(|| "the hex service's reply carries no transcript".to_string())?
        .to_string();
    let duration_ms = value.get("durationMs").and_then(|one| one.as_u64());
    Ok(Clip { text, duration_ms })
}

/// The refusal a non-2xx reply carries: hex's own words when the body is
/// its error document (`{status: "error", code, message}`), the status
/// otherwise. The message is where a missing model names
/// `hex model install`.
fn refusal(status: u16, body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let (Some(code), Some(message)) = (
            value.get("code").and_then(|one| one.as_str()),
            value.get("message").and_then(|one| one.as_str()),
        ) {
            return format!("the hex service refused ({code}): {message}");
        }
    }
    format!("the hex service refused with HTTP {status}")
}

/// Where the hex binary lives: `FLEET_HEX` first (a path, judged by
/// existence), then a `hex` on PATH, then the macOS app bundle. A refusal
/// names every place that was looked at.
pub fn hex_binary() -> Result<PathBuf, String> {
    if let Some(asked) = std::env::var_os("FLEET_HEX") {
        let path = PathBuf::from(&asked);
        return if path.is_file() {
            Ok(path)
        } else {
            Err(format!(
                "FLEET_HEX names `{}`, which is not a file",
                path.display()
            ))
        };
    }
    if let Some(found) = path_lookup("hex") {
        return Ok(found);
    }
    let bundled = PathBuf::from("/Applications/Hex.app/Contents/MacOS/hex");
    if bundled.is_file() {
        return Ok(bundled);
    }
    Err("hex is not installed — dictation reads a `hex` on PATH, \
         /Applications/Hex.app/Contents/MacOS/hex, or the path FLEET_HEX names \
         (install: brew install --cask anomalyco/tap/hex)"
        .to_string())
}

fn path_lookup(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// The service, spawned and kept warm for the life of the verb.
pub struct Service {
    child: Child,
    stdin: Option<ChildStdin>,
    ready: Ready,
    agent: ureq::Agent,
}

impl Service {
    /// Resolve the binary and start the service.
    pub fn start() -> Result<Service, String> {
        Self::start_with(&hex_binary()?)
    }

    /// Start a given binary as `service --embedded` and wait for its one
    /// ready line — bounded, so a child that dies or hangs cannot hold the
    /// verb.
    pub fn start_with(binary: &Path) -> Result<Service, String> {
        let mut child = Command::new(binary)
            .arg("service")
            .arg("--embedded")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                format!(
                    "the hex service could not start ({}): {error}",
                    binary.display()
                )
            })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "the hex service's stdin is not piped".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "the hex service's stdout is not piped".to_string())?;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = std::io::BufReader::new(stdout);
            let mut line = String::new();
            let _ = sender.send(reader.read_line(&mut line).map(|read| (read, line)));
        });
        let (read, line) = match receiver.recv_timeout(READY_TIMEOUT) {
            Ok(Ok(pair)) => pair,
            Ok(Err(error)) => {
                let _ = child.kill();
                return Err(format!(
                    "the hex service's stdout could not be read: {error}"
                ));
            }
            Err(_) => {
                let _ = child.kill();
                return Err(format!(
                    "the hex service wrote no ready line within {}s",
                    READY_TIMEOUT.as_secs()
                ));
            }
        };
        if read == 0 {
            let status = child.try_wait().ok().flatten();
            let _ = child.kill();
            return Err(match status {
                Some(status) => format!("the hex service exited before it was ready ({status})"),
                None => "the hex service closed its output before it was ready".to_string(),
            });
        }
        let ready = match parse_ready(&line) {
            Ok(ready) => ready,
            Err(error) => {
                let _ = child.kill();
                return Err(error);
            }
        };
        Ok(Service {
            child,
            stdin: Some(stdin),
            ready,
            agent: ureq::AgentBuilder::new()
                .timeout(TRANSCRIBE_TIMEOUT)
                .build(),
        })
    }

    /// The endpoint and token the service minted.
    pub fn ready(&self) -> &Ready {
        &self.ready
    }

    /// Transcribe one WAV clip. The reply's text is the model's own; hex
    /// never pastes, rewrites or records.
    pub fn transcribe(&self, wav: &[u8], model: &str) -> Result<Clip, String> {
        if model.is_empty()
            || !model
                .chars()
                .all(|one| one.is_ascii_alphanumeric() || one == '_' || one == '-' || one == '.')
        {
            return Err(format!("`{model}` is not a model id"));
        }
        let url = format!("{}/transcriptions?model={model}", self.ready.url);
        let request = self
            .agent
            .post(&url)
            .set("Authorization", &format!("Bearer {}", self.ready.token))
            .set("Content-Type", "audio/wav");
        match request.send_bytes(wav) {
            Ok(reply) => {
                let body = reply.into_string().map_err(|error| {
                    format!("the hex service's reply could not be read: {error}")
                })?;
                parse_transcript(&body)
            }
            Err(ureq::Error::Status(status, reply)) => {
                let body = reply.into_string().unwrap_or_default();
                Err(refusal(status, &body))
            }
            Err(ureq::Error::Transport(error)) => {
                Err(format!("the hex service did not answer: {error}"))
            }
        }
    }
}

impl Drop for Service {
    /// Closing stdin is the lease's end; the helper shuts down cleanly and
    /// is killed only if it does not.
    fn drop(&mut self) {
        drop(self.stdin.take());
        for _ in 0..60 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A 16-bit PCM WAV around interleaved samples — the shape hex accepts.
/// The device's own rate and channel count ride the header; hex resamples.
pub fn wav_bytes(samples: &[i16], channels: u16, rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.max(1).to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * channels.max(1) as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(channels.max(1) * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

/// Record the default input for `seconds` and return the WAV. The device's
/// own configuration is kept (rate and channels); no device is a refusal
/// by name.
pub fn record(seconds: f64) -> Result<Recording, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    if !(0.2..=600.0).contains(&seconds) {
        return Err(format!(
            "{seconds}s is not a recording length fleet will take (0.2 to 600)"
        ));
    }
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "no input device is available to record from".to_string())?;
    let name = device
        .description()
        .map(|described| described.name().to_string())
        .unwrap_or_else(|_| "the default input".to_string());
    let supported = device
        .default_input_config()
        .map_err(|error| format!("`{name}` has no input configuration: {error}"))?;
    let sample_format = supported.sample_format();
    let channels = supported.channels();
    let rate = supported.sample_rate();
    let config = supported.config();
    let sink = std::sync::Arc::new(std::sync::Mutex::new(Vec::<i16>::new()));
    let on_error = |_error| {};
    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let sink = sink.clone();
            device.build_input_stream(
                config,
                move |data: &[f32], _| pump(&sink, data.iter().copied()),
                on_error,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let sink = sink.clone();
            device.build_input_stream(
                config,
                move |data: &[i16], _| {
                    if let Ok(mut held) = sink.lock() {
                        held.extend_from_slice(data);
                    }
                },
                on_error,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let sink = sink.clone();
            device.build_input_stream(
                config,
                move |data: &[u16], _| pump(&sink, data.iter().map(|one| *one as f32 - 32768.0)),
                on_error,
                None,
            )
        }
        other => {
            return Err(format!(
                "`{name}` speaks {other:?} samples, which fleet cannot record yet"
            ))
        }
    }
    .map_err(|error| format!("`{name}` would not open for recording: {error}"))?;
    stream
        .play()
        .map_err(|error| format!("`{name}` would not start recording: {error}"))?;
    std::thread::sleep(Duration::from_secs_f64(seconds));
    drop(stream);
    let samples = sink.lock().map(|held| held.clone()).unwrap_or_default();
    Ok(Recording {
        wav: wav_bytes(&samples, channels, rate),
        device: name,
        seconds,
        samples: samples.len(),
    })
}

/// Collect F32-range samples as i16 (the WAV's shape).
fn pump(sink: &std::sync::Arc<std::sync::Mutex<Vec<i16>>>, samples: impl Iterator<Item = f32>) {
    if let Ok(mut held) = sink.lock() {
        held.extend(samples.map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ready_line_names_the_loopback_endpoint_and_token() {
        let ready = parse_ready(
            r#"{"type":"ready","url":"http://127.0.0.1:49731","token":"hex_abc","apiVersion":"2","pid":42}"#,
        )
        .unwrap();
        assert_eq!(ready.url, "http://127.0.0.1:49731");
        assert_eq!(ready.token, "hex_abc");
        assert_eq!(ready.pid, 42);
    }

    #[test]
    fn a_ready_line_that_is_not_ready_and_an_off_loopback_url_are_refused() {
        let not_ready = parse_ready(r#"{"type":"error","code":"not-running"}"#).unwrap_err();
        assert!(not_ready.contains("not `ready`"), "{not_ready}");
        let public = parse_ready(
            r#"{"type":"ready","url":"http://example.com:1","token":"t","apiVersion":"2","pid":1}"#,
        )
        .unwrap_err();
        assert!(public.contains("not loopback"), "{public}");
        let malformed = parse_ready("{not json").unwrap_err();
        assert!(malformed.contains("not JSON"), "{malformed}");
        let tokenless = parse_ready(
            r#"{"type":"ready","url":"http://127.0.0.1:9","token":"","apiVersion":"2","pid":1}"#,
        )
        .unwrap_err();
        assert!(tokenless.contains("no token"), "{tokenless}");
    }

    #[test]
    fn the_transcript_reply_carries_the_text_and_the_duration() {
        let clip = parse_transcript(r#"{"durationMs":4565,"transcript":"hello there"}"#).unwrap();
        assert_eq!(clip.text, "hello there");
        assert_eq!(clip.duration_ms, Some(4565));
        let empty = parse_transcript(r#"{"transcript":"","durationMs":0}"#).unwrap();
        assert_eq!(empty.text, "");
        assert!(parse_transcript(r#"{"durationMs":1}"#)
            .unwrap_err()
            .contains("no transcript"));
    }

    #[test]
    fn a_refusal_keeps_hexs_own_words() {
        let missing = refusal(
            400,
            r#"{"status":"error","code":"model-unavailable","message":"`m` isn't ready; run `hex model install m`"}"#,
        );
        assert!(missing.contains("model-unavailable"), "{missing}");
        assert!(missing.contains("hex model install m"), "{missing}");
        let bare = refusal(500, "<html>oops</html>");
        assert_eq!(bare, "the hex service refused with HTTP 500");
    }

    #[test]
    fn the_wav_header_is_riff_pcm_16_bit() {
        let wav = wav_bytes(&[0, 1, -1], 1, 16_000);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(u16::from_le_bytes([wav[20], wav[21]]), 1, "PCM");
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1, "mono");
        assert_eq!(
            u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]),
            16_000
        );
        assert_eq!(u16::from_le_bytes([wav[34], wav[35]]), 16, "16-bit");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]),
            6,
            "three 16-bit samples"
        );
        assert_eq!(wav.len(), 44 + 6);
    }

    #[test]
    fn a_model_id_the_url_could_not_carry_is_refused() {
        // The check lives in transcribe; the pure part is worth pinning here.
        let service = Service {
            child: Command::new("true").spawn().unwrap(),
            stdin: None,
            ready: Ready {
                url: "http://127.0.0.1:9".to_string(),
                token: "t".to_string(),
                pid: 0,
            },
            agent: ureq::AgentBuilder::new().build(),
        };
        let error = service.transcribe(b"RIFF", "bad model?x=1").unwrap_err();
        assert!(error.contains("not a model id"), "{error}");
    }
}
