//! The hybrid retrieval over a lane's cached corpus store.
//!
//! The lane's store is a pair of files: `manifest.json` (each entry's key,
//! content hash and text) and `vectors.bin` (the packed float32 vectors,
//! parallel to the entries). This module reads both directly — no per-search
//! lane call — BM25s the query over the entry text, cosine-searches the
//! vectors, and fuses the two rankings with RRF. The lane's own hash-gated
//! refresh keeps the store current; the lexical index rides the same
//! invalidation through a sidecar keyed by the entries' content hash.
//!
//! The query's vector comes from fleet's own embedder (the recall path's
//! probed loopback call), not the lane — so a dead lane costs the vector route
//! and the lexical route still answers, the same named degradation the memory
//! ranker has.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::fusion::fuse;
use crate::lexical::{LexDoc, LexIndex};

/// The lane manifest's entry: the corpus key, its content hash, and its text.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestEntry {
    pub key: String,
    pub sha256: String,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(default)]
    #[allow(dead_code)]
    model: String,
    #[serde(default)]
    dims: usize,
    #[serde(default)]
    entries: Vec<ManifestEntry>,
}

/// The cached lexical index, keyed by the entries' content hash.
#[derive(Debug, Serialize, Deserialize)]
struct LexicalCache {
    hash: String,
    index: LexIndex,
}

/// One hybrid hit: the fused score and both routes' contributions.
#[derive(Debug, Clone, PartialEq)]
pub struct HybridHit {
    pub key: String,
    pub score: f64,
    pub lexical: f64,
    pub vector: f64,
    pub text: String,
}

/// A lane's cached corpus store, opened for hybrid search.
#[derive(Debug)]
pub struct HybridStore {
    project: String,
    entries: Vec<ManifestEntry>,
    vectors: Vec<Vec<f32>>,
    lexical: LexIndex,
    /// The structural outlines of the workspace's source, as a second lexical
    /// route beside the corpus. `None` when the caller did not ask for it.
    outline_lexical: Option<LexIndex>,
    /// The outline texts by file path, so a hit carries its retrievable text.
    outline_texts: HashMap<String, String>,
}

/// The directories the outline walk skips — the same generated and vendored
/// trees the glob's walker skips, so the index does not fill with noise.
const OUTLINE_SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    ".mypy_cache",
    ".pytest_cache",
    "dist",
];

/// The most files the outline walk reads. A workspace larger than this is
/// indexed to the cap, newest first — an outline is a sample of the structure,
/// not an inventory.
const OUTLINE_FILE_CAP: usize = 5_000;

/// The lane's store root, resolved the way the lane resolves it: the
/// environment first, then the default under the home.
pub fn store_root() -> PathBuf {
    match std::env::var("BLEND1_VECTORS_DIR") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => home_dir().join(".local/share/blend1/vectors"),
    }
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

impl HybridStore {
    /// Open the store for a project under the lane's store root.
    pub fn open(store_root: &Path, project: &str) -> Result<Self, String> {
        let project_dir = store_root.join(project);
        let dir = find_store_dir(&project_dir)?;
        let manifest = read_manifest(&dir)?;
        let entries = manifest.entries;
        if entries.is_empty() {
            return Err(format!("the store for `{project}` carries no entries"));
        }
        let vectors = read_vectors(&dir, entries.len(), manifest.dims)?;
        let hash = entries_hash(&entries);
        let cache_path = dir.join(".lexical-index.json");
        let lexical = match load_cache(&cache_path, &hash) {
            Some(index) => index,
            None => {
                let docs: Vec<LexDoc> = entries
                    .iter()
                    .map(|entry| LexDoc {
                        id: entry.key.clone(),
                        text: entry.text.clone(),
                    })
                    .collect();
                let index = LexIndex::new(docs);
                let _ = save_cache(
                    &cache_path,
                    &LexicalCache {
                        hash: hash.clone(),
                        index: index.clone(),
                    },
                );
                index
            }
        };
        Ok(Self {
            project: project.to_string(),
            entries,
            vectors,
            lexical,
            outline_lexical: None,
            outline_texts: HashMap::new(),
        })
    }

    /// Open the store and index the workspace's source outlines beside it.
    ///
    /// The outlines are the second lexical route: a query naming a symbol's
    /// signature retrieves the file's structure rather than a line of its
    /// body. The walk skips the generated and vendored trees and caps at
    /// [`OUTLINE_FILE_CAP`] files.
    pub fn open_with_outlines(
        store_root: &Path,
        project: &str,
        workspace: &Path,
    ) -> Result<Self, String> {
        let mut store = Self::open(store_root, project)?;
        let (index, texts) = build_outline_index(workspace)?;
        store.outline_lexical = Some(index);
        store.outline_texts = texts;
        Ok(store)
    }

    pub fn project(&self) -> &str {
        &self.project
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Search with an already-computed query vector — the testable path.
    ///
    /// Three routes fuse: the corpus lexical route, the outline lexical route
    /// when the store was opened with outlines, and the vector route. RRF sums
    /// the routes' rank contributions, so a file the two lexical routes both
    /// name outranks one only the corpus mentions.
    pub fn search_with(
        &self,
        query: &str,
        limit: usize,
        query_vector: Option<&[f32]>,
    ) -> Vec<HybridHit> {
        let lexical = self.lexical.rank(query);
        let mut lexical_scores: HashMap<String, f64> = lexical.iter().cloned().collect();
        let mut routes: Vec<Vec<String>> = vec![ids(&lexical)];
        if let Some(outline) = &self.outline_lexical {
            let outline_hits = outline.rank(query);
            lexical_scores.extend(outline_hits.iter().cloned());
            routes.push(ids(&outline_hits));
        }
        let vector = match query_vector {
            Some(vector) => self.vector_rank(vector),
            None => Vec::new(),
        };
        let vector_scores: HashMap<String, f64> = vector.iter().cloned().collect();
        routes.push(ids(&vector));
        let fused = fuse(&routes);
        fused
            .into_iter()
            .take(limit)
            .map(|(key, score)| HybridHit {
                lexical: lexical_scores.get(&key).copied().unwrap_or(0.0),
                vector: vector_scores.get(&key).copied().unwrap_or(0.0),
                text: self.text_of(&key),
                key,
                score,
            })
            .collect()
    }

    /// Search with fleet's own embedder for the query vector.
    pub fn search(&self, query: &str, limit: usize) -> Vec<HybridHit> {
        let vector = crate::memory::probed_embedder().embed(query);
        let slice = vector.as_deref();
        self.search_with(query, limit, slice)
    }

    fn vector_rank(&self, query_vector: &[f32]) -> Vec<(String, f64)> {
        let mut scored: Vec<(String, f64)> = self
            .entries
            .iter()
            .zip(self.vectors.iter())
            .map(|(entry, vector)| (entry.key.clone(), cosine(query_vector, vector)))
            .filter(|(_, score)| score.is_finite())
            .collect();
        scored.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        scored
    }

    /// The retrievable text of a hit: the corpus entry's text, else the
    /// outline's, so a fused hit always carries the evidence it was ranked on.
    fn text_of(&self, key: &str) -> String {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.text.clone())
            .or_else(|| self.outline_texts.get(key).cloned())
            .unwrap_or_default()
    }
}

/// The ids of a ranked route, best first — the shape [`fuse`] takes.
fn ids(ranked: &[(String, f64)]) -> Vec<String> {
    ranked.iter().map(|(key, _)| key.clone()).collect()
}

/// Build the outline index for a workspace: walk the source files, extract a
/// structural outline from each, and index the outlines' text by file path.
/// A file with no outline (an unsupported language, a parse failure) is
/// skipped, not named — the index is a sample of the structure.
fn build_outline_index(workspace: &Path) -> Result<(LexIndex, HashMap<String, String>), String> {
    let mut docs: Vec<LexDoc> = Vec::new();
    let mut texts: HashMap<String, String> = HashMap::new();
    let mut files: Vec<PathBuf> = Vec::new();
    collect_source_files(workspace, &mut files, 0);
    files.sort();
    for path in files.iter().take(OUTLINE_FILE_CAP) {
        let Some(source) = std::fs::read_to_string(path).ok() else {
            continue;
        };
        let outline = crate::outline::extract(&path.to_string_lossy(), &source);
        if outline.text.is_empty() {
            continue;
        }
        let key = path.to_string_lossy().to_string();
        texts.insert(key.clone(), outline.text.clone());
        docs.push(LexDoc {
            id: key,
            text: outline.text,
        });
    }
    if docs.is_empty() {
        return Err(format!(
            "no source outlines under `{}` — the workspace carries none of the \
             bounded languages",
            workspace.display()
        ));
    }
    Ok((LexIndex::new(docs), texts))
}

/// A depth-bounded walk that skips the generated and vendored trees.
fn collect_source_files(root: &Path, files: &mut Vec<PathBuf>, depth: usize) {
    if depth > 12 || files.len() >= OUTLINE_FILE_CAP {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.is_dir() {
            if OUTLINE_SKIP_DIRS.contains(&name) || name.starts_with('.') {
                continue;
            }
            collect_source_files(&path, files, depth + 1);
        } else if crate::outline::OutlineLanguage::from_path(&path).is_some() {
            files.push(path);
        }
    }
}

/// The cosine between two vectors, 0.0 when either is empty or the dimensions
/// disagree — a mismatched store must not produce a NaN.
fn cosine(left: &[f32], right: &[f32]) -> f64 {
    if left.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut left_norm = 0.0f64;
    let mut right_norm = 0.0f64;
    for (a, b) in left.iter().zip(right.iter()) {
        let a = *a as f64;
        let b = *b as f64;
        dot += a * b;
        left_norm += a * a;
        right_norm += b * b;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }
    dot / (left_norm.sqrt() * right_norm.sqrt())
}

/// Find the store directory for a project: the newest `*/manifest.json` under
/// the project directory. A project with several model slugs uses the most
/// recently written one.
fn find_store_dir(project_dir: &Path) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    let entries = match std::fs::read_dir(project_dir) {
        Ok(entries) => entries,
        Err(error) => {
            return Err(format!(
                "no store for this project under `{}`: {error}",
                project_dir.display()
            ))
        }
    };
    for entry in entries.flatten() {
        let manifest = entry.path().join("manifest.json");
        if manifest.is_file() {
            candidates.push(entry.path());
        }
    }
    if candidates.is_empty() {
        return Err(format!(
            "no manifest under `{}` — the lane has not indexed it",
            project_dir.display()
        ));
    }
    candidates.sort_by_key(|dir| {
        std::fs::metadata(dir)
            .and_then(|meta| meta.modified())
            .unwrap_or(std::time::UNIX_EPOCH)
    });
    Ok(candidates.pop().expect("the list is non-empty"))
}

fn read_manifest(dir: &Path) -> Result<Manifest, String> {
    let path = dir.join("manifest.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("`{}` cannot be read: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("`{}` is not a manifest: {error}", path.display()))
}

/// Read `vectors.bin`: `count * dims` native-endian float32 values, row-major,
/// parallel to the manifest's entries.
fn read_vectors(dir: &Path, count: usize, dims: usize) -> Result<Vec<Vec<f32>>, String> {
    let path = dir.join("vectors.bin");
    let bytes = std::fs::read(&path)
        .map_err(|error| format!("`{}` cannot be read: {error}", path.display()))?;
    if dims == 0 {
        return Err(format!("`{}` declares zero dimensions", path.display()));
    }
    let width = dims * 4;
    if bytes.len() < count * width {
        return Err(format!(
            "`{}` carries {} bytes for {count} entries of {dims} dimensions (needs {})",
            path.display(),
            bytes.len(),
            count * width
        ));
    }
    let mut vectors = Vec::with_capacity(count);
    for index in 0..count {
        let mut vector = Vec::with_capacity(dims);
        let start = index * width;
        let (chunks, remainder) = bytes[start..start + width].as_chunks::<4>();
        debug_assert!(
            remainder.is_empty(),
            "a row is a whole number of f32 values"
        );
        for chunk in chunks {
            vector.push(f32::from_ne_bytes(*chunk));
        }
        vectors.push(vector);
    }
    Ok(vectors)
}

/// A stable hash of the entries' content hashes, so the lexical cache follows
/// the lane's own invalidation: a changed entry changes the hash.
fn entries_hash(entries: &[ManifestEntry]) -> String {
    let mut hash: u64 = 0xcbf_29ce_4842_2223;
    for entry in entries {
        for byte in entry.key.as_bytes().iter().chain(entry.sha256.as_bytes()) {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn load_cache(path: &Path, hash: &str) -> Option<LexIndex> {
    let text = std::fs::read_to_string(path).ok()?;
    let cache: LexicalCache = serde_json::from_str(&text).ok()?;
    if cache.hash == hash {
        Some(cache.index)
    } else {
        None
    }
}

fn save_cache(path: &Path, cache: &LexicalCache) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string(cache).unwrap_or_default())?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixture store on disk: a project with a manifest and a vectors.bin.
    fn fixture(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "fleet-hybrid-{tag}-{}-{serial}",
            std::process::id()
        ));
        let dir = root.join("fixture").join("embeddinggemma-2");
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = serde_json::json!({
            "model": "embeddinggemma-2",
            "dims": 2,
            "count": 2,
            "entries": [
                { "key": "doc-a", "sha256": "aa", "text": "the router selects a model" },
                { "key": "doc-b", "sha256": "bb", "text": "the cache is warm and the store is warm" }
            ]
        });
        std::fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
        // Two 2-dimension vectors, packed native-endian float32.
        let mut bytes = Vec::new();
        for value in [1.0f32, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        std::fs::write(dir.join("vectors.bin"), bytes).unwrap();
        root
    }

    #[test]
    fn the_lexical_route_outranks_a_vector_only_miss() {
        let root = fixture("lexical");
        let store = HybridStore::open(&root, "fixture").expect("the store opens");
        // The query matches doc-a's text exactly; its vector is orthogonal.
        let hits = store.search_with("router selects a model", 2, Some(&[0.0, 1.0]));
        assert_eq!(hits.len(), 2, "both routes contribute");
        assert_eq!(
            hits[0].key, "doc-a",
            "the lexical match outranks the vector-only miss"
        );
        assert!(hits[0].lexical > 0.0, "the lexical route scored it");
        assert!(hits[1].vector > 0.0, "the vector route scored the other");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_fused_score_is_the_rrf_sum() {
        let root = fixture("fused");
        let store = HybridStore::open(&root, "fixture").expect("the store opens");
        let hits = store.search_with("the warm store", 2, Some(&[0.0, 1.0]));
        let top = hits
            .iter()
            .find(|hit| hit.key == "doc-b")
            .expect("doc-b is hit");
        // doc-b is the vector route's top (orthogonal to doc-a) and the lexical
        // route's top (it repeats "warm" and "store"), so it leads the fusion.
        assert!(
            hits[0].key == "doc-b",
            "the document both routes rank first wins"
        );
        assert!(top.score > 0.0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_changed_sha_rebuilds_the_lexical_cache() {
        let root = fixture("cache");
        let store_one = HybridStore::open(&root, "fixture").expect("the store opens");
        assert_eq!(store_one.len(), 2);
        let cache_path = root
            .join("fixture")
            .join("embeddinggemma-2")
            .join(".lexical-index.json");
        let first = std::fs::read_to_string(&cache_path).expect("the cache is written");
        // Change an entry's text and hash; the cache must not be reused.
        let manifest_path = root
            .join("fixture")
            .join("embeddinggemma-2")
            .join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["entries"][0]["sha256"] = serde_json::json!("cc");
        manifest["entries"][0]["text"] = serde_json::json!("the photosynth pathway");
        std::fs::write(&manifest_path, manifest.to_string()).unwrap();
        let store_two = HybridStore::open(&root, "fixture").expect("the store reopens");
        let second = std::fs::read_to_string(&cache_path).expect("the cache is rewritten");
        assert_ne!(first, second, "a changed entry rewrites the cache");
        let hits = store_two.search_with("photosynth", 1, None);
        assert_eq!(hits[0].key, "doc-a", "the rebuilt index sees the new text");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_hybrid_search_retrieves_a_symbol_by_its_signature() {
        let root = fixture("outline");
        let workspace = root.join("src");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(
            workspace.join("engine.rs"),
            "pub fn resolve_query_signature(query: &str) -> Vec<Hit> { todo!() }\n",
        )
        .unwrap();
        let store =
            HybridStore::open_with_outlines(&root, "fixture", &workspace).expect("the store opens");
        let hits = store.search_with("resolve_query_signature", 1, None);
        assert_eq!(hits.len(), 1, "the outline route answers: {hits:?}");
        assert!(
            hits[0].key.contains("engine.rs"),
            "the file is the hit: {:?}",
            hits[0].key
        );
        assert!(
            hits[0].text.contains("resolve_query_signature"),
            "the retrievable text carries the signature: {:?}",
            hits[0].text
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_store_is_named() {
        let root =
            std::env::temp_dir().join(format!("fleet-hybrid-missing-{}", std::process::id()));
        let error = HybridStore::open(&root, "absent").expect_err("the store is absent");
        assert!(
            error.contains("no store for this project"),
            "the absence is named: {error}"
        );
    }
}
