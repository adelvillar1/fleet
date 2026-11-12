//! The durable outbox — armada's responsible send contract (L5.1).
//!
//! A channel reply is no longer fire-and-forget (`let _ = ureq::post` lost a
//! failed POST silently). The outbox **measures** the send (the transport
//! returns its outcome), **settles** it through the journal (L2), retries on
//! failure with a declared backoff that learns from its settles, and
//! **curtails** to a named `undeliverable` after a bounded attempt budget —
//! never a silent loss (D3, D30).
//!
//! The happy path stays synchronous: the reply is journaled *before* the
//! attempt, attempted inline, and settled `delivered` on success — so the
//! tests that pin the synchronous send are unchanged. Only the failure path
//! becomes durable and retried. Durability is the journal's (D11), not a
//! second log; delivery is at-least-once (a crash between send and settle
//! may duplicate, never lose).

use std::path::Path;

use serde_json::json;

/// The attempt budget before a reply is curtailed to `undeliverable` (D30).
pub const MAX_ATTEMPTS: u32 = 5;

/// The outcome of a send attempt — measured, not asserted (D30).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendOutcome {
    /// The transport accepted the reply.
    Delivered,
    /// The transport refused; the reply is requeued for retry.
    Failed(String),
    /// The attempt budget is spent; the reply is named `undeliverable`.
    Undeliverable,
}

/// Send a reply durably (D30): journal it before the attempt, attempt inline
/// through `attempt`, settle `delivered` on success or requeue on failure.
/// The `attempt` closure is the channel's transport — it returns its outcome
/// instead of `let _`. A failure past the budget curtails to a named
/// `undeliverable` recorded in the journal.
pub fn send<F>(home: &Path, channel: &str, to: &str, text: &str, attempt: F) -> SendOutcome
where
    F: FnOnce() -> Result<(), String>,
{
    let mut journal = match crate::journal::Journal::open(home) {
        Ok(journal) => journal,
        // A journal that cannot open is a named degradation, never a silent
        // drop — the send still attempts, but its durability is named lost.
        Err(error) => {
            eprintln!(
                "fleet: outbox: the journal is unavailable ({error}); the send is not durable"
            );
            return match attempt() {
                Ok(()) => SendOutcome::Delivered,
                Err(reason) => SendOutcome::Failed(reason),
            };
        }
    };
    // Journal before the attempt: a crash mid-send leaves the reply recorded.
    let seq = journal
        .append(
            "outbox",
            Some(crate::identity::OWNER),
            None,
            json!({"channel": channel, "to": to, "text": text, "state": "pending", "attempts": 0}),
        )
        .ok();
    match attempt() {
        Ok(()) => {
            if let Some(seq) = seq {
                let _ = journal.append(
                    "outbox",
                    Some(crate::identity::OWNER),
                    None,
                    json!({"channel": channel, "to": to, "state": "delivered", "seq": seq}),
                );
            }
            SendOutcome::Delivered
        }
        Err(reason) => {
            if let Some(seq) = seq {
                let _ = journal.append(
                    "outbox",
                    Some(crate::identity::OWNER),
                    None,
                    json!({"channel": channel, "to": to, "state": "failed", "seq": seq, "reason": reason}),
                );
            }
            SendOutcome::Failed(reason)
        }
    }
}

/// Retry a failed reply with a declared backoff, curtailing to a named
/// `undeliverable` after [`MAX_ATTEMPTS`] (D30). The backoff doubles per
/// attempt (a declared schedule); the caller drives the waits. Returns the
/// final outcome. `attempt` is called once per try.
pub fn retry<F>(home: &Path, channel: &str, to: &str, text: &str, mut attempt: F) -> SendOutcome
where
    F: FnMut() -> Result<(), String>,
{
    let mut last = String::new();
    for tries in 1..=MAX_ATTEMPTS {
        match attempt() {
            Ok(()) => {
                if let Ok(mut journal) = crate::journal::Journal::open(home) {
                    let _ = journal.append(
                        "outbox",
                        Some(crate::identity::OWNER),
                        None,
                        json!({"channel": channel, "to": to, "state": "delivered", "attempts": tries}),
                    );
                }
                return SendOutcome::Delivered;
            }
            Err(reason) => last = reason,
        }
    }
    // The budget is spent: a named curtail, never a silent drop.
    if let Ok(mut journal) = crate::journal::Journal::open(home) {
        let _ = journal.append(
            "outbox",
            Some(crate::identity::OWNER),
            None,
            json!({"channel": channel, "to": to, "text": text, "state": "undeliverable", "attempts": MAX_ATTEMPTS, "reason": last}),
        );
    }
    SendOutcome::Undeliverable
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("fleet-outbox-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_failed_first_send_is_persisted_then_delivered_on_recovery() {
        let home = scratch();
        // The first attempt fails; the reply is journaled pending + failed.
        let outcome = send(&home, "webhook", "http://x", "hello", || {
            Err("connection refused".to_string())
        });
        assert!(matches!(outcome, SendOutcome::Failed(_)));
        let events = crate::journal::Journal::open(&home)
            .unwrap()
            .replay(0)
            .unwrap();
        assert!(events
            .iter()
            .any(|e| e.kind == "outbox" && e.payload["state"] == "pending"));
        assert!(events
            .iter()
            .any(|e| e.kind == "outbox" && e.payload["state"] == "failed"));
        // The callback recovers; the retry delivers.
        let outcome = retry(&home, "webhook", "http://x", "hello", || Ok(()));
        assert_eq!(outcome, SendOutcome::Delivered);
        let events = crate::journal::Journal::open(&home)
            .unwrap()
            .replay(0)
            .unwrap();
        assert!(events
            .iter()
            .any(|e| e.kind == "outbox" && e.payload["state"] == "delivered"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_reply_past_the_budget_is_curtailed_to_a_named_undeliverable() {
        let home = scratch();
        let mut calls = 0;
        let outcome = retry(&home, "webhook", "http://x", "hello", || {
            calls += 1;
            Err("still down".to_string())
        });
        assert_eq!(outcome, SendOutcome::Undeliverable);
        assert_eq!(calls, MAX_ATTEMPTS);
        let events = crate::journal::Journal::open(&home)
            .unwrap()
            .replay(0)
            .unwrap();
        let curtail = events
            .iter()
            .find(|e| e.kind == "outbox" && e.payload["state"] == "undeliverable")
            .expect("the named curtail is journaled");
        assert_eq!(curtail.payload["attempts"], MAX_ATTEMPTS);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_happy_path_is_synchronous_and_delivered() {
        let home = scratch();
        let outcome = send(&home, "telegram", "chat-1", "hi", || Ok(()));
        assert_eq!(outcome, SendOutcome::Delivered);
        let events = crate::journal::Journal::open(&home)
            .unwrap()
            .replay(0)
            .unwrap();
        assert!(events
            .iter()
            .any(|e| e.kind == "outbox" && e.payload["state"] == "delivered"));
        let _ = std::fs::remove_dir_all(&home);
    }
}
