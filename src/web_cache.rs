//! The web lane's answer cache: identical calls within a TTL are
//! answered from the home's `web-cache.json` instead of the lane, so a
//! development loop does not pay TinyFish for the same question twice.
//!
//! The key is the call's own arguments — the exact JSON the model sent —
//! beside the lane's name, so a different query, a different fetch flag,
//! or a different page bound is a different key and a fresh read. Only
//! answers are cached; a lane that failed is exactly the one worth
//! retrying, and an error that outlived its cause would be a lie the
//! cache told twice.
//!
//! The TTL is `FLEET_WEB_CACHE_TTL_SECS`, default one hour, `0`
//! disabling the cache outright — the owner's dial for the cost-versus-
//! freshness trade the blueprint names.

use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

/// The cache's file, beside the route cache in the fleet home.
pub const CACHE_FILE: &str = "web-cache.json";

/// The default time a cached answer stays fresh, in seconds.
pub const DEFAULT_TTL_SECS: u64 = 3_600;

/// The most entries the cache holds. A web loop generates questions
/// faster than it revisits them, so the cap is an eviction bound, not a
/// working-set estimate: the oldest entry goes when the cap is hit.
const MAX_ENTRIES: usize = 64;

/// One cached answer: what the lane said, and when it was said.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    stored_at: u64,
    answer: String,
}

/// The cache, loaded and held by the caller that dispatches web calls.
#[derive(Debug, Default)]
pub struct WebCache {
    path: Option<PathBuf>,
    ttl_secs: u64,
    entries: BTreeMap<String, Entry>,
}

/// Seconds since the epoch, for the TTL arithmetic and the stamps.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// The TTL from the environment, or the default. `0` disables the
/// cache: every call is a fresh read, which is what a development loop
/// that is testing the lane itself wants.
pub fn ttl_secs() -> u64 {
    std::env::var("FLEET_WEB_CACHE_TTL_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_TTL_SECS)
}

/// The cache key for one web call: the lane's name and the exact
/// arguments the model sent. The arguments are already stable JSON —
/// the same question produces byte-identical calls.
pub fn key_for(lane: &str, arguments: &str) -> String {
    format!("{lane}:{arguments}")
}

impl WebCache {
    /// Load the cache from the home, with the TTL from the environment.
    /// A missing or unparsable file is an empty cache, not an error —
    /// the cache only ever saves money, and a broken cache must not
    /// cost a run.
    pub fn load(home: &Path) -> WebCache {
        let path = home.join(CACHE_FILE);
        let entries: BTreeMap<String, Entry> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        WebCache {
            path: Some(path),
            ttl_secs: ttl_secs(),
            entries,
        }
    }

    /// A cache that holds nothing and writes nowhere: the disabled
    /// shape, for callers whose TTL is zero.
    pub fn disabled() -> WebCache {
        WebCache {
            path: None,
            ttl_secs: 0,
            entries: BTreeMap::new(),
        }
    }

    /// Whether the cache is enabled: a zero TTL disables it.
    pub fn enabled(&self) -> bool {
        self.ttl_secs > 0 && self.path.is_some()
    }

    /// The cached answer for one call, when it is fresh enough. An
    /// expired entry is dropped here, so a put after a miss never
    /// resurrects it.
    pub fn get(&mut self, key: &str) -> Option<(String, u64)> {
        let now = now_secs();
        let entry = self.entries.get(key)?;
        let age = now.saturating_sub(entry.stored_at);
        if age >= self.ttl_secs {
            // Expired: dropped here, so a put after a miss never
            // resurrects it.
            self.entries.remove(key);
            return None;
        }
        Some((entry.answer.clone(), age))
    }

    /// Cache one answer, evicting the oldest entry when the cap is hit.
    pub fn put(&mut self, key: String, answer: String) {
        let now = now_secs();
        self.entries.insert(key, Entry { stored_at: now, answer });
        while self.entries.len() > MAX_ENTRIES {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.stored_at)
                .map(|(key, _)| key.clone());
            match oldest {
                Some(key) => {
                    self.entries.remove(&key);
                }
                None => break,
            }
        }
    }

    /// Write the cache to the home, atomically: a torn cache file is an
    /// empty cache on the next load, never a failed run.
    pub fn flush(&self) {
        let Some(path) = &self.path else { return };
        if self.entries.is_empty() {
            return;
        }
        let body = match serde_json::to_string(&self.entries) {
            Ok(body) => body,
            Err(_) => return,
        };
        let temp = path.with_extension("json.tmp");
        if std::fs::write(&temp, body).is_ok() {
            let _ = std::fs::rename(&temp, path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cached_answer_round_trips_within_the_ttl() {
        let dir = std::env::temp_dir().join(format!("fleet-web-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut cache = WebCache::load(&dir);
        cache.put(key_for("blend1", r#"{"query":"rust crates"}"#), "the answer".to_string());
        let (answer, age) = cache
            .get(&key_for("blend1", r#"{"query":"rust crates"}"#))
            .expect("a fresh entry answers");
        assert_eq!(answer, "the answer");
        assert!(age < 5, "{age}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_different_query_is_a_different_key() {
        let dir = std::env::temp_dir().join(format!("fleet-web-cache-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut cache = WebCache::load(&dir);
        cache.put(key_for("blend1", r#"{"query":"a"}"#), "answer a".to_string());
        assert!(cache.get(&key_for("blend1", r#"{"query":"b"}"#)).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_round_trip_through_the_file_survives_a_reload() {
        let dir = std::env::temp_dir().join(format!("fleet-web-cache-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        {
            let mut cache = WebCache::load(&dir);
            cache.put(key_for("blend1", r#"{"query":"a"}"#), "the answer".to_string());
            cache.flush();
        }
        let mut reloaded = WebCache::load(&dir);
        assert_eq!(
            reloaded
                .get(&key_for("blend1", r#"{"query":"a"}"#))
                .expect("the answer survives the reload")
                .0,
            "the answer"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_disabled_cache_never_answers_and_never_writes() {
        let dir = std::env::temp_dir().join(format!("fleet-web-cache-off-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut cache = WebCache::load(&dir);
        cache.ttl_secs = 0;
        cache.put(key_for("blend1", r#"{"query":"a"}"#), "the answer".to_string());
        assert!(cache.get(&key_for("blend1", r#"{"query":"a"}"#)).is_none());
        cache.flush();
        assert!(!dir.join(CACHE_FILE).exists(), "a disabled cache writes nothing");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_cap_evicts_the_oldest_entry() {
        let dir = std::env::temp_dir().join(format!("fleet-web-cache-cap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut cache = WebCache::load(&dir);
        for index in 0..=MAX_ENTRIES {
            cache.put(format!("key-{index}"), format!("answer-{index}"));
        }
        assert!(cache.entries.len() <= MAX_ENTRIES);
        assert!(
            !cache.entries.contains_key(&format!("key-0")),
            "the oldest entry is the one that goes"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
