//! The design canvas's document and its transforms.
//!
//! One project is one JSON document at `<home>/canvas/<slug>/project.json`
//! — the design-canvas contract ported: an ordered array of frames, where
//! **array order is z-order** (frames render in array order), each frame an
//! opaque self-contained HTML page positioned on the board, with its token
//! tweaks, its comment pins and its version snapshots. The document is the
//! home's, and this module is its only writer — the face is a client of
//! it, never a second writer.
//!
//! The transforms here are the part that must agree byte-for-byte
//! wherever they are used: `inject_tweaks` is the one sanctioned frame
//! mutation (the live `srcdoc`, the preview route, the ZIP's HTML and the
//! screenshot's source all run it), `extract_root_tokens` feeds the token
//! exports, and `a11y_scan` is a scanner over the same HTML — never a
//! parser, so no HTML crate is added (the source's own scan is
//! scanner-shaped: regex color parsing over the extracted `:root` values
//! plus structural checks; its fidelity is a flag, not a certification).
//!
//! Caps, named: 512 KiB of HTML per frame, 20 versions per frame, 10
//! reviews per frame. A document past what the reader accepts is refused
//! by name, never silently dropped.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The canvas root under the fleet home.
pub const CANVAS_DIR: &str = "canvas";

/// The most HTML one frame may carry. A generated page is a few KB; the
/// cap is a refusal against a runaway generation, not a budget.
pub const MAX_FRAME_BYTES: usize = 512 * 1024;

/// The most version snapshots one frame keeps; the oldest falls off.
pub const MAX_VERSIONS: usize = 20;

/// The brands' own directory under the canvas root. A brand is a board's
/// design system: tokens and a doctrine, chosen per board, and binding on
/// every page a run lands there.
pub const BRANDS_DIR: &str = "brands";

/// The most brands one home keeps.
pub const MAX_BRANDS: usize = 50;

/// The most tokens one brand carries.
pub const MAX_BRAND_TOKENS: usize = 32;

/// The most doctrine text one brand carries.
pub const MAX_BRAND_DOCTRINE: usize = 2_000;

/// The most reviews one frame keeps.
pub const MAX_REVIEWS: usize = 10;

/// The board's three viewport presets, in world units: desktop, tablet
/// portrait, phone portrait.
pub const VIEWPORT_PRESETS: &[(&str, f64, f64)] = &[
    ("desktop", 1280.0, 900.0),
    ("tablet", 768.0, 1024.0),
    ("phone", 390.0, 844.0),
];

// --- the types --------------------------------------------------------------

/// One token tweak a frame exposes: what the panel renders and which CSS
/// variable it writes. Ported from the source's `TweakSpec`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TweakSpec {
    /// The value's key in `tweak_values` and the default's identity.
    pub key: String,
    pub label: String,
    /// `slider` | `select` | `toggle` | `color` — the control the panel
    /// renders. An unknown kind renders as a slider. The source's own
    /// spelling (`type`) is accepted beside this one, and so is the
    /// camelCase `cssVar` below: a frame written by either dialect lands.
    #[serde(alias = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default)]
    pub default: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    /// The CSS variable without its `--`, e.g. `accent`.
    #[serde(alias = "cssVar")]
    pub css_var: String,
}

/// One version snapshot: what the frame's content was, when.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub name: String,
    pub html: String,
    #[serde(default)]
    pub tweak_values: BTreeMap<String, Value>,
    /// `generate` | `refine` | `restore`.
    pub source: String,
    pub created: String,
}

/// One comment pin, at a normalized position over the frame (0–1 of the
/// frame's own size, so a viewport switch never moves it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub text: String,
    /// `owner` | `client` — one machine has one owner; the field keeps the
    /// contract the source established.
    #[serde(default)]
    pub source: String,
    pub created: String,
    /// The frame a fix made for this pin, when one has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_frame_id: Option<String>,
    #[serde(default)]
    pub replies: Vec<Reply>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reply {
    pub author: String,
    pub body: String,
    pub created: String,
}

/// One frame: an opaque HTML page on the board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub html: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default, alias = "tweakSpec")]
    pub tweak_spec: Vec<TweakSpec>,
    #[serde(default, alias = "tweakValues")]
    pub tweak_values: BTreeMap<String, Value>,
    /// The model that generated it, when one did. `import` for a frame
    /// brought in from outside.
    #[serde(default)]
    pub model: String,
    /// The page set this frame belongs to, when it is one page of a site:
    /// the set's pages share a token system, a navigation and a packet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "genMeta")]
    pub gen_meta: Option<Value>,
    #[serde(default)]
    pub versions: Vec<Version>,
    #[serde(default)]
    pub pins: Vec<Pin>,
    /// The last review of this frame, when one ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<Value>,
    pub created: String,
    pub updated: String,
}

/// One turn of the board's conversation: what was asked, what came back,
/// and the frames it landed. The board's chat is the document's — it
/// survives a reload, and the runner (the CLI, a cron job) can read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatTurn {
    /// `owner` | `designer` — the two speakers a board has.
    pub role: String,
    pub text: String,
    /// The frames this turn landed, by id.
    #[serde(default)]
    pub frames: Vec<String>,
    pub at: String,
}

/// The most turns a board's chat keeps; the oldest fall off.
pub const MAX_CHAT_TURNS: usize = 200;

/// One project: the board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub created: String,
    pub updated: String,
    pub frames: Vec<Frame>,
    /// The board's conversation, oldest first.
    #[serde(default)]
    pub chat: Vec<ChatTurn>,
    /// The brand the board designs under, by slug: its tokens are the
    /// default tweak values of every frame a run lands here.
    #[serde(default)]
    pub brand: Option<String>,
    /// The reference site this board was scanned against, cached: the
    /// facts are Rust's, and every surface (the composer, the CLI, a cron
    /// job) reads the same ones.
    #[serde(default)]
    pub reference: Option<BoardReference>,
    /// The project folder this board belongs to — an absolute path, when
    /// one was named. A board is project knowledge (the owner's
    /// 2026-11-01 ruling): the binding feeds the rail's grouping, the
    /// digest's path, the corpus-refresh glob, and the fact the
    /// lifecycle writes. None is the unbound board, kept visible.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "projectPath"
    )]
    pub project: Option<String>,
}

/// A scanned reference site, as the board keeps it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardReference {
    pub url: String,
    /// The fact block one run reads.
    pub block: String,
    /// The site's own pages, as the scan found them.
    #[serde(default)]
    pub pages: Vec<String>,
    pub fetched_at: String,
}

/// A project as the pickers list it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProjectSummary {
    pub slug: String,
    pub name: String,
    pub frames: usize,
    pub updated: String,
    /// The bound project folder, when the board carries one — the rail
    /// groups by it without a second read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

// --- the store --------------------------------------------------------------

pub fn canvas_root(home: &Path) -> PathBuf {
    home.join(CANVAS_DIR)
}

/// The project's directory name from a human name: lowercase words joined
/// by dashes, digits kept, everything else folded away.
pub fn slug_of(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let slug = out.trim_matches('-').to_string();
    if slug.is_empty() {
        "untitled".to_string()
    } else {
        slug.chars().take(48).collect()
    }
}

fn project_path(home: &Path, slug: &str) -> PathBuf {
    canvas_root(home).join(slug).join("project.json")
}

pub fn exists(home: &Path, slug: &str) -> bool {
    project_path(home, slug).is_file()
}

/// Every project under the home, newest first.
pub fn projects(home: &Path) -> Vec<ProjectSummary> {
    let root = canvas_root(home);
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut out: Vec<ProjectSummary> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let slug = entry.file_name().to_string_lossy().to_string();
            // `canvas/brands/` holds the design systems, not a board.
            if slug == BRANDS_DIR {
                return None;
            }
            let project = load(home, &slug).ok()?;
            Some(ProjectSummary {
                slug,
                name: project.name,
                frames: project.frames.len(),
                updated: project.updated,
                project: project.project,
            })
        })
        .collect();
    out.sort_by(|a, b| b.updated.cmp(&a.updated));
    out
}

/// Read one project. A missing or unreadable document is a named error —
/// the pickers never invent a board.
pub fn load(home: &Path, slug: &str) -> Result<Project, String> {
    let path = project_path(home, slug);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("the project `{slug}` could not be read: {error}"))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("the project `{slug}` is not a canvas document: {error}"))
}

/// Write one project, atomically: a unique temp beside it, then a rename,
/// so a reader never sees half a document. A bound board's digest is
/// regenerated here — the one funnel every write path crosses, so no
/// path can move the document without refreshing its knowledge account.
pub fn save(home: &Path, slug: &str, project: &Project) -> Result<(), String> {
    let path = project_path(home, slug);
    let dir = path
        .parent()
        .ok_or_else(|| "the project path has no directory".to_string())?;
    std::fs::create_dir_all(dir)
        .map_err(|error| format!("could not make {}: {error}", dir.display()))?;
    let text = serde_json::to_string_pretty(project)
        .map_err(|error| format!("the project does not serialize: {error}"))?;
    crate::settings::write_atomic(&path, &text)?;
    refresh_digest(home, slug, project)
}

// --- the digest --------------------------------------------------------------
//
// A board is project knowledge (the owner's 2026-11-01 ruling): every
// bound board keeps a deterministic markdown account of itself at a
// project-keyed path — the unit the corpus-refresh globs embed. The
// pages' HTML is the artifact; the digest is the knowledge.

/// The directory of digests, under the canvas root: one subdirectory per
/// project key, one file per board.
pub const DIGESTS_DIR: &str = "digests";

/// The project key of a bound path: its final segment — the same name
/// the corpus-refresh jobs and the blend1 vector stores key by. A path
/// with no final segment (a root) is unkeyable, so the binding reads
/// as unbound.
pub fn project_key(project: &str) -> Option<String> {
    let trimmed = project.trim_end_matches('/');
    let name = Path::new(trimmed)
        .file_name()?
        .to_string_lossy()
        .to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Where a bound board's digest lives.
pub fn digest_path(home: &Path, key: &str, slug: &str) -> PathBuf {
    canvas_root(home)
        .join(DIGESTS_DIR)
        .join(key)
        .join(format!("{slug}.md"))
}

/// The digest's bounds: a board this large still embeds as one file.
const DIGEST_PAGES: usize = 40;
const DIGEST_TOKENS: usize = 60;
const DIGEST_REVIEWS: usize = 10;

/// The deterministic markdown account of a board. Same document, same
/// bytes — a refresh that re-embeds an unchanged board drops nothing.
pub fn digest_markdown(slug: &str, project: &Project) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", project.name));
    out.push_str(&format!("board: {slug}\n"));
    out.push_str(&format!(
        "project: {}\n",
        project.project.as_deref().unwrap_or("\u{2014}")
    ));
    out.push_str(&format!("updated: {}\n", project.updated));
    out.push_str(&format!(
        "brand: {}\n",
        project.brand.as_deref().unwrap_or("\u{2014}")
    ));
    out.push_str(&format!("pages: {}\n", project.frames.len()));

    out.push_str("\n## Pages\n\n");
    for frame in project.frames.iter().take(DIGEST_PAGES) {
        out.push_str(&format!(
            "- {} \u{b7} {}x{} \u{b7} model {} \u{b7} set {}\n",
            frame.name,
            frame.w as i64,
            frame.h as i64,
            if frame.model.is_empty() {
                "\u{2014}"
            } else {
                frame.model.as_str()
            },
            frame.set.as_deref().unwrap_or("\u{2014}")
        ));
    }
    if project.frames.len() > DIGEST_PAGES {
        out.push_str(&format!(
            "- \u{2026} {} more\n",
            project.frames.len() - DIGEST_PAGES
        ));
    }

    // The token system, as the pages declare it: the union of every
    // page's `:root` custom-property names, sorted — the same sheet the
    // exports and the packet read.
    let mut tokens: Vec<String> = Vec::new();
    for frame in &project.frames {
        for (key, _) in extract_root_tokens(&frame.html) {
            if !tokens.contains(&key) {
                tokens.push(key);
            }
        }
    }
    tokens.sort();
    tokens.truncate(DIGEST_TOKENS);
    out.push_str("\n## Tokens\n\n");
    if tokens.is_empty() {
        out.push_str("\u{2014}\n");
    } else {
        out.push_str(&tokens.join(", "));
        out.push('\n');
    }

    // The reviews the pages carry: the eight scores, in name order. A
    // page without a review is not listed.
    let mut reviews = Vec::new();
    for frame in &project.frames {
        let Some(review) = frame.review.as_ref() else {
            continue;
        };
        let Some(scores) = review.get("scores").and_then(Value::as_object) else {
            continue;
        };
        if scores.is_empty() {
            continue;
        }
        let mut parts: Vec<String> = scores
            .iter()
            .map(|(name, score)| {
                let number = score.as_f64().unwrap_or(f64::NAN);
                format!("{name} {}", number as i64)
            })
            .collect();
        parts.sort();
        reviews.push(format!("- {}: {}\n", frame.name, parts.join(", ")));
        if reviews.len() >= DIGEST_REVIEWS {
            break;
        }
    }
    if !reviews.is_empty() {
        out.push_str("\n## Reviews\n\n");
        for line in reviews {
            out.push_str(&line);
        }
    }
    out
}

/// Regenerate a bound board's digest beside its document; an unbound
/// board keeps no digest (there is no key to file it under).
fn refresh_digest(home: &Path, slug: &str, project: &Project) -> Result<(), String> {
    let Some(key) = project.project.as_deref().and_then(project_key) else {
        return Ok(());
    };
    let path = digest_path(home, &key, slug);
    let dir = path
        .parent()
        .ok_or_else(|| "the digest path has no directory".to_string())?;
    std::fs::create_dir_all(dir)
        .map_err(|error| format!("could not make {}: {error}", dir.display()))?;
    crate::settings::write_atomic(&path, &digest_markdown(slug, project))
}

/// Remove one board's digest at the key the caller names — the clearing
/// op and the delete path know the old binding; `save` never guesses.
pub fn remove_digest(home: &Path, key: &str, slug: &str) -> Result<(), String> {
    let path = digest_path(home, key, slug);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "could not remove the digest {}: {error}",
            path.display()
        )),
    }
}

/// K6 (designs-are-project-knowledge): the bounded design account a run
/// born in a project carries in its assembled context — one line per
/// board from the project's digests, newest first, absent when the
/// project has none. The digest is the knowledge; this is its read.
pub fn design_context_section(home: &Path, key: &str) -> Option<String> {
    let dir = canvas_root(home).join(DIGESTS_DIR).join(key);
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut boards: Vec<(std::time::SystemTime, String, String, usize)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()?.to_str()? != "md" {
                return None;
            }
            let text = std::fs::read_to_string(&path).ok()?;
            let name = text
                .lines()
                .find_map(|line| line.strip_prefix("# "))?
                .to_string();
            let pages = text
                .lines()
                .find_map(|line| line.strip_prefix("pages: "))?
                .trim()
                .parse::<usize>()
                .ok()?;
            let stamp = entry.metadata().ok()?.modified().ok()?;
            Some((
                stamp,
                name,
                path.file_stem()?.to_string_lossy().to_string(),
                pages,
            ))
        })
        .collect();
    if boards.is_empty() {
        return None;
    }
    boards.sort_by(|a, b| b.0.cmp(&a.0));
    boards.truncate(8);
    let mut out = format!(
        "Design boards bound to `{key}` (from their digests; the board file is the truth):\n"
    );
    for (_, name, slug, pages) in boards {
        out.push_str(&format!(
            "- {name} ({slug}) \u{2014} {pages} page{}\n",
            if pages == 1 { "" } else { "s" }
        ));
    }
    Some(out)
}

/// Delete a board whole: its directory (frames, assets, exports,
/// reviews and versions ride along) and its digest at the key the
/// binding names. A board that is not there is a named error — a
/// delete is exact, never a silent no-op.
pub fn delete_project(home: &Path, slug: &str) -> Result<(), String> {
    let project = load(home, slug)?;
    if let Some(key) = project.project.as_deref().and_then(project_key) {
        remove_digest(home, &key, slug)?;
    }
    let dir = canvas_root(home).join(slug);
    std::fs::remove_dir_all(&dir)
        .map_err(|error| format!("the board `{slug}` could not be removed: {error}"))
}

/// Create an empty project under a fresh slug. A name that collides takes
/// a numeric suffix so a second board never overwrites the first.
pub fn create(home: &Path, name: &str) -> Result<(String, Project), String> {
    let base = slug_of(name);
    let mut slug = base.clone();
    let mut n = 2;
    while exists(home, &slug) {
        slug = format!("{base}-{n}");
        n += 1;
    }
    let now = crate::calib::stamp();
    let project = Project {
        version: 1,
        name: if name.trim().is_empty() {
            "Untitled board".to_string()
        } else {
            name.trim().to_string()
        },
        created: now.clone(),
        updated: now,
        frames: Vec::new(),
        chat: Vec::new(),
        brand: None,
        reference: None,
        project: None,
    };
    save(home, &slug, &project)?;
    Ok((slug, project))
}

// --- brands -----------------------------------------------------------------

/// One brand: the board's design system. Tokens are `:root` custom
/// property names and values (the same names a frame's tweak spec writes),
/// and the doctrine is the sentence or three a designer reads as
/// standing instruction.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Brand {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub tokens: BTreeMap<String, String>,
    #[serde(default)]
    pub doctrine: String,
    pub created: String,
    pub updated: String,
}

pub fn brands_dir(home: &Path) -> PathBuf {
    canvas_root(home).join(BRANDS_DIR)
}

fn brand_path(home: &Path, slug: &str) -> PathBuf {
    brands_dir(home).join(format!("{slug}.json"))
}

/// Every brand this home keeps, by name.
pub fn brands(home: &Path) -> Vec<Brand> {
    let Ok(entries) = std::fs::read_dir(brands_dir(home)) else {
        return Vec::new();
    };
    let mut out: Vec<Brand> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| {
            let text = std::fs::read_to_string(entry.path()).ok()?;
            serde_json::from_str(&text).ok()
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

pub fn load_brand(home: &Path, slug: &str) -> Result<Brand, String> {
    let path = brand_path(home, slug);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("the brand `{slug}` could not be read: {error}"))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("the brand `{slug}` is not a brand: {error}"))
}

/// Write one brand, creating or replacing it by slug. A token is a custom
/// property name and its value; the caps are refusals by name, never a
/// silent trim.
pub fn put_brand(
    home: &Path,
    name: &str,
    tokens: BTreeMap<String, String>,
    doctrine: &str,
) -> Result<Brand, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("the brand names nothing".to_string());
    }
    let slug = slug_of(name);
    let known = brands(home);
    if !known.iter().any(|brand| brand.slug == slug) && known.len() >= MAX_BRANDS {
        return Err(format!(
            "this home keeps {MAX_BRANDS} brands already — drop one before adding another"
        ));
    }
    if tokens.len() > MAX_BRAND_TOKENS {
        return Err(format!(
            "the brand carries {} tokens; the cap is {MAX_BRAND_TOKENS}",
            tokens.len()
        ));
    }
    for (token, value) in &tokens {
        let key = token.trim_start_matches("--");
        if key.is_empty() || key.len() > 40 || key.contains(char::is_whitespace) {
            return Err(format!("`{token}` is not a custom property name"));
        }
        if value.len() > 120 || value.contains(';') || value.contains('}') {
            return Err(format!(
                "the value of `{key}` is not a token value (120 chars, no `;` or `}}`)"
            ));
        }
    }
    if doctrine.len() > MAX_BRAND_DOCTRINE {
        return Err(format!(
            "the doctrine is {} chars; the cap is {MAX_BRAND_DOCTRINE}",
            doctrine.len()
        ));
    }
    let now = crate::calib::stamp();
    let created = known
        .iter()
        .find(|brand| brand.slug == slug)
        .map(|brand| brand.created.clone())
        .unwrap_or_else(|| now.clone());
    let brand = Brand {
        slug: slug.clone(),
        name: name.to_string(),
        tokens: tokens
            .into_iter()
            .map(|(token, value)| (token.trim_start_matches("--").to_string(), value))
            .collect(),
        doctrine: doctrine.trim().to_string(),
        created,
        updated: now,
    };
    let dir = brands_dir(home);
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("could not make {}: {error}", dir.display()))?;
    let text = serde_json::to_string_pretty(&brand)
        .map_err(|error| format!("the brand does not serialize: {error}"))?;
    crate::settings::write_atomic(&brand_path(home, &slug), &text)?;
    Ok(brand)
}

/// Drop one brand. A board that had chosen it keeps the choice recorded
/// but the choice no longer resolves, which the read names.
pub fn delete_brand(home: &Path, slug: &str) -> Result<(), String> {
    let path = brand_path(home, slug);
    if !path.is_file() {
        return Err(format!("no brand `{slug}`"));
    }
    std::fs::remove_file(&path)
        .map_err(|error| format!("could not drop {}: {error}", path.display()))
}

/// Record the board's reference scan — or clear it with `None`.
pub fn set_reference(
    home: &Path,
    slug: &str,
    reference: Option<BoardReference>,
) -> Result<(), String> {
    let mut project = load(home, slug)?;
    project.reference = reference;
    project.updated = crate::calib::stamp();
    save(home, slug, &project)
}

/// The board's assets: files (images, an inlined SVG) that a page refers
/// to by name and the canvas inlines as `data:` URIs, so a frame stays one
/// self-contained page.
pub const ASSETS_DIR: &str = "assets";

/// The most bytes one asset may carry. The frame cap is twice this, so a
/// page can carry several real images and still be a page.
pub const MAX_ASSET_BYTES: usize = 256 * 1024;

/// The most assets one board keeps.
pub const MAX_ASSETS: usize = 64;

/// One board asset, as the manifest records it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    /// The placeholder name the page writes: `{{img:NAME}}`.
    pub name: String,
    /// The file under the board's `assets/` directory.
    pub file: String,
    #[serde(default = "asset_mime_default")]
    pub mime: String,
    pub bytes: usize,
    pub created: String,
}

fn asset_mime_default() -> String {
    "application/octet-stream".to_string()
}

pub fn assets_dir(home: &Path, slug: &str) -> PathBuf {
    canvas_root(home).join(slug).join(ASSETS_DIR)
}

fn assets_manifest(home: &Path, slug: &str) -> PathBuf {
    assets_dir(home, slug).join("manifest.json")
}

/// Every asset the board keeps, by name.
pub fn assets(home: &Path, slug: &str) -> Vec<Asset> {
    let Ok(text) = std::fs::read_to_string(assets_manifest(home, slug)) else {
        return Vec::new();
    };
    let mut out: Vec<Asset> = serde_json::from_str(&text).unwrap_or_default();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn save_assets(home: &Path, slug: &str, list: &[Asset]) -> Result<(), String> {
    let dir = assets_dir(home, slug);
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("could not make {}: {error}", dir.display()))?;
    let text = serde_json::to_string_pretty(list)
        .map_err(|error| format!("the asset manifest does not serialize: {error}"))?;
    crate::settings::write_atomic(&assets_manifest(home, slug), &text)
}

/// The MIME one asset's bytes are, sniffed from the bytes themselves: an
/// extension is a claim, the magic is the fact. SVG is text and is
/// recognized by its own root element.
pub fn asset_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Some("image/png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF8") {
        return Some("image/gif");
    }
    if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]).to_ascii_lowercase();
    if head.contains("<svg") {
        return Some("image/svg+xml");
    }
    None
}

/// Store one asset on a board. The name is the placeholder the page
/// writes, the file is written beside the manifest, and a repeated name
/// replaces in place. Caps by name: the byte cap, the count, the MIME —
/// an asset the canvas cannot inline is not an asset.
pub fn put_asset(
    home: &Path,
    slug: &str,
    name: &str,
    bytes: &[u8],
    mime_hint: Option<&str>,
) -> Result<Asset, String> {
    if !exists(home, slug) {
        return Err(format!("no canvas board `{slug}`"));
    }
    let name = slug_of(name);
    if name.is_empty() || name == "untitled" {
        return Err("the asset names nothing; give it a name the page can write".to_string());
    }
    if bytes.is_empty() {
        return Err(format!("the asset `{name}` is empty"));
    }
    if bytes.len() > MAX_ASSET_BYTES {
        return Err(format!(
            "the asset `{name}` is {} bytes; the cap is {MAX_ASSET_BYTES}",
            bytes.len()
        ));
    }
    let sniffed = asset_mime(bytes).ok_or_else(|| {
        format!(
            "the asset `{name}` is not an image the canvas inlines (png, jpeg, gif, webp or svg)"
        )
    })?;
    if let Some(hint) = mime_hint.filter(|hint| !hint.trim().is_empty()) {
        let hint = hint.trim();
        // A hint may only agree with the bytes: the sniff is the truth.
        if !hint.starts_with(sniffed) && !sniffed.starts_with(hint) {
            return Err(format!(
                "the asset `{name}` declares `{hint}` but its bytes are `{sniffed}`"
            ));
        }
    }
    let bytes = if sniffed == "image/svg+xml" {
        let text = String::from_utf8(bytes.to_vec())
            .map_err(|_| format!("the svg asset `{name}` is not UTF-8 text"))?;
        svg_sanitize(&text)?.into_bytes()
    } else {
        bytes.to_vec()
    };
    if bytes.len() > MAX_ASSET_BYTES {
        return Err(format!(
            "the asset `{name}` is {} bytes after cleaning; the cap is {MAX_ASSET_BYTES}",
            bytes.len()
        ));
    }
    let mut list = assets(home, slug);
    let ext = match sniffed {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "svg",
    };
    let file = format!("{name}.{ext}");
    if !list.iter().any(|asset| asset.name == name) && list.len() >= MAX_ASSETS {
        return Err(format!(
            "the board keeps {MAX_ASSETS} assets already — drop one before adding another"
        ));
    }
    let dir = assets_dir(home, slug);
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("could not make {}: {error}", dir.display()))?;
    std::fs::write(dir.join(&file), &bytes)
        .map_err(|error| format!("could not write the asset `{name}`: {error}"))?;
    let now = crate::calib::stamp();
    let created = list
        .iter()
        .find(|asset| asset.name == name)
        .map(|asset| asset.created.clone())
        .unwrap_or_else(|| now.clone());
    // A replacement with a different extension leaves no orphan behind.
    for old in list
        .iter()
        .filter(|asset| asset.name == name && asset.file != file)
    {
        let _ = std::fs::remove_file(dir.join(&old.file));
    }
    let asset = Asset {
        name: name.clone(),
        file,
        mime: sniffed.to_string(),
        bytes: bytes.len(),
        created,
    };
    list.retain(|entry| entry.name != name);
    list.push(asset.clone());
    list.sort_by(|a, b| a.name.cmp(&b.name));
    save_assets(home, slug, &list)?;
    Ok(asset)
}

/// One asset's bytes, read back through the manifest.
pub fn asset_bytes(home: &Path, slug: &str, name: &str) -> Result<(Asset, Vec<u8>), String> {
    let name = slug_of(name);
    let asset = assets(home, slug)
        .into_iter()
        .find(|asset| asset.name == name)
        .ok_or_else(|| format!("no asset `{name}` on board `{slug}`"))?;
    let path = assets_dir(home, slug).join(&asset.file);
    let bytes = std::fs::read(&path)
        .map_err(|error| format!("the asset `{name}` could not be read: {error}"))?;
    Ok((asset, bytes))
}

/// One asset as the `data:` URI a page embeds — the single inlining path,
/// so the preview, the export and the model's own page agree.
pub fn asset_data_uri(home: &Path, slug: &str, name: &str) -> Result<String, String> {
    let (asset, bytes) = asset_bytes(home, slug, name)?;
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", asset.mime, encoded))
}

pub fn delete_asset(home: &Path, slug: &str, name: &str) -> Result<(), String> {
    let name = slug_of(name);
    let mut list = assets(home, slug);
    let Some(asset) = list.iter().find(|asset| asset.name == name).cloned() else {
        return Err(format!("no asset `{name}` on board `{slug}`"));
    };
    list.retain(|entry| entry.name != name);
    let dir = assets_dir(home, slug);
    std::fs::remove_file(dir.join(&asset.file))
        .map_err(|error| format!("could not drop the asset `{name}`: {error}"))?;
    save_assets(home, slug, &list)
}

/// Make an SVG safe to inline: the active elements go, event handlers go,
/// and external or script targets go — the file still renders, it just
/// cannot act or reach out. A design asset is a picture; the cleaning is a
/// scanner over tags and attributes, the same rule the a11y scan follows.
pub fn svg_sanitize(svg: &str) -> Result<String, String> {
    let lower = svg.to_ascii_lowercase();
    if !lower.contains("<svg") {
        return Err("the file is not an SVG (no <svg element)".to_string());
    }
    let mut out = svg.to_string();
    for (open, close) in [
        ("<script", "</script>"),
        ("<foreignobject", "</foreignobject>"),
        ("<iframe", "</iframe>"),
        ("<object", "</object>"),
        ("<embed", "</embed>"),
    ] {
        loop {
            let lower_out = out.to_ascii_lowercase();
            let Some(start) = lower_out.find(open) else {
                break;
            };
            let end = lower_out[start..]
                .find(close)
                .map(|at| start + at + close.len())
                .unwrap_or(out.len());
            out.replace_range(start..end, "");
        }
    }
    Ok(strip_attributes(&out))
}

/// Strip the attributes an asset must not carry: an `on<event>=` handler,
/// or a `href`/`src` that reaches outside the document. Everything else is
/// copied byte for byte, so the picture is unchanged.
fn strip_attributes(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(lt) = rest.find('<') {
        out.push_str(&rest[..lt]);
        let after = &rest[lt..];
        // A tag ends at the first `>` outside quotes: a `d="a > b"` path
        // must not be cut in half.
        let mut quote: Option<char> = None;
        let mut end = None;
        for (at, ch) in after.char_indices() {
            match quote {
                Some(mark) if ch == mark => quote = None,
                Some(_) => {}
                None if ch == '"' || ch == '\'' => quote = Some(ch),
                None if ch == '>' => {
                    end = Some(at);
                    break;
                }
                None => {}
            }
        }
        let Some(end) = end else {
            out.push_str(after);
            return out;
        };
        let tag = &after[..=end];
        out.push_str(&clean_tag(tag));
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// One tag with its forbidden attributes removed. The tag name, the
/// whitespace between attributes and every allowed attribute survive as
/// written.
fn clean_tag(tag: &str) -> String {
    let mut out = String::with_capacity(tag.len());
    let mut at = 0;
    let bytes = tag.as_bytes();
    // The name (and the leading `<`, `/` or `!`) copies verbatim.
    while at < bytes.len() {
        let ch = bytes[at];
        if ch.is_ascii_whitespace() || ch == b'>' || ch == b'/' {
            break;
        }
        at += 1;
    }
    out.push_str(&tag[..at]);
    while at < bytes.len() {
        // Whitespace before an attribute.
        let space = at;
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        let gap = &tag[space..at];
        if at >= bytes.len() || bytes[at] == b'>' || bytes[at] == b'/' {
            out.push_str(gap);
            out.push_str(&tag[at..]);
            return out;
        }
        // The name.
        let name_start = at;
        while at < bytes.len() {
            let ch = bytes[at];
            if ch.is_ascii_alphanumeric()
                || ch == b' '
                || ch == b':'
                || ch == b'-'
                || ch == b'_'
                || ch == b'.'
            {
                at += 1;
            } else {
                break;
            }
        }
        let name = tag[name_start..at].trim().to_ascii_lowercase();
        // The value, if one follows the `=`.
        let mut value_start = at;
        while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
            value_start += 1;
        }
        let mut value = "";
        let mut value_end = at;
        if value_start < bytes.len() && bytes[value_start] == b'=' {
            let mut cursor = value_start + 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && (bytes[cursor] == b'"' || bytes[cursor] == b'\'') {
                let mark = bytes[cursor];
                let start = cursor + 1;
                let mut close = start;
                while close < bytes.len() && bytes[close] != mark {
                    close += 1;
                }
                value = &tag[start..close.min(tag.len())];
                value_end = (close + 1).min(tag.len());
            } else {
                let start = cursor;
                let mut close = start;
                while close < bytes.len()
                    && !bytes[close].is_ascii_whitespace()
                    && bytes[close] != b'>'
                {
                    close += 1;
                }
                value = &tag[start..close];
                value_end = close;
            }
        }
        let handler = name.starts_with("on") && name.len() > 2;
        let external = matches!(name.as_str(), "href" | "src" | "xlink:href") && {
            let lower = value.trim().to_ascii_lowercase();
            lower.starts_with("http:")
                || lower.starts_with("https:")
                || lower.starts_with("//")
                || lower.starts_with("javascript:")
                || lower.starts_with("data:text/html")
        };
        if !handler && !external {
            let end = if value.is_empty() { at } else { value_end };
            out.push_str(gap);
            out.push_str(&tag[name_start..end]);
            at = end;
            // A value-less attribute keeps its own name only.
            if value.is_empty() && at == name_start {
                at += 1;
            }
        } else {
            at = if value.is_empty() { at } else { value_end };
        }
    }
    out.push_str(&tag[at.min(tag.len())..]);
    out
}

/// Inline a page's `{{img:NAME}}` placeholders. Every placeholder must
/// resolve: a page that names an asset the board does not keep is refused
/// by name, because the alternative is a page that renders a literal
/// `{{img:…}}` to the owner.
pub fn substitute_assets(
    html: &str,
    resolved: &BTreeMap<String, String>,
) -> Result<String, String> {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("{{img:") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "{{img:".len()..];
        let Some(end) = after.find("}}") else {
            return Err("a `{{img:` placeholder never closes with `}}`".to_string());
        };
        let name = slug_of(after[..end].trim());
        let uri = resolved
            .get(&name)
            .ok_or_else(|| format!("the page names an asset `{name}` the board does not keep"))?;
        out.push_str(uri);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// One asset landed as a page: an SVG wrapped in a document that centres
/// it, so an imported mark is a frame like any other.
pub fn frame_from_svg(home: &Path, slug: &str, name: &str) -> Result<String, String> {
    let (asset, bytes) = asset_bytes(home, slug, name)?;
    if asset.mime != "image/svg+xml" {
        return Err(format!(
            "`{}` is {}; only an svg becomes a page",
            asset.name, asset.mime
        ));
    }
    let svg = String::from_utf8(bytes)
        .map_err(|_| format!("the svg asset `{}` is not UTF-8 text", asset.name))?;
    let page = format!(
        "<!doctype html><html><head><meta name=\"viewport\" content=\"width=device-width\"><style>:root{{--page:#ffffff;--ink:#1c1a17}}html,body{{margin:0;height:100%;display:grid;place-items:center;background:var(--page);color:var(--ink)}}svg{{max-width:100%;max-height:100%}}</style></head><body>{svg}</body></html>"
    );
    put_frame(home, slug, &asset.name, &page, None, None, "import")
}

/// Choose the board's brand — or clear it with `None`.
pub fn set_brand(home: &Path, slug: &str, brand: Option<&str>) -> Result<(), String> {
    let mut project = load(home, slug)?;
    if let Some(brand) = brand {
        // The choice must resolve: a board pointing at a brand this home
        // does not keep is a board whose tokens were never real.
        load_brand(home, brand)?;
        project.brand = Some(brand.to_string());
    } else {
        project.brand = None;
    }
    project.updated = crate::calib::stamp();
    save(home, slug, &project)
}

/// Adopt the sample board on a home that has no projects yet: the two
/// proven-valid frames from the source's fixtures, so a fresh canvas has
/// something real on it.
pub fn ensure_sample(home: &Path) -> Result<(), String> {
    if projects(home).is_empty() {
        let text = include_str!("gui/assets/design-sample.json");
        let project: Project = serde_json::from_str(text)
            .map_err(|error| format!("the bundled sample board is not a document: {error}"))?;
        save(home, &slug_of(&project.name.clone()), &project)?;
    }
    Ok(())
}

// --- the board's conversation ----------------------------------------------

/// Append one turn to a board's chat, capping the list. The frames a
/// turn landed ride it, so a chip survives the reload without a second
/// index.
pub fn append_chat(
    home: &Path,
    slug: &str,
    role: &str,
    text: &str,
    frames: &[String],
) -> Result<ChatTurn, String> {
    let mut project = load(home, slug)?;
    let turn = ChatTurn {
        role: role.trim().to_string(),
        text: text.trim().to_string(),
        frames: frames.to_vec(),
        at: crate::calib::stamp(),
    };
    project.chat.push(turn.clone());
    while project.chat.len() > MAX_CHAT_TURNS {
        project.chat.remove(0);
    }
    project.updated = turn.at.clone();
    save(home, slug, &project)?;
    Ok(turn)
}

/// What one design request is for. The mode is the composer's verb, and
/// it is what decides which instruction the task carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignMode {
    /// A question about the board; no frame is expected.
    Ask,
    /// One or more new frames from the brief.
    Generate,
    /// A new version of an existing frame.
    Refine,
    /// A fresh frame in a selected frame's spirit.
    Regenerate,
    /// A review's fixes applied to its frame.
    Fixes,
    /// A multi-page site from the brief.
    PageSet,
    /// A site modelled on a reference URL's facts.
    Redesign,
}

impl DesignMode {
    pub fn parse(word: &str) -> Option<DesignMode> {
        Some(match word.trim().to_ascii_lowercase().as_str() {
            "ask" => DesignMode::Ask,
            "generate" => DesignMode::Generate,
            "refine" => DesignMode::Refine,
            "regenerate" => DesignMode::Regenerate,
            "fixes" => DesignMode::Fixes,
            "page_set" => DesignMode::PageSet,
            "redesign" => DesignMode::Redesign,
            _ => return None,
        })
    }
}

/// The models designated for design work (the owner's ruling, 2026-10-31).
/// A page asks a different question of a model than code or prose does, so
/// the design surface carries its own set instead of borrowing a routing
/// chain: the page's picker names one per run and the run goes out pinned
/// to exactly that `provider/model`. The router neither picks among these
/// nor walks them as a workload — the fact that a run started here is what
/// makes its route a design route.
pub const DESIGN_MODELS: &[(&str, &str)] = &[
    ("deepseek", "deepseek-flash"),
    ("zai-coding-plan", "GLM-5.3-Flash"),
    // The same GLM on Ollama's cloud account: a different host for the
    // family, and the owner's fallback when zai's GLM is unavailable.
    ("ollama-cloud", "glm-5.3-flash"),
];

/// The style preset one request chose, split where the library splits it:
/// the spec is the positive prompt, the avoid list the negative one. Both
/// ride the task; the library is data (`assets/design-presets.json`), so a
/// preset is always exactly what the source's authors wrote.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Preset {
    pub label: String,
    pub spec: String,
    pub avoid: String,
}

/// The facts one composed task may carry, each optional: the mode decides
/// which ride.
#[derive(Debug, Clone, Default)]
pub struct DesignFacts {
    /// The frame the request is about (refine, regenerate, fixes).
    pub frame: Option<Frame>,
    /// The pins open on that frame.
    pub pins: Vec<String>,
    /// The review stored on the frame, when one is.
    pub review: Option<Value>,
    /// The chosen style preset: its spec, and the avoid list that rides
    /// the task as the negative prompt.
    pub preset: Option<Preset>,
    /// The chosen brand's name, tokens and doctrine.
    pub brand: Option<(String, BTreeMap<String, String>, String)>,
    /// The brief's target frame size, decided by [`frame_size_for`].
    pub size: Option<(&'static str, f64, f64)>,
    /// How many directions the brief asked for.
    pub directions: usize,
    /// The reference site's style facts, when the request names a URL.
    pub reference: Option<String>,
    /// The board's page count target for a page set.
    pub pages: Option<usize>,
    /// The board's own inventory — every frame, one line each — so a
    /// question about the board is answered from the facts, not by
    /// exploring the console with tools.
    pub board: Vec<String>,
}

/// One token of a brief decides the target frame: a brief that says
/// mobile gets a phone canvas, a tablet one gets a tablet, and everything
/// else the desktop the board defaults to. Rust's rule, so the CLI and
/// the composer size a frame the same way.
pub fn frame_size_for(brief: &str) -> (&'static str, f64, f64) {
    let lower = brief.to_ascii_lowercase();
    if lower.contains("mobile") || lower.contains("phone") || lower.contains("app") {
        ("mobile", 390.0, 844.0)
    } else if lower.contains("tablet") || lower.contains("ipad") {
        ("tablet", 768.0, 1024.0)
    } else {
        ("desktop", 1280.0, 900.0)
    }
}

/// The size one viewport word means: the closed set the board's presets
/// and the brief's rule both speak. An unknown word is desktop.
pub fn sizing_for(viewport: &str) -> (&'static str, f64, f64) {
    match viewport.trim().to_ascii_lowercase().as_str() {
        "mobile" | "phone" => ("mobile", 390.0, 844.0),
        "tablet" => ("tablet", 768.0, 1024.0),
        _ => ("desktop", 1280.0, 900.0),
    }
}

/// The task one design request composes — the whole instruction, in Rust,
/// so every surface (the composer, the CLI, a cron job) asks for the same
/// thing in the same words. The doctrine itself lives in the designer
/// role; this is the request around it.
pub fn designer_task(mode: DesignMode, brief: &str, slug: &str, facts: &DesignFacts) -> String {
    let brief = brief.trim();
    let mut task = String::new();
    if !facts.board.is_empty() {
        task.push_str(&format!(
            "The board `{slug}` holds {} frame{}:\n",
            facts.board.len(),
            if facts.board.len() == 1 { "" } else { "s" }
        ));
        for line in &facts.board {
            task.push_str(&format!("- {line}\n"));
        }
        task.push('\n');
    }
    match mode {
        DesignMode::Ask => task.push_str(&format!("About the design board `{slug}`: {brief}")),
        DesignMode::Generate => task.push_str(&format!(
            "Design: {brief}\n\nPut the page on the board with the `frame` tool (project `{slug}`)."
        )),
        DesignMode::Refine => {
            let (name, id) = facts
                .frame
                .as_ref()
                .map(|frame| (frame.name.as_str(), frame.id.as_str()))
                .unwrap_or(("", ""));
            task.push_str(&format!(
                "Refine the design frame `{name}` (frame_id `{id}`) on board `{slug}`: {brief}\n\nCall the `frame` tool with project `{slug}` and frame_id `{id}` so the page keeps its identity and the previous page is kept as a version."
            ));
        }
        DesignMode::Regenerate => {
            let (name, id) = facts
                .frame
                .as_ref()
                .map(|frame| (frame.name.as_str(), frame.id.as_str()))
                .unwrap_or(("", ""));
            task.push_str(&format!(
                "Take a different direction from the design frame `{name}` (frame_id `{id}`) on board `{slug}`: {brief}\n\nPut a NEW frame on the board with the `frame` tool (project `{slug}`) \u{2014} do not refine the existing one; the owner wants an alternative beside it."
            ));
        }
        DesignMode::Fixes => {
            let (name, id) = facts
                .frame
                .as_ref()
                .map(|frame| (frame.name.as_str(), frame.id.as_str()))
                .unwrap_or(("", ""));
            let mut fixes: Vec<String> = Vec::new();
            if let Some(review) = &facts.review {
                if let Some(list) = review.get("fixes").and_then(|fixes| fixes.as_array()) {
                    for fix in list {
                        if let Some(fix) = fix.as_str() {
                            fixes.push(fix.to_string());
                        }
                    }
                }
            }
            task.push_str(&format!(
                "Apply the design review's fixes to the frame `{name}` (frame_id `{id}`) on board `{slug}`."
            ));
            if fixes.is_empty() {
                task.push_str(
                    "\n\nThe review named no fixes; read the page and repair what is weakest.",
                );
            } else {
                task.push_str("\n\nThe fixes the review named:");
                for fix in &fixes {
                    task.push_str(&format!("\n- {fix}"));
                }
            }
            task.push_str(&format!(
                "\n\nCall the `frame` tool with project `{slug}` and frame_id `{id}`, changing exactly what the fixes name."
            ));
            if !brief.is_empty() {
                task.push_str(&format!("\n\nThe owner adds: {brief}"));
            }
        }
        DesignMode::PageSet => {
            let pages = facts.pages.unwrap_or(3).max(1);
            task.push_str(&format!(
                "Design a coherent {pages}-page site for: {brief}\n\nPlan the pages and one shared token system first \u{2014} the same `:root` variables on every page and one navigation \u{2014} then put each page on the board `{slug}` with the `frame` tool (project `{slug}`, set `{slug}`), one call per page, each named for its page. Internal links point at the other pages' frame names; the canvas rewrites them to the set's slugs and reports any link that resolves to no page."
            ));
        }
        DesignMode::Redesign => {
            task.push_str(&format!(
                "Redesign: {brief}\n\nModel the result on the reference facts above \u{2014} its palette, type and layout discipline, not its copy \u{2014} and put the page on the board `{slug}` with the `frame` tool (project `{slug}`)."
            ));
        }
    }
    // A question has no frame, so no size line: the size belongs to the
    // modes that put one on the board.
    if !matches!(mode, DesignMode::Ask) {
        if let Some((size, w, h)) = facts.size {
            task.push_str(&format!(
                "\n\nTarget frame: {size} ({w:.0}\u{00d7}{h:.0}) \u{2014} pass `viewport: \"{size}\"` to the `frame` tool."
            ));
        }
    }
    if facts.directions > 1 && matches!(mode, DesignMode::Generate) {
        task.push_str(&format!(
            "\n\nGive {directions} directions, one `frame` call each, each clearly different in type, palette and layout discipline.",
            directions = facts.directions
        ));
    }
    if let Some(preset) = &facts.preset {
        task.push_str(&format!(
            "\n\nStyle: {} \u{2014} {}",
            preset.label, preset.spec
        ));
        // The library's own avoid list is the negative prompt: named so a
        // run knows what the direction is *not*.
        if !preset.avoid.trim().is_empty() {
            task.push_str(&format!("\n\nAvoid: {}", preset.avoid));
        }
    }
    if let Some((name, tokens, doctrine)) = &facts.brand {
        task.push_str(&format!("\n\nThe brand `{name}` is binding. Its tokens:"));
        for (token, value) in tokens {
            task.push_str(&format!(" --{token}: {value};"));
        }
        if !doctrine.trim().is_empty() {
            task.push_str(&format!("\n{doctrine}"));
        }
    }
    if !facts.pins.is_empty() {
        task.push_str("\n\nOpen pins on the frame:");
        for pin in &facts.pins {
            task.push_str(&format!("\n- {pin}"));
        }
    }
    if let Some(reference) = &facts.reference {
        task.push_str(&format!("\n\nReference facts:\n{reference}"));
    }
    task
}

// --- the reference site -----------------------------------------------------

/// The most bytes one reference page may carry.
pub const MAX_SITE_BYTES: usize = 2 * 1024 * 1024;

/// How many redirects a reference fetch follows.
pub const MAX_REDIRECTS: usize = 3;

/// The most pages a scan lists for the owner to pick from.
pub const MAX_DISCOVERED_PAGES: usize = 24;

/// One fetched page.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchedSite {
    pub url: String,
    pub html: String,
}

/// The design DNA a page carries: its palette, type, scale, radii, layout
/// signals and the pages of its own site. Every field is a scan of what
/// the page declares — a flag, not a rendering.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleFacts {
    pub url: String,
    pub title: String,
    pub description: String,
    pub palette: Vec<String>,
    pub fonts: Vec<String>,
    pub sizes: Vec<String>,
    pub radii: Vec<String>,
    pub layout: Vec<String>,
    pub pages: Vec<String>,
}

impl StyleFacts {
    /// The block one run reads. Lines that found nothing say so.
    pub fn block(&self) -> String {
        fn or_none(list: &[String], none: &str) -> String {
            if list.is_empty() {
                none.to_string()
            } else {
                list.join(" ")
            }
        }
        format!(
            "reference: {url}\ntitle: {title}\npalette: {palette}\ntype: {fonts} at {sizes}\nradius: {radii}\nlayout: {layout}\npages: {pages}",
            url = self.url,
            title = if self.title.is_empty() { "(none)" } else { &self.title },
            palette = or_none(&self.palette, "(no colors declared)"),
            fonts = or_none(&self.fonts, "(no families declared)"),
            sizes = or_none(&self.sizes, "(no sizes declared)"),
            radii = or_none(&self.radii, "(no radii declared)"),
            layout = or_none(&self.layout, "(no layout signals)"),
            pages = or_none(&self.pages, "(no same-site links)"),
        )
    }
}

/// Normalize a reference URL: http(s) only, no credentials, a lowercase
/// host. Everything else is refused by name — a reference fetch is a read
/// of a page the owner pointed at, not a request builder.
pub fn normalize_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("the reference names no URL".to_string());
    }
    let lower = trimmed.to_ascii_lowercase();
    let (scheme, rest) = if lower.starts_with("https://") {
        ("https", &trimmed[8..])
    } else if lower.starts_with("http://") {
        ("http", &trimmed[7..])
    } else {
        return Err(format!(
            "`{trimmed}` is not an http(s) URL — a reference must be one"
        ));
    };
    if rest.contains('@')
        && rest
            .split(['/', '?', '#'])
            .next()
            .unwrap_or("")
            .contains('@')
    {
        return Err("the reference carries credentials; a fetch never sends them".to_string());
    }
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..host_end];
    if authority.is_empty() {
        return Err(format!("`{trimmed}` names no host"));
    }
    let mut path = &rest[host_end..];
    if !path.starts_with('/') {
        path = if path.is_empty() { "/" } else { path };
    }
    let (host, port) = split_host_port(authority)?;
    let lower_host = host.to_ascii_lowercase();
    let port = match (scheme, port) {
        ("https", None) | ("https", Some(443)) => String::new(),
        ("http", None) | ("http", Some(80)) => String::new(),
        (_, Some(port)) => format!(":{port}"),
        (_, None) => String::new(),
    };
    Ok(format!("{scheme}://{lower_host}{port}{path}"))
}

/// A URL's host and optional port, IPv6 literals included.
fn split_host_port(authority: &str) -> Result<(&str, Option<u16>), String> {
    if let Some(rest) = authority.strip_prefix('[') {
        let Some(close) = rest.find(']') else {
            return Err(format!("`{authority}` has an unclosed IPv6 literal"));
        };
        let host = &authority[..close + 2];
        let after = &rest[close + 1..];
        let port = match after.strip_prefix(':') {
            Some(port) => Some(
                port.parse::<u16>()
                    .map_err(|_| format!("`{authority}` names no port"))?,
            ),
            None => None,
        };
        return Ok((host, port));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) => Ok((
            host,
            Some(
                port.parse::<u16>()
                    .map_err(|_| format!("`{authority}` names no port"))?,
            ),
        )),
        None => Ok((authority, None)),
    }
}

/// Whether a host is one a reference fetch must not reach: loopback,
/// private ranges, link-local, or a name that resolves on the local
/// machine by convention.
pub fn is_private_host(host: &str) -> bool {
    let host = host.trim_matches(['[', ']']).to_ascii_lowercase();
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host == "0.0.0.0"
    {
        return true;
    }
    match host.parse::<std::net::IpAddr>() {
        Ok(ip) => is_private_ip(ip),
        Err(_) => false,
    }
}

fn is_private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.octets()[0] == 0
                || (v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]))
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                || (v6.segments()[0] & 0xffc0) == 0xfe80
                || v6
                    .to_ipv4_mapped()
                    .is_some_and(|mapped| is_private_ip(std::net::IpAddr::V4(mapped)))
        }
    }
}

/// One exchange a reference fetch makes. The seam exists so the refusals
/// the network cannot produce on demand — a redirect past the cap, a body
/// past the byte cap — are pinned without a public host.
pub trait SiteTransport {
    fn get(&self, url: &str, pinned: Vec<std::net::SocketAddr>) -> Result<SiteResponse, String>;
}

/// One transport answer: the status, the headers a fetch decides by, and
/// the body as text (read up to the cap plus one byte).
#[derive(Debug, Clone, Default)]
pub struct SiteResponse {
    pub status: u16,
    pub location: Option<String>,
    pub content_type: Option<String>,
    pub body: String,
}

/// The real transport: ureq, with the resolution pinned to the addresses
/// that were checked, so a second DNS answer cannot move the request.
struct UreqTransport;

impl SiteTransport for UreqTransport {
    fn get(&self, url: &str, pinned: Vec<std::net::SocketAddr>) -> Result<SiteResponse, String> {
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .timeout(std::time::Duration::from_secs(20))
            .resolver(
                move |_: &str| -> std::io::Result<Vec<std::net::SocketAddr>> { Ok(pinned.clone()) },
            )
            .build();
        let response = match agent.get(url).call() {
            Ok(response) => response,
            Err(ureq::Error::Status(code, response)) => {
                return Ok(SiteResponse {
                    status: code,
                    location: response.header("location").map(str::to_string),
                    content_type: response.header("content-type").map(str::to_string),
                    body: String::new(),
                });
            }
            Err(error) => return Err(format!("the site could not be read: {error}")),
        };
        let status = response.status();
        let location = response.header("location").map(str::to_string);
        let content_type = response.header("content-type").map(str::to_string);
        use std::io::Read;
        let mut body = String::new();
        response
            .into_reader()
            .take(MAX_SITE_BYTES as u64 + 1)
            .read_to_string(&mut body)
            .map_err(|error| format!("the page could not be read: {error}"))?;
        Ok(SiteResponse {
            status,
            location,
            content_type,
            body,
        })
    }
}

/// Fetch one reference page: http(s) only, a public host only, the
/// resolution pinned to the addresses that were checked, at most three
/// redirects — each re-validated — and a byte cap. Every refusal names
/// what it refused.
pub fn fetch_site(url: &str) -> Result<FetchedSite, String> {
    fetch_with(url, &UreqTransport)
}

fn fetch_with(url: &str, transport: &dyn SiteTransport) -> Result<FetchedSite, String> {
    let mut current = normalize_url(url)?;
    for hop in 0..=MAX_REDIRECTS {
        let Some((scheme, rest)) = current.split_once("://") else {
            return Err(format!("`{current}` is not a URL"));
        };
        let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..host_end];
        let (host, port) = split_host_port(authority)?;
        if is_private_host(host) {
            return Err(format!(
                "`{host}` is a private or loopback host; a reference must be a public site"
            ));
        }
        let port = port.unwrap_or(if scheme == "https" { 443 } else { 80 });
        let addrs: Vec<std::net::SocketAddr> = std::net::ToSocketAddrs::to_socket_addrs(&(
            host.trim_matches(['[', ']']).to_string(),
            port,
        ))
        .map_err(|error| format!("`{host}` did not resolve: {error}"))?
        .collect();
        if addrs.is_empty() {
            return Err(format!("`{host}` did not resolve"));
        }
        if let Some(bad) = addrs.iter().find(|addr| is_private_ip(addr.ip())) {
            return Err(format!(
                "`{host}` resolves to a private address ({}) — a reference must be public",
                bad.ip()
            ));
        }
        let response = transport.get(&current, addrs)?;
        if (300..400).contains(&response.status) {
            if hop == MAX_REDIRECTS {
                return Err(format!(
                    "the site redirects more than {MAX_REDIRECTS} times; a reference must land"
                ));
            }
            let location = response
                .location
                .ok_or_else(|| format!("the site answered {} with no location", response.status))?;
            let next = if location.starts_with("http://") || location.starts_with("https://") {
                location.to_string()
            } else if let Some(path) = location.strip_prefix('/') {
                format!("{scheme}://{authority}/{path}")
            } else {
                format!(
                    "{scheme}://{authority}/{}",
                    location.trim_start_matches("./")
                )
            };
            current = normalize_url(&next)?;
            continue;
        }
        if !(200..300).contains(&response.status) {
            return Err(format!(
                "the site answered {}; nothing to scan",
                response.status
            ));
        }
        let content_type = response
            .content_type
            .unwrap_or_else(|| "text/html".to_string());
        if !content_type.to_ascii_lowercase().contains("html") {
            return Err(format!(
                "the site served `{content_type}`; a reference must be a page"
            ));
        }
        if response.body.len() > MAX_SITE_BYTES {
            return Err(format!(
                "the page is larger than {MAX_SITE_BYTES} bytes; a reference must be a page, not an asset"
            ));
        }
        return Ok(FetchedSite {
            url: current,
            html: response.body,
        });
    }
    Err(format!(
        "the site redirects more than {MAX_REDIRECTS} times; a reference must land"
    ))
}

/// Scan a page for the facts a design run can model: the declared palette,
/// type families and sizes, radii, layout signals, and the site's own
/// pages. A scanner over declarations, never a parser — the same rule the
/// a11y scan follows.
pub fn style_facts(url: &str, html: &str) -> StyleFacts {
    let mut facts = StyleFacts {
        url: url.to_string(),
        ..Default::default()
    };
    facts.title = between(html, "<title", "</title>")
        .map(|text| {
            text.split_once('>')
                .map(|(_, rest)| rest)
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .unwrap_or_default();
    if let Some(meta) = find_meta_description(html) {
        facts.description = meta;
    }
    let mut palette: Vec<(String, usize)> = Vec::new();
    let mut fonts: Vec<(String, usize)> = Vec::new();
    let mut sizes: Vec<(String, usize)> = Vec::new();
    let mut radii: Vec<(String, usize)> = Vec::new();
    let mut grids = 0usize;
    let mut flexes = 0usize;
    let mut widths: Vec<(String, usize)> = Vec::new();
    for (property, value) in declarations(html) {
        match property.as_str() {
            "color" | "background" | "background-color" | "border-color" | "fill" | "stroke" => {
                for color in hex_colors(&value) {
                    bump(&mut palette, color);
                }
            }
            "font-family" => bump(&mut fonts, clean_value(&value)),
            "font-size" => bump(&mut sizes, clean_value(&value)),
            "border-radius" => bump(&mut radii, clean_value(&value)),
            "max-width" => bump(&mut widths, clean_value(&value)),
            "display" => {
                let value = value.trim().to_ascii_lowercase();
                if value == "grid" {
                    grids += 1;
                } else if value == "flex" {
                    flexes += 1;
                }
            }
            _ => {}
        }
    }
    // Colours also ride inline attributes (`fill="#fff"`), which the
    // declaration scan does not see.
    for (attribute, value) in attributes(html) {
        if matches!(attribute.as_str(), "fill" | "stroke" | "color") {
            for color in hex_colors(&value) {
                bump(&mut palette, color);
            }
        }
    }
    facts.palette = ranked(palette, 8);
    facts.fonts = ranked(fonts, 4);
    facts.sizes = ranked(sizes, 6);
    facts.radii = ranked(radii, 4);
    if grids > 0 {
        facts.layout.push(format!("grid×{grids}"));
    }
    if flexes > 0 {
        facts.layout.push(format!("flex×{flexes}"));
    }
    for width in ranked(widths, 2) {
        facts.layout.push(format!("max-width {width}"));
    }
    facts.pages = discover_pages(html, url);
    facts
}

/// The pages a site's own markup points at, as paths or absolute URLs on
/// the same host, deduped and capped.
pub fn discover_pages(html: &str, base: &str) -> Vec<String> {
    let base_host = base
        .split_once("://")
        .map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or("").to_string())
        .unwrap_or_default();
    let mut out: Vec<String> = Vec::new();
    for (attribute, value) in attributes(html) {
        if attribute != "href" {
            continue;
        }
        let href = value.trim();
        if href.is_empty()
            || href.starts_with('#')
            || href.starts_with("mailto:")
            || href.starts_with("tel:")
            || href.starts_with("javascript:")
        {
            continue;
        }
        let page = if let Some(rest) = href.strip_prefix("//") {
            // Protocol-relative: same scheme, host must match.
            if !rest.starts_with(&base_host) {
                continue;
            }
            format!("{}://{}", base.split("://").next().unwrap_or("https"), rest)
        } else if href.starts_with("http://") || href.starts_with("https://") {
            let host = href
                .split_once("://")
                .map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or(""))
                .unwrap_or("");
            if host != base_host {
                continue;
            }
            href.to_string()
        } else if href.starts_with('/') {
            href.to_string()
        } else if href.starts_with("..") || href.contains(':') {
            continue;
        } else {
            format!("/{}", href.trim_start_matches("./"))
        };
        let page = page.split('#').next().unwrap_or(&page).to_string();
        if !out.contains(&page) {
            out.push(page);
        }
        if out.len() >= MAX_DISCOVERED_PAGES {
            break;
        }
    }
    out
}

/// The text between two markers, with the opening tag's own tail.
fn between<'a>(html: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find(open)?;
    let rest = &html[start..];
    let end = lower[start..].find(close)?;
    Some(&rest[..end])
}

fn find_meta_description(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut at = 0;
    while let Some(found) = lower[at..].find("<meta") {
        let start = at + found;
        let end = lower[start..].find('>').map(|offset| start + offset)?;
        let tag = &html[start..end];
        let tag_lower = tag.to_ascii_lowercase();
        if tag_lower.contains("name=\"description\"") || tag_lower.contains("name='description'") {
            // The value is read from the page's own text, so its case and
            // wording survive: the scan finds, it does not rewrite.
            let value = tag_lower
                .find("content=")
                .map(|at| tag[at + "content=".len()..].trim())
                .unwrap_or("");
            let cleaned = value.trim_matches(['"', '\'']).trim().to_string();
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
        at = end;
    }
    None
}

/// Every `property: value` declaration the page carries, in order.
fn declarations(html: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(colon) = rest.find(':') {
        let before = &rest[..colon];
        let name_start = before
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .map(|at| at + 1)
            .unwrap_or(0);
        let name = before[name_start..].trim().to_ascii_lowercase();
        let after = &rest[colon + 1..];
        let end = after.find([';', '}', '"']).unwrap_or(after.len());
        let value = &after[..end];
        if !name.is_empty() && !value.trim().is_empty() {
            out.push((name, value.trim().to_string()));
        }
        rest = &after[end..];
        if rest.is_empty() {
            break;
        }
        rest = &rest[1..];
    }
    out
}

/// Every `attribute="value"` pair in the page's tags (single or double
/// quoted), in order.
fn attributes(html: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = html.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        // An attribute name ends where whitespace or `=` stops it.
        let name_start = at;
        while at < bytes.len()
            && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'-' || bytes[at] == b'_')
        {
            at += 1;
        }
        if at == name_start {
            at += 1;
            continue;
        }
        let name = html[name_start..at].to_ascii_lowercase();
        let mut cursor = at;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor < bytes.len() && bytes[cursor] == b'=' {
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && (bytes[cursor] == b'"' || bytes[cursor] == b'\'') {
                let mark = bytes[cursor];
                let start = cursor + 1;
                let mut close = start;
                while close < bytes.len() && bytes[close] != mark {
                    close += 1;
                }
                out.push((name, html[start..close.min(html.len())].to_string()));
                at = (close + 1).min(bytes.len());
                continue;
            }
        }
        at = at.max(name_start + 1);
    }
    out
}

/// The hex colours in one value, normalized lowercase.
fn hex_colors(value: &str) -> Vec<String> {
    let lower = value.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(found) = lower[at..].find('#') {
        let start = at + found;
        let mut end = start + 1;
        while end < bytes.len() && bytes[end].is_ascii_hexdigit() {
            end += 1;
        }
        let digits = end - start - 1;
        if digits == 3 || digits == 6 || digits == 8 {
            out.push(lower[start..end].to_string());
        }
        at = end.max(start + 1);
        if at >= lower.len() {
            break;
        }
    }
    out.sort();
    out.dedup();
    out
}

fn clean_value(value: &str) -> String {
    value.trim().trim_matches(['\'', '"']).trim().to_string()
}

fn bump(list: &mut Vec<(String, usize)>, value: String) {
    if value.is_empty() {
        return;
    }
    match list.iter_mut().find(|(known, _)| *known == value) {
        Some((_, count)) => *count += 1,
        None => list.push((value, 1)),
    }
}

fn ranked(list: Vec<(String, usize)>, cap: usize) -> Vec<String> {
    let mut list = list;
    list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    list.into_iter().take(cap).map(|(value, _)| value).collect()
}

// --- the codegen and the diff ------------------------------------------------

/// A component name from a frame's name, the source's own rule: words are
/// PascalCase, and a name that cannot start an identifier gets `Frame`.
pub fn to_component_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect();
    let joined: String = cleaned
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();
    match joined.chars().next() {
        Some(first) if first.is_ascii_alphabetic() => joined,
        _ => format!("Frame{joined}"),
    }
}

/// A React component from a frame page: the body's markup as JSX (attribute
/// names renamed, styles as objects, void tags self-closed, text escaped,
/// scripts held out as a TODO), the page's style in a `<style>` element, and
/// the frame's tweaks as props that override the same custom properties.
/// A deterministic transform — the source's own, ported — so the export is
/// testable and the CLI, the face and a cron job write identical bytes.
pub fn to_tsx(
    frame_name: &str,
    html: &str,
    tweaks: &[TweakSpec],
    values: &BTreeMap<String, Value>,
) -> String {
    let component = to_component_name(frame_name);
    let css = between(html, "<style", "</style>")
        .map(|chunk| {
            chunk
                .split_once('>')
                .map(|(_, css)| css.trim().to_string())
                .unwrap_or_default()
        })
        .unwrap_or_default();
    let body = between(html, "<body", "</body>")
        .map(|chunk| {
            chunk
                .split_once('>')
                .map(|(_, body)| body.to_string())
                .unwrap_or_default()
        })
        .unwrap_or_else(|| html.to_string());
    // Scripts are held out of the component and named as a TODO: a page's
    // behaviour is not a mechanical transform.
    let mut scripts: Vec<String> = Vec::new();
    let mut body = body;
    loop {
        let lower = body.to_ascii_lowercase();
        let Some(start) = lower.find("<script") else {
            break;
        };
        let Some(close) = lower[start..].find("</script>") else {
            break;
        };
        let end = start + close + "</script>".len();
        let tag_end = lower[start..]
            .find('>')
            .map(|at| start + at + 1)
            .unwrap_or(end);
        if tag_end <= end {
            scripts.push(body[tag_end..start + close].trim().to_string());
        }
        body.replace_range(start..end, "");
    }
    let jsx = convert_element(&body);
    let mut props: Vec<String> = Vec::new();
    let mut types: Vec<String> = Vec::new();
    let mut injections: Vec<String> = Vec::new();
    for spec in tweaks {
        let ident = safe_ident(&spec.key);
        let current = values.get(&spec.key).cloned().unwrap_or_else(|| {
            if spec.default.is_null() {
                match spec.kind.as_str() {
                    "toggle" => Value::Bool(true),
                    _ => Value::Number(0.into()),
                }
            } else {
                spec.default.clone()
            }
        });
        let literal = match &current {
            Value::String(text) => format!("\"{}\"", text.replace('"', "\\\"")),
            other => other.to_string(),
        };
        let kind = match spec.kind.as_str() {
            "toggle" => "boolean",
            "slider" => "number",
            _ => "string | number",
        };
        let label = if spec.label.is_empty() {
            &spec.key
        } else {
            &spec.label
        };
        props.push(format!("  /** {label} */\n  {ident} = {literal},"));
        types.push(format!("    /** {label} */\n    {ident}?: {kind};"));
        injections.push(format!("    --{}: ${{{ident}}} as any,", spec.key));
    }
    let props_block = if props.is_empty() {
        String::new()
    } else {
        format!(
            "  {{\n{}\n  }}: {{\n{}\n  }} = {{}}",
            props.join("\n"),
            types.join("\n")
        )
    };
    // The tweak props override the page's own custom properties.
    let sheet = if injections.is_empty() {
        css.clone()
    } else {
        let override_block = format!("\n      :root {{\n{}\n      }}", injections.join("\n"));
        if css.to_ascii_lowercase().contains(":root") {
            format!("{css}{override_block}")
        } else {
            format!("{override_block}\n{css}")
        }
    };
    let indented = jsx
        .split('\n')
        .map(|line| {
            if line.trim().is_empty() {
                line.to_string()
            } else {
                format!("      {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let script_note = if scripts.is_empty() {
        String::new()
    } else {
        format!(
            "\n/*\n * TODO: port inline script(s) from the original frame:\n{}\n */",
            scripts
                .iter()
                .map(|script| format!(
                    " * {}",
                    script.split('\n').collect::<Vec<_>>().join("\n * ")
                ))
                .collect::<Vec<_>>()
                .join("\n *\n")
        )
    };
    format!(
        "/**\n * {frame_name} — generated by fleet's design canvas.\n * Design tokens live in :root below (or in tokens.css); the tweak props map to\n * the same CSS custom properties, so the component stays live-tweakable.\n */\nexport default function {component}({props_block}) {{\n  return (\n    <>\n      <style>{{`\n{sheet}\n      `}}</style>\n{indented}\n    </>\n  );\n}}{script_note}\n"
    )
}

fn safe_ident(key: &str) -> String {
    let cleaned: String = key
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    if cleaned.is_empty() {
        "tweak".to_string()
    } else if cleaned.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("_{cleaned}")
    } else {
        cleaned
    }
}

fn jsx_attr_name(attr: &str) -> String {
    let lower = attr.to_ascii_lowercase();
    let renamed = match lower.as_str() {
        "class" => Some("className"),
        "for" => Some("htmlFor"),
        "tabindex" => Some("tabIndex"),
        "readonly" => Some("readOnly"),
        "maxlength" => Some("maxLength"),
        "minlength" => Some("minLength"),
        "autocomplete" => Some("autoComplete"),
        "autofocus" => Some("autoFocus"),
        "autoplay" => Some("autoPlay"),
        "colspan" => Some("colSpan"),
        "rowspan" => Some("rowSpan"),
        "contenteditable" => Some("contentEditable"),
        "crossorigin" => Some("crossOrigin"),
        "datetime" => Some("dateTime"),
        "enctype" => Some("encType"),
        "novalidate" => Some("noValidate"),
        "spellcheck" => Some("spellCheck"),
        "srcdoc" => Some("srcDoc"),
        "srcset" => Some("srcSet"),
        "usemap" => Some("useMap"),
        _ => None,
    };
    if let Some(name) = renamed {
        return name.to_string();
    }
    if lower.starts_with("data-") || lower.starts_with("aria-") {
        return lower;
    }
    let mut out = String::with_capacity(attr.len());
    let mut upper = false;
    for ch in attr.chars() {
        if ch == '-' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// An inline style string as a JSX object literal.
fn style_string_to_object(style: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for declaration in style.split(';') {
        let Some((prop, value)) = declaration.split_once(':') else {
            continue;
        };
        let prop = prop.trim();
        let value = value.trim();
        if prop.is_empty() || value.is_empty() {
            continue;
        }
        let key = if prop.starts_with("--") {
            format!("\"{prop}\"")
        } else {
            let mut out = String::new();
            let mut upper = false;
            for ch in prop.chars() {
                if ch == '-' {
                    upper = true;
                } else if upper {
                    out.extend(ch.to_uppercase());
                    upper = false;
                } else {
                    out.push(ch);
                }
            }
            out
        };
        parts.push(format!(
            "{key}: {}",
            serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
        ));
    }
    format!("{{{{ {} }}}}", parts.join(", "))
}

fn escape_jsx_text(text: &str) -> String {
    text.replace('{', "&#123;")
        .replace('}', "&#125;")
        .replace('\'', "&apos;")
}

/// The page's markup as JSX. A scanner over tags (quotes honored), the same
/// discipline as the rest of the module.
fn convert_element(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut at = 0;
    while at < html.len() {
        let comment = html[at..].find("<!--").map(|found| at + found);
        let Some(tag_start) = html[at..].find('<').map(|found| at + found) else {
            out.push_str(&escape_jsx_text(&html[at..]));
            break;
        };
        out.push_str(&escape_jsx_text(&html[at..tag_start]));
        if comment == Some(tag_start) {
            match html[tag_start..].find("-->") {
                Some(close) => {
                    out.push_str(&format!(
                        "{{/*{}*/}}",
                        &html[tag_start + 4..tag_start + close]
                    ));
                    at = tag_start + close + 3;
                    continue;
                }
                None => {
                    out.push_str(&html[tag_start..]);
                    break;
                }
            }
        }
        let Some(tag_end) = find_tag_end(html, tag_start) else {
            out.push_str(&html[tag_start..]);
            break;
        };
        let raw = &html[tag_start..=tag_end];
        at = tag_end + 1;
        let name: String = raw
            .trim_start_matches('<')
            .trim_start_matches('/')
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
            .collect::<String>()
            .to_ascii_lowercase();
        if name.is_empty() {
            out.push_str(&format!(
                "{{{}}}",
                serde_json::to_string(raw).unwrap_or_default()
            ));
            continue;
        }
        if raw.starts_with("</") {
            out.push_str(&format!("</{name}>"));
            continue;
        }
        let self_closing = raw.ends_with("/>")
            || matches!(
                name.as_str(),
                "area"
                    | "base"
                    | "br"
                    | "col"
                    | "embed"
                    | "hr"
                    | "img"
                    | "input"
                    | "link"
                    | "meta"
                    | "param"
                    | "source"
                    | "track"
                    | "wbr"
            );
        let attrs = parse_attrs(raw);
        let rendered: Vec<String> = attrs
            .iter()
            .map(|(attr, value)| {
                let name = jsx_attr_name(attr);
                match value {
                    None => name,
                    Some(value) if name == "style" && !value.contains('{') => {
                        format!("style={}", style_string_to_object(value))
                    }
                    Some(value) => format!(
                        "{name}={}",
                        serde_json::to_string(value).unwrap_or_default()
                    ),
                }
            })
            .collect();
        let attr_str = if rendered.is_empty() {
            String::new()
        } else {
            format!(" {}", rendered.join(" "))
        };
        out.push_str(&format!(
            "<{name}{attr_str}{}>",
            if self_closing { " /" } else { "" }
        ));
    }
    out
}

/// The index of a tag's closing `>`, honoring quotes.
fn find_tag_end(html: &str, start: usize) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (offset, ch) in html[start + 1..].char_indices() {
        match quote {
            Some(mark) if ch == mark => quote = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == '>' => return Some(start + 1 + offset),
            None => {}
        }
    }
    None
}

/// A tag's attributes, in order; a value-less attribute has `None`.
fn parse_attrs(tag: &str) -> Vec<(String, Option<String>)> {
    let body = tag
        .trim_start_matches('<')
        .trim_start_matches('/')
        .trim_end_matches('>')
        .trim_end_matches('/');
    let name_end = body.find(char::is_whitespace).unwrap_or(body.len());
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    let rest = &body[name_end..];
    let bytes = rest.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        if at >= bytes.len() {
            break;
        }
        let name_start = at;
        while at < bytes.len()
            && (bytes[at].is_ascii_alphanumeric()
                || matches!(bytes[at], b':' | b'_' | b'.' | b'-' | b'@'))
        {
            at += 1;
        }
        if at == name_start {
            at += 1;
            continue;
        }
        let name = rest[name_start..at].to_string();
        let mut value = None;
        let mut probe = at;
        while probe < bytes.len() && bytes[probe].is_ascii_whitespace() {
            probe += 1;
        }
        if probe < bytes.len() && bytes[probe] == b'=' {
            let mut cursor = probe + 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && (bytes[cursor] == b'"' || bytes[cursor] == b'\'') {
                let mark = bytes[cursor];
                let start = cursor + 1;
                let mut close = start;
                while close < bytes.len() && bytes[close] != mark {
                    close += 1;
                }
                value = Some(rest[start..close.min(rest.len())].to_string());
                at = (close + 1).min(bytes.len());
            } else {
                let start = cursor;
                let mut close = start;
                while close < bytes.len() && !bytes[close].is_ascii_whitespace() {
                    close += 1;
                }
                value = Some(rest[start..close].to_string());
                at = close;
            }
        }
        out.push((name, value));
    }
    out
}

/// One token's change between two versions of a page.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TokenChange {
    pub token: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// The token diff between two versions of a page: what was added, what was
/// dropped, what changed value. Deterministic and ordered by token name.
pub fn token_diff(before_html: &str, after_html: &str) -> Vec<TokenChange> {
    let before: BTreeMap<String, String> = extract_root_tokens(before_html).into_iter().collect();
    let after: BTreeMap<String, String> = extract_root_tokens(after_html).into_iter().collect();
    let mut names: Vec<String> = before.keys().chain(after.keys()).cloned().collect();
    names.sort();
    names.dedup();
    let mut out = Vec::new();
    for token in names {
        let was = before.get(&token);
        let now = after.get(&token);
        if was == now {
            continue;
        }
        out.push(TokenChange {
            token,
            before: was.cloned(),
            after: now.cloned(),
        });
    }
    out
}

// --- page sets ---------------------------------------------------------------

/// One page of a set: the frame, its name, and the slug its file takes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PageSlug {
    pub id: String,
    pub name: String,
    pub slug: String,
}

/// The pages of one set, in board order.
pub fn page_set<'a>(project: &'a Project, set: &str) -> Vec<&'a Frame> {
    project
        .frames
        .iter()
        .filter(|frame| frame.set.as_deref() == Some(set))
        .collect()
}

/// Every set the board carries, by name, in the order its first page
/// landed.
pub fn sets(project: &Project) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for frame in &project.frames {
        if let Some(set) = &frame.set {
            if !out.contains(set) {
                out.push(set.clone());
            }
        }
    }
    out
}

/// The file slug each page of a set takes. A duplicate slug is the
/// coherence failure the packet names — the slug itself stays honest
/// (the name, folded), never silently suffixed.
pub fn page_set_slugs(frames: &[&Frame]) -> Vec<PageSlug> {
    frames
        .iter()
        .map(|frame| PageSlug {
            id: frame.id.clone(),
            name: frame.name.clone(),
            slug: slug_of(&frame.name),
        })
        .collect()
}

/// One set's coherence, as facts: the tokens every page declares, the
/// tokens a page is missing, links that point at no page in the set, and
/// duplicate slugs. The report is a read of what the pages say.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Coherence {
    pub pages: usize,
    /// The `:root` tokens every page of the set declares.
    pub shared: Vec<String>,
    /// `page: token` for a token the rest of the set declares and one
    /// page does not.
    pub missing: Vec<String>,
    /// `page → target` for an internal link that names no page in the set.
    pub broken: Vec<String>,
    /// The slugs two pages share.
    pub duplicates: Vec<String>,
}

impl Coherence {
    pub fn ok(&self) -> bool {
        self.missing.is_empty() && self.broken.is_empty() && self.duplicates.is_empty()
    }
}

/// The fact report of one set.
pub fn coherence(frames: &[&Frame]) -> Coherence {
    let slugs = page_set_slugs(frames);
    let token_sets: Vec<BTreeMap<String, String>> = frames
        .iter()
        .map(|frame| {
            extract_root_tokens(&frame.html)
                .into_iter()
                .collect::<BTreeMap<_, _>>()
        })
        .collect();
    let mut shared: Vec<String> = Vec::new();
    if let Some(first) = token_sets.first() {
        for token in first.keys() {
            if token_sets.iter().all(|set| set.contains_key(token)) {
                shared.push(token.clone());
            }
        }
    }
    // The missing report is over the UNION: a token the rest of the set
    // declares and one page does not is exactly what the intersection
    // cannot see.
    let mut union: Vec<String> = Vec::new();
    for set in &token_sets {
        for token in set.keys() {
            if !union.contains(token) {
                union.push(token.clone());
            }
        }
    }
    let mut missing: Vec<String> = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        for token in &union {
            if !token_sets[index].contains_key(token) {
                missing.push(format!("{}: {token}", frame.name));
            }
        }
    }
    // A set that agrees on no token at all is the failure the
    // intersection cannot see: name every page rather than report
    // silence.
    if shared.is_empty() && frames.len() > 1 {
        for frame in frames {
            missing.push(format!("{}: no shared token", frame.name));
        }
    }
    let mut duplicates: Vec<String> = Vec::new();
    for (index, page) in slugs.iter().enumerate() {
        if slugs
            .iter()
            .enumerate()
            .any(|(other, one)| other != index && one.slug == page.slug)
            && !duplicates.contains(&page.slug)
        {
            duplicates.push(page.slug.clone());
        }
    }
    let known: Vec<String> = slugs
        .iter()
        .flat_map(|page| [page.name.to_lowercase(), page.slug.clone()])
        .collect();
    let mut broken: Vec<String> = Vec::new();
    for frame in frames {
        for (attribute, value) in attributes(&frame.html) {
            if attribute != "href" {
                continue;
            }
            let target = value.trim();
            if target.is_empty()
                || target.starts_with('#')
                || target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("//")
                || target.starts_with("mailto:")
                || target.starts_with("tel:")
                || target.starts_with("data:")
                || target.starts_with("javascript:")
            {
                continue;
            }
            let clean = target
                .trim_start_matches("./")
                .trim_start_matches('/')
                .trim_end_matches(".html")
                .trim_end_matches('/')
                .to_lowercase();
            if !known.contains(&clean) {
                broken.push(format!("{} → {target}", frame.name));
            }
        }
    }
    Coherence {
        pages: frames.len(),
        shared,
        missing,
        broken,
        duplicates,
    }
}

/// Rewrite a page's internal links to the set's slugs: a link naming
/// another page (by frame name or by `/name`) becomes `<slug>.html`, the
/// file the packet writes. External links, anchors and the page's own
/// name are left alone; one implementation serves the preview and the
/// packet.
pub fn rewrite_page_links(html: &str, slugs: &[PageSlug], own: &str) -> String {
    let mut out = html.to_string();
    for page in slugs {
        if page.name.eq_ignore_ascii_case(own) {
            continue;
        }
        for form in [
            format!("href=\"{}\"", page.name),
            format!("href='/{}'", page.name),
            format!("href=\"/{}\"", page.slug),
            format!("href=\"{}\"", page.slug),
            format!("href=\"/{}\"", page.name),
        ] {
            let replacement = format!("href=\"{}.html\"", page.slug);
            out = out.replace(&form, &replacement);
        }
    }
    out
}

/// The packet one set hands over: each page's file (its tweaks injected
/// and its internal links rewritten), the tokens, the manifest, the
/// changelog, and the captured PNGs when the caller brought them.
pub fn set_packet(
    home: &Path,
    slug: &str,
    set: &str,
    pngs: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let project = load(home, slug)?;
    let frames = page_set(&project, set);
    if frames.is_empty() {
        return Err(format!("no page set `{set}` on board `{slug}`"));
    }
    frames_packet(slug, set, &frames, pngs)
}

/// The packet of any chosen frames — a set, or the owner's own selection:
/// each page's file, the tokens, the manifest, the changelog, and whatever
/// captures the caller brought.
pub fn selection_packet(
    home: &Path,
    slug: &str,
    ids: &[String],
    pngs: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let project = load(home, slug)?;
    let frames: Vec<&Frame> = project
        .frames
        .iter()
        .filter(|frame| ids.iter().any(|id| id == &frame.id))
        .collect();
    if frames.is_empty() {
        return Err("the selection names no frame on this board".to_string());
    }
    frames_packet(slug, "selection", &frames, pngs)
}

fn frames_packet(
    slug: &str,
    set: &str,
    frames: &[&Frame],
    pngs: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let slugs = page_set_slugs(frames);
    let report = coherence(frames);
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    let mut manifest_pages: Vec<Value> = Vec::new();
    let mut tokens_css = String::from("/* the set's tokens, as the pages declare them */\n");
    let mut tokens_json: BTreeMap<String, Value> = BTreeMap::new();
    for (frame, page) in frames.iter().zip(slugs.iter()) {
        let html = inject_tweaks(&frame.html, &frame.tweak_spec, &frame.tweak_values);
        let html = rewrite_page_links(&html, &slugs, &frame.name);
        parts.push((format!("{}.html", page.slug), html.into_bytes()));
        let tokens = extract_root_tokens(&frame.html);
        tokens_css.push_str(&format!(
            "/* {}.html */\n:root {{\n{}\n}}\n",
            page.slug,
            tokens
                .iter()
                .map(|(token, value)| format!("  --{token}: {value};"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
        tokens_json.insert(
            page.slug.clone(),
            Value::Object(
                tokens
                    .iter()
                    .map(|(token, value)| (token.clone(), Value::String(value.clone())))
                    .collect(),
            ),
        );
        let desktop = pngs.get(&frame.id);
        let mobile = pngs.get(&format!("{}:mobile", frame.id));
        manifest_pages.push(serde_json::json!({
            "file": format!("{}.html", page.slug),
            "name": frame.name,
            "id": frame.id,
            "size": format!("{}×{}", frame.w, frame.h),
            "tokens": tokens.len(),
            "png": desktop.map(|_| format!("png/{}.png", page.slug)),
            "png_mobile": mobile.map(|_| format!("png/{}-mobile.png", page.slug)),
        }));
        if let Some(bytes) = desktop {
            parts.push((format!("png/{}.png", page.slug), bytes.clone()));
        }
        if let Some(bytes) = mobile {
            parts.push((format!("png/{}-mobile.png", page.slug), bytes.clone()));
        }
    }
    parts.push(("tokens.css".to_string(), tokens_css.into_bytes()));
    parts.push((
        "tokens.json".to_string(),
        serde_json::to_vec_pretty(&tokens_json)
            .map_err(|error| format!("the tokens do not serialize: {error}"))?,
    ));
    let manifest = serde_json::json!({
        "board": slug,
        "set": set,
        "pages": manifest_pages,
        "shared_tokens": report.shared,
        "coherence": {
            "ok": report.ok(),
            "missing": report.missing,
            "broken": report.broken,
            "duplicates": report.duplicates,
        },
        "created": crate::calib::stamp(),
    });
    parts.push((
        "manifest.json".to_string(),
        serde_json::to_vec_pretty(&manifest)
            .map_err(|error| format!("the manifest does not serialize: {error}"))?,
    ));
    let mut changelog = format!(
        "# {set}\n\n{} page{} on board `{slug}`, handed over {}.\n\n",
        frames.len(),
        if frames.len() == 1 { "" } else { "s" },
        crate::calib::stamp()
    );
    for (frame, page) in frames.iter().zip(slugs.iter()) {
        changelog.push_str(&format!(
            "- `{}.html` — {}, {}×{}, {} token{}, {} version{}\n",
            page.slug,
            frame.name,
            frame.w,
            frame.h,
            extract_root_tokens(&frame.html).len(),
            if extract_root_tokens(&frame.html).len() == 1 {
                ""
            } else {
                "s"
            },
            frame.versions.len(),
            if frame.versions.len() == 1 { "" } else { "s" },
        ));
    }
    if !report.ok() {
        changelog.push_str("\n## coherence\n\n");
        for line in &report.missing {
            changelog.push_str(&format!("- missing token — {line}\n"));
        }
        for line in &report.broken {
            changelog.push_str(&format!("- broken link — {line}\n"));
        }
        for line in &report.duplicates {
            changelog.push_str(&format!("- duplicate slug — {line}\n"));
        }
    }
    parts.push(("changelog.md".to_string(), changelog.into_bytes()));
    Ok(crate::deck::zip_parts(&parts))
}

/// Put one landed frame into a set.
pub fn set_frame_set(home: &Path, slug: &str, frame_id: &str, set: &str) -> Result<(), String> {
    let mut project = load(home, slug)?;
    let frame = project
        .frames
        .iter_mut()
        .find(|frame| frame.id == frame_id)
        .ok_or_else(|| format!("no frame `{frame_id}` on this board"))?;
    let set = set.trim();
    frame.set = if set.is_empty() {
        None
    } else {
        Some(set.to_string())
    };
    project.updated = crate::calib::stamp();
    save(home, slug, &project)
}

// --- frame operations -------------------------------------------------------

/// A fresh id: a prefix and eight hex characters of randomness, unique
/// within a board by construction of the randomness and the clock.
fn new_id(prefix: &str) -> String {
    format!("{prefix}-{:08x}", rand::random::<u32>())
}

/// Validate and clean one frame's HTML for storage: a complete document,
/// nothing larger than the cap. Returns the normalized HTML.
pub fn validate_frame_html(html: &str) -> Result<String, String> {
    let trimmed = html.trim();
    if trimmed.is_empty() {
        return Err("the frame carries no HTML".to_string());
    }
    if trimmed.len() > MAX_FRAME_BYTES {
        return Err(format!(
            "the frame is {} bytes; the cap is {MAX_FRAME_BYTES} — trim the page",
            trimmed.len()
        ));
    }
    let lower = trimmed.to_ascii_lowercase();
    if !(lower.starts_with("<!doctype") || lower.contains("<html")) {
        return Err(
            "the frame is not a complete document: start it with `<!doctype html>` and a <html> element"
                .to_string(),
        );
    }
    Ok(trimmed.to_string())
}

/// Add a frame to a board. With `frame_id` the call **refines** that
/// frame instead: the new HTML becomes the frame's content and the
/// previous content is snapshotted as a version. Without one, the frame
/// is new and lands to the right of the board's bounds.
pub fn put_frame(
    home: &Path,
    slug: &str,
    name: &str,
    html: &str,
    tweak_spec: Option<&[TweakSpec]>,
    frame_id: Option<&str>,
    model: &str,
) -> Result<String, String> {
    put_frame_sized(home, slug, name, html, tweak_spec, frame_id, model, None)
}

/// The same write with the frame's size decided by the caller: a brief
/// that asked for a phone gets a phone canvas. `None` keeps the board's
/// desktop default.
#[allow(clippy::too_many_arguments)]
pub fn put_frame_sized(
    home: &Path,
    slug: &str,
    name: &str,
    html: &str,
    tweak_spec: Option<&[TweakSpec]>,
    frame_id: Option<&str>,
    model: &str,
    size: Option<(f64, f64)>,
) -> Result<String, String> {
    let html = validate_frame_html(html)?;
    let mut project = load(home, slug)?;
    let now = crate::calib::stamp();
    if let Some(id) = frame_id {
        let frame = project
            .frames
            .iter_mut()
            .find(|frame| frame.id == id)
            .ok_or_else(|| format!("no frame `{id}` on this board"))?;
        // Snapshot what the frame was, then take the new content.
        push_version(frame, "refine");
        frame.html = html;
        if let Some(spec) = tweak_spec {
            frame.tweak_spec = spec.to_vec();
            frame.tweak_values.clear();
        }
        if !name.trim().is_empty() {
            frame.name = name.trim().to_string();
        }
        frame.model = model.to_string();
        frame.updated = now.clone();
        project.updated = now;
        let id = frame.id.clone();
        save(home, slug, &project)?;
        return Ok(id);
    }
    // The new frame lands to the right of the board's bounds, top-aligned
    // with the first row — the source's own placement.
    let (x, y) = if project.frames.is_empty() {
        (0.0, 0.0)
    } else {
        let right = project
            .frames
            .iter()
            .map(|frame| frame.x + frame.w)
            .fold(f64::MIN, f64::max);
        let top = project
            .frames
            .iter()
            .map(|frame| frame.y)
            .fold(f64::MAX, f64::min);
        (right + 96.0, top)
    };
    let frame = Frame {
        id: new_id("f"),
        name: if name.trim().is_empty() {
            "untitled".to_string()
        } else {
            name.trim().to_string()
        },
        description: String::new(),
        html,
        x,
        y,
        w: size.map(|(w, _)| w).unwrap_or(VIEWPORT_PRESETS[0].1),
        h: size.map(|(_, h)| h).unwrap_or(VIEWPORT_PRESETS[0].2),
        tweak_spec: tweak_spec.map(<[TweakSpec]>::to_vec).unwrap_or_default(),
        tweak_values: BTreeMap::new(),
        model: model.to_string(),
        set: None,
        gen_meta: None,
        versions: Vec::new(),
        pins: Vec::new(),
        review: None,
        created: now.clone(),
        updated: now.clone(),
    };
    let mut frame = frame;
    push_version(&mut frame, "generate");
    let id = frame.id.clone();
    project.frames.push(frame);
    project.updated = now;
    save(home, slug, &project)?;
    Ok(id)
}

/// Snapshot a frame's content as a version, capping the list. The
/// snapshot is the frame's *current* content, so a restore is itself
/// snapshotted before it overwrites.
fn push_version(frame: &mut Frame, source: &str) {
    frame.versions.push(Version {
        id: new_id("v"),
        name: frame.name.clone(),
        html: frame.html.clone(),
        tweak_values: frame.tweak_values.clone(),
        source: source.to_string(),
        created: crate::calib::stamp(),
    });
    while frame.versions.len() > MAX_VERSIONS {
        frame.versions.remove(0);
    }
}

/// The geometry and tweak write the face makes: position, size, name,
/// description, tweak values — never the HTML (that is a version event).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FrameUpdate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tweak_values: Option<BTreeMap<String, Value>>,
    /// The array position — z-order, since frames render in array order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z: Option<usize>,
}

pub fn update_frame(
    home: &Path,
    slug: &str,
    frame_id: &str,
    update: &FrameUpdate,
) -> Result<(), String> {
    let mut project = load(home, slug)?;
    let index = project
        .frames
        .iter()
        .position(|frame| frame.id == frame_id)
        .ok_or_else(|| format!("no frame `{frame_id}` on this board"))?;
    let now = crate::calib::stamp();
    {
        let frame = &mut project.frames[index];
        if let Some(x) = update.x {
            frame.x = x;
        }
        if let Some(y) = update.y {
            frame.y = y;
        }
        if let Some(w) = update.w {
            frame.w = w.clamp(120.0, 4096.0);
        }
        if let Some(h) = update.h {
            frame.h = h.clamp(120.0, 8192.0);
        }
        if let Some(name) = &update.name {
            if !name.trim().is_empty() {
                frame.name = name.trim().to_string();
            }
        }
        if let Some(description) = &update.description {
            frame.description = description.clone();
        }
        if let Some(values) = &update.tweak_values {
            frame.tweak_values = values.clone();
        }
        frame.updated = now.clone();
    }
    if let Some(z) = update.z {
        if z < project.frames.len() {
            let frame = project.frames.remove(index);
            project.frames.insert(z, frame);
        }
    }
    project.updated = now;
    save(home, slug, &project)
}

pub fn delete_frame(home: &Path, slug: &str, frame_id: &str) -> Result<(), String> {
    let mut project = load(home, slug)?;
    let before = project.frames.len();
    project.frames.retain(|frame| frame.id != frame_id);
    if project.frames.len() == before {
        return Err(format!("no frame `{frame_id}` on this board"));
    }
    project.updated = crate::calib::stamp();
    save(home, slug, &project)
}

/// Duplicate a frame to the right of itself (the source's offset), the
/// copy carrying the same content and spec but its own identity and a
/// fresh history.
pub fn duplicate_frame(home: &Path, slug: &str, frame_id: &str) -> Result<String, String> {
    let mut project = load(home, slug)?;
    let source = project
        .frames
        .iter()
        .find(|frame| frame.id == frame_id)
        .ok_or_else(|| format!("no frame `{frame_id}` on this board"))?
        .clone();
    let now = crate::calib::stamp();
    let copy = Frame {
        id: new_id("f"),
        name: format!("{} copy", source.name),
        x: source.x + source.w + 96.0,
        y: source.y,
        created: now.clone(),
        updated: now.clone(),
        versions: Vec::new(),
        pins: Vec::new(),
        review: None,
        ..source
    };
    let id = copy.id.clone();
    project.frames.push(copy);
    project.updated = now;
    save(home, slug, &project)?;
    Ok(id)
}

/// Restore a version: the frame's current content is snapshotted first
/// (so the restore itself is undoable), then the version's content wins.
pub fn restore_version(
    home: &Path,
    slug: &str,
    frame_id: &str,
    version_id: &str,
) -> Result<(), String> {
    let mut project = load(home, slug)?;
    let frame = project
        .frames
        .iter_mut()
        .find(|frame| frame.id == frame_id)
        .ok_or_else(|| format!("no frame `{frame_id}` on this board"))?;
    let version = frame
        .versions
        .iter()
        .find(|version| version.id == version_id)
        .ok_or_else(|| format!("no version `{version_id}` on this frame"))?
        .clone();
    push_version(frame, "restore");
    frame.html = version.html;
    frame.tweak_values = version.tweak_values;
    frame.updated = crate::calib::stamp();
    project.updated = frame.updated.clone();
    save(home, slug, &project)
}

/// The `frame` tool's handler: read the model's call, validate the page,
/// and put it on the board its `project` names. Returns the slug, the
/// frame id, and whether the call added a frame (true) or refined one.
///
/// The tool refuses by name for the same reasons the writer does, plus
/// the tool-specific ones: a `frame_id` that is not on the board, a
/// tweak spec that does not deserialize. The model reads the refusal and
/// fixes the one thing.
pub fn frame_from_tool(home: &Path, arguments: &Value) -> Result<(String, String, bool), String> {
    let slug = arguments
        .get("project")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|slug| !slug.is_empty())
        .ok_or_else(|| {
            "the call names no project; pass the canvas board's slug the brief named".to_string()
        })?
        .to_string();
    let name = arguments
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("untitled");
    let html = arguments
        .get("html")
        .and_then(Value::as_str)
        .ok_or_else(|| "the call carries no html".to_string())?;
    let frame_id = arguments
        .get("frame_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty());
    let spec: Option<Vec<TweakSpec>> = match arguments.get("tweak_spec") {
        None | Some(Value::Null) => None,
        Some(value) => Some(serde_json::from_value(value.clone()).map_err(|error| {
            format!("the tweak_spec is not a tweak list: {error} — each needs key, label, kind and css_var")
        })?),
    };
    if !exists(home, &slug) {
        return Err(format!(
            "no canvas board `{slug}` — create the board first, or check the slug the brief named"
        ));
    }
    // The page's assets: each entry either names one the board keeps or
    // hands a file to be read, MIME-checked and stored first. Both end the
    // same way — one `data:` URI per name — and the page's placeholders
    // are inlined through that single map.
    let mut resolved: BTreeMap<String, String> = BTreeMap::new();
    if let Some(list) = arguments.get("assets").and_then(Value::as_array) {
        for entry in list {
            let name = entry
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| {
                    "an asset entry names nothing; each is `{name, path}`".to_string()
                })?;
            if let Some(path) = entry
                .get("path")
                .and_then(Value::as_str)
                .filter(|path| !path.trim().is_empty())
            {
                let bytes = read_asset_file(home, path)?;
                put_asset(home, &slug, name, &bytes, None)?;
            }
            resolved.insert(slug_of(name), asset_data_uri(home, &slug, name)?);
        }
    }
    let html = substitute_assets(html, &resolved)?;
    let html = html.as_str();
    let added = frame_id.is_none();
    // The page's viewport: the brief's own rule decides the default, and
    // an explicit word from the call wins.
    let (_, w, h) = sizing_for(
        arguments
            .get("viewport")
            .and_then(Value::as_str)
            .unwrap_or("desktop"),
    );
    let id = put_frame_sized(
        home,
        &slug,
        name,
        html,
        spec.as_deref(),
        frame_id,
        "designer",
        Some((w, h)),
    )?;
    // A page set: the frame joins the set the brief named, so the board
    // groups it, checks its tokens and links, and hands the set over as
    // one packet.
    if let Some(set) = arguments
        .get("set")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|set| !set.is_empty())
    {
        set_frame_set(home, &slug, &id, set)?;
    }
    // The board's brand seeds the page's color defaults: a spec key that
    // names one of the brand's tokens starts at the brand's value, so a
    // branded board lands pages in its own palette without a second pass.
    seed_brand_tokens(home, &slug, &id)?;
    Ok((slug, id, added))
}

/// The bytes of one file the `frame` tool named. A model may point at a
/// file inside the fleet home or inside the run's own working directory —
/// nowhere else, so a call cannot read the machine behind the owner's
/// back.
fn read_asset_file(home: &Path, path: &str) -> Result<Vec<u8>, String> {
    let named = Path::new(path);
    let resolved = if named.is_absolute() {
        named.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("the working directory could not be read: {error}"))?
            .join(named)
    };
    let canonical = resolved
        .canonicalize()
        .map_err(|error| format!("the asset `{path}` could not be read: {error}"))?;
    let home_root = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
    let cwd = std::env::current_dir()
        .map(|dir| dir.canonicalize().unwrap_or(dir))
        .unwrap_or_default();
    if !canonical.starts_with(&home_root) && !canonical.starts_with(&cwd) {
        return Err(format!(
            "the asset `{path}` is outside the fleet home and the working directory — pass a path inside them, or upload it to the board"
        ));
    }
    std::fs::read(&canonical)
        .map_err(|error| format!("the asset `{path}` could not be read: {error}"))
}

/// Give one frame the board's brand as its color defaults. Only a color
/// tweak takes a token — a slider's value is a number and a token is a
/// declaration — and a value the frame already carries is left alone.
fn seed_brand_tokens(home: &Path, slug: &str, frame_id: &str) -> Result<(), String> {
    let mut project = load(home, slug)?;
    let Some(brand_slug) = project.brand.clone() else {
        return Ok(());
    };
    let Ok(brand) = load_brand(home, &brand_slug) else {
        return Ok(());
    };
    let Some(frame) = project.frames.iter_mut().find(|frame| frame.id == frame_id) else {
        return Ok(());
    };
    let mut seeded = false;
    for spec in &frame.tweak_spec {
        if spec.kind != "color" || frame.tweak_values.contains_key(&spec.key) {
            continue;
        }
        if let Some(value) = brand.tokens.get(&spec.key) {
            frame
                .tweak_values
                .insert(spec.key.clone(), Value::String(value.clone()));
            seeded = true;
        }
    }
    if seeded {
        project.updated = crate::calib::stamp();
        save(home, slug, &project)?;
    }
    Ok(())
}

// --- pins -------------------------------------------------------------------

pub fn add_pin(
    home: &Path,
    slug: &str,
    frame_id: &str,
    x: f64,
    y: f64,
    text: &str,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("a pin needs its comment".to_string());
    }
    let mut project = load(home, slug)?;
    let frame = project
        .frames
        .iter_mut()
        .find(|frame| frame.id == frame_id)
        .ok_or_else(|| format!("no frame `{frame_id}` on this board"))?;
    let pin = Pin {
        id: new_id("p"),
        x: x.clamp(0.0, 1.0),
        y: y.clamp(0.0, 1.0),
        text: text.trim().to_string(),
        source: "owner".to_string(),
        created: crate::calib::stamp(),
        resolved_frame_id: None,
        replies: Vec::new(),
    };
    let id = pin.id.clone();
    frame.pins.push(pin);
    project.updated = crate::calib::stamp();
    save(home, slug, &project)?;
    Ok(id)
}

pub fn reply_pin(
    home: &Path,
    slug: &str,
    frame_id: &str,
    pin_id: &str,
    body: &str,
) -> Result<(), String> {
    if body.trim().is_empty() {
        return Err("a reply needs its text".to_string());
    }
    let mut project = load(home, slug)?;
    let frame = project
        .frames
        .iter_mut()
        .find(|frame| frame.id == frame_id)
        .ok_or_else(|| format!("no frame `{frame_id}` on this board"))?;
    let pin = frame
        .pins
        .iter_mut()
        .find(|pin| pin.id == pin_id)
        .ok_or_else(|| format!("no pin `{pin_id}` on this frame"))?;
    pin.replies.push(Reply {
        author: "owner".to_string(),
        body: body.trim().to_string(),
        created: crate::calib::stamp(),
    });
    project.updated = crate::calib::stamp();
    save(home, slug, &project)
}

/// Mark a pin resolved by the frame a fix made for it (or by `None`,
/// which clears the mark and returns the pin to open).
pub fn resolve_pin(
    home: &Path,
    slug: &str,
    frame_id: &str,
    pin_id: &str,
    resolved_frame_id: Option<&str>,
) -> Result<(), String> {
    let mut project = load(home, slug)?;
    let frame = project
        .frames
        .iter_mut()
        .find(|frame| frame.id == frame_id)
        .ok_or_else(|| format!("no frame `{frame_id}` on this board"))?;
    let pin = frame
        .pins
        .iter_mut()
        .find(|pin| pin.id == pin_id)
        .ok_or_else(|| format!("no pin `{pin_id}` on this frame"))?;
    pin.resolved_frame_id = resolved_frame_id.map(str::to_string);
    project.updated = crate::calib::stamp();
    save(home, slug, &project)
}

/// Store a review on a frame, capping the retained list. The review is a
/// structured value: scores, strengths, fixes, and the scan's facts.
pub fn put_review(home: &Path, slug: &str, frame_id: &str, review: Value) -> Result<(), String> {
    let mut project = load(home, slug)?;
    let frame = project
        .frames
        .iter_mut()
        .find(|frame| frame.id == frame_id)
        .ok_or_else(|| format!("no frame `{frame_id}` on this board"))?;
    // Keep one latest review plus its history, bounded by the cap.
    let mut history = frame
        .review
        .as_ref()
        .and_then(|review| review.get("history").cloned())
        .and_then(|history| history.as_array().cloned())
        .unwrap_or_default();
    if let Some(previous) = frame.review.take() {
        let mut previous = previous;
        if let Some(object) = previous.as_object_mut() {
            object.remove("history");
        }
        history.push(previous);
    }
    while history.len() > MAX_REVIEWS {
        history.remove(0);
    }
    let mut review = review;
    if let Some(object) = review.as_object_mut() {
        object.insert("history".to_string(), Value::Array(history));
    }
    frame.review = Some(review);
    project.updated = crate::calib::stamp();
    save(home, slug, &project)
}

/// The eight scores a review names — design-canvas's own set, kept closed
/// so a card can render every one (a missing score reads as `—`, never as
/// a row that vanishes).
pub const REVIEW_SCORES: &[&str] = &[
    "hierarchy",
    "spacing",
    "contrast",
    "accessibility",
    "polish",
    "motion",
    "aiux",
    "responsive",
];

/// The review ask one frame composes: two captures (desktop and phone),
/// the vision read over both, the eight scores, and the tool that stores
/// the result. Rust composes it so the composer, the CLI and a cron job
/// ask for the same review.
pub fn review_task(
    slug: &str,
    frame_name: &str,
    frame_id: &str,
    captures: &[(String, String)],
) -> String {
    let mut task = format!(
        "Review the design frame `{frame_name}` (project `{slug}`, frame_id `{frame_id}`).\n\n"
    );
    if captures.is_empty() {
        task.push_str(
            "Open the frame's preview with the browse tool and screenshot it at the frame's own size and again at a phone's (390×844), read both screenshots with the vision tool, then ",
        );
    } else {
        task.push_str("Two captures of the page are on disk:\n");
        for (label, path) in captures {
            task.push_str(&format!("- {label}: {path}\n"));
        }
        task.push_str("\nRead both with the vision tool — the desktop capture judges layout and type, the phone one judges the small-screen reading — then ");
    }
    task.push_str(&format!(
        "store the review with `review_frame` (project `{slug}`, frame_id `{frame_id}`): one score 0–10 for each of {}, a one or two sentence summary, what the page gets right, and the highest-value fixes — one thing each.",
        REVIEW_SCORES.join(", ")
    ));
    task
}

/// The `review_frame` tool's handler: read a structured review and store
/// it on the frame. The review is what a review run produced — the vision
/// model's scores and fixes, beside the deterministic scan's facts the
/// read ships anyway. Validation is the shape's: a scores map of numbers
/// and bounded text lists.
pub fn review_from_tool(home: &Path, arguments: &Value) -> Result<(String, String), String> {
    let slug = arguments
        .get("project")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|slug| !slug.is_empty())
        .ok_or_else(|| "the call names no project".to_string())?
        .to_string();
    let frame_id = arguments
        .get("frame_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "the call names no frame_id".to_string())?
        .to_string();
    let scores = match arguments.get("scores") {
        Some(Value::Object(map)) => {
            let mut clean: BTreeMap<String, f64> = BTreeMap::new();
            for (key, value) in map {
                let value = value
                    .as_f64()
                    .ok_or_else(|| format!("the score `{key}` is not a number"))?;
                clean.insert(key.clone(), value.clamp(0.0, 10.0));
            }
            clean
        }
        Some(_) => return Err("the scores are not a map of numbers".to_string()),
        None => BTreeMap::new(),
    };
    let list = |key: &str| -> Vec<String> {
        arguments
            .get(key)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .take(12)
                    .map(|item| item.chars().take(400).collect::<String>())
                    .collect()
            })
            .unwrap_or_default()
    };
    let review = serde_json::json!({
        "kind": "vision",
        "scores": scores,
        "summary": arguments
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .chars()
            .take(800)
            .collect::<String>(),
        "strengths": list("strengths"),
        "fixes": list("fixes"),
        "reviewed": crate::calib::stamp(),
    });
    put_review(home, &slug, &frame_id, review)?;
    Ok((slug, frame_id))
}

// --- the sanitized mutation path: tweak injection ---------------------------

/// Escape a value for a CSS custom property: the injection is a style
/// block built from owner- and model-written values, so braces, semicolons
/// and angle brackets never reach the sheet.
fn css_value(value: &Value, spec: &TweakSpec) -> Option<String> {
    let raw = match (spec.kind.as_str(), value) {
        ("toggle", Value::Bool(on)) => if *on { "1" } else { "0" }.to_string(),
        ("toggle", _) => return None,
        (_, Value::Number(number)) => {
            let unit = spec.unit.clone().unwrap_or_default();
            format!("{number}{unit}")
        }
        (_, Value::String(text)) => {
            if text
                .chars()
                .any(|ch| ch == '{' || ch == '}' || ch == '<' || ch == '>' || ch == ';')
            {
                return None;
            }
            // A unit belongs to a length; a colour or a keyword carries
            // its own form and `#ff0000rem` is never a value.
            let unit = if spec.kind == "color" || spec.kind == "select" {
                String::new()
            } else {
                spec.unit.clone().unwrap_or_default()
            };
            format!("{text}{unit}")
        }
        _ => return None,
    };
    Some(raw)
}

/// The one sanctioned frame mutation: insert (or replace) the
/// `<style id="fleet-tweaks">` block that carries the current tweak
/// values as `:root` overrides. Every consumer — the live `srcdoc`, the
/// preview route, the ZIP's HTML, the screenshot's source — runs this,
/// so no two of them can disagree.
pub fn inject_tweaks(html: &str, specs: &[TweakSpec], values: &BTreeMap<String, Value>) -> String {
    let mut declarations = String::new();
    for spec in specs {
        let Some(value) = values.get(&spec.key) else {
            continue;
        };
        if let Some(text) = css_value(value, spec) {
            declarations.push_str(&format!("--{}: {};", spec.css_var, text));
        }
    }
    let style = format!("<style id=\"fleet-tweaks\">:root{{{declarations}}}</style>");
    // Replace an existing block first, so a second pass never stacks.
    if let Some(start) = html.find("<style id=\"fleet-tweaks\">") {
        if let Some(end) = html[start..].find("</style>") {
            let end = start + end + "</style>".len();
            let mut out = String::with_capacity(html.len() + style.len());
            out.push_str(&html[..start]);
            out.push_str(&style);
            out.push_str(&html[end..]);
            return out;
        }
    }
    // Otherwise inject before </head>, or before the first <body>, or at
    // the very start when the document is bare.
    for marker in ["</head>", "<body"] {
        if let Some(at) = html.to_ascii_lowercase().find(marker) {
            let mut out = String::with_capacity(html.len() + style.len());
            out.push_str(&html[..at]);
            out.push_str(&style);
            out.push_str(&html[at..]);
            return out;
        }
    }
    format!("{style}{html}")
}

/// The `:root` token map a frame declares, from its HTML — the raw
/// custom properties, before any tweak override. Used by the token
/// exports and by the a11y scan's contrast pairs.
pub fn extract_root_tokens(html: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut at = 0;
    while let Some(found) = lower[at..].find(":root") {
        let start = at + found + ":root".len();
        let Some(open) = html[start..].find('{') else {
            break;
        };
        let body_start = start + open + 1;
        let Some(close) = html[body_start..].find('}') else {
            break;
        };
        let body = &html[body_start..body_start + close];
        for pair in body.split(';') {
            let Some((name, value)) = pair.split_once(':') else {
                continue;
            };
            let name = name.trim();
            let value = value.trim();
            if !name.starts_with("--") || value.is_empty() {
                continue;
            }
            let name = name.trim_start_matches("--").to_string();
            if !out.iter().any(|(known, _)| *known == name) {
                out.push((name, value.to_string()));
            }
        }
        at = body_start + close;
        if at >= html.len() {
            break;
        }
    }
    out
}

/// The tokens a frame's HTML declares, as a `:root` stylesheet — the
/// hand-off's `tokens.css`.
pub fn tokens_css(html: &str) -> String {
    let tokens = extract_root_tokens(html);
    let mut out = String::from(":root {\n");
    for (name, value) in &tokens {
        out.push_str(&format!("  --{name}: {value};\n"));
    }
    out.push_str("}\n");
    out
}

/// The tokens as W3C-draft JSON: `{group: {token: { "$value": "…" }}}`,
/// flattened into one group named after nothing but the frame — the
/// hand-off contract's simplest valid shape.
pub fn tokens_json(html: &str, name: &str) -> Value {
    let tokens = extract_root_tokens(html);
    let mut map = serde_json::Map::new();
    for (token, value) in tokens {
        map.insert(
            token,
            serde_json::json!({ "$type": "color", "$value": value }),
        );
    }
    serde_json::json!({ name: map })
}

// --- the a11y scan ----------------------------------------------------------

/// One scan fact: a check, its one-line detail, and its verdict.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct A11yCheck {
    /// `contrast` | `alt` | `labels` | `focus` | `viewport`.
    pub kind: String,
    pub detail: String,
    pub pass: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct A11yScan {
    pub checks: Vec<A11yCheck>,
    pub passed: usize,
    pub total: usize,
}

fn parse_colour(text: &str) -> Option<(f64, f64, f64)> {
    let text = text.trim().to_ascii_lowercase();
    if let Some(hex) = text.strip_prefix('#') {
        let hex: String = hex.chars().take(6).collect();
        let full = match hex.len() {
            3 => hex.chars().flat_map(|ch| [ch, ch]).collect::<String>(),
            6 => hex,
            _ => return None,
        };
        let r = u8::from_str_radix(&full[0..2], 16).ok()? as f64;
        let g = u8::from_str_radix(&full[2..4], 16).ok()? as f64;
        let b = u8::from_str_radix(&full[4..6], 16).ok()? as f64;
        return Some((r, g, b));
    }
    for (prefix, close) in [("rgb(", ')'), ("rgba(", ')')] {
        if let Some(rest) = text.strip_prefix(prefix) {
            let rest = rest.split(close).next()?;
            let parts: Vec<f64> = rest
                .split([',', ' '])
                .filter(|part| !part.trim().is_empty())
                .filter_map(|part| part.trim().parse::<f64>().ok())
                .collect();
            if parts.len() >= 3 {
                return Some((parts[0], parts[1], parts[2]));
            }
        }
    }
    None
}

fn relative_luminance((r, g, b): (f64, f64, f64)) -> f64 {
    let channel = |value: f64| {
        let value = value / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

fn contrast_ratio(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (light, dark) = if la >= lb { (la, lb) } else { (lb, la) };
    (light + 0.05) / (dark + 0.05)
}

/// The deterministic a11y scan: contrast pairs computed from the frame's
/// own `:root` tokens, plus the structural checks a string scan can make
/// honestly — `<img>` alt, form labels, a focus style, the viewport meta.
/// A flag, not a certification (the write guard's own idiom): every check
/// names what it read.
pub fn a11y_scan(html: &str) -> A11yScan {
    let mut checks: Vec<A11yCheck> = Vec::new();
    let tokens = extract_root_tokens(html);
    let find = |name: &str| -> Option<(f64, f64, f64)> {
        tokens
            .iter()
            .find(|(token, _)| token == name)
            .and_then(|(_, value)| parse_colour(value))
    };
    // The canonical pairs a generated page uses; any pair whose both ends
    // resolve is measured.
    let pairs = [
        ("ink on page", "ink", "page", 4.5),
        ("ink-secondary on page", "ink-secondary", "page", 4.5),
        ("ink on surface", "ink", "surface", 4.5),
        ("accent on page", "accent", "page", 3.0),
    ];
    for (label, fg, bg, floor) in pairs {
        if let (Some(fg), Some(bg)) = (find(fg), find(bg)) {
            let ratio = contrast_ratio(fg, bg);
            checks.push(A11yCheck {
                kind: "contrast".to_string(),
                detail: format!("{label}: {ratio:.2}:1 (floor {floor})"),
                pass: ratio >= floor,
            });
        }
    }
    let lower = html.to_ascii_lowercase();
    // Every <img> carries a non-empty alt (alt="" is a deliberate
    // decorative mark and passes).
    let img_count = lower.matches("<img").count();
    let img_alt = lower.matches("<img").filter(|_| true).count();
    let _ = img_alt;
    let missing_alt = count_imgs_without_alt(html);
    checks.push(A11yCheck {
        kind: "alt".to_string(),
        detail: if img_count == 0 {
            "no <img> elements".to_string()
        } else {
            format!("{img_count} image(s), {missing_alt} missing alt")
        },
        pass: missing_alt == 0,
    });
    // A form control with no label and no aria-label is a naming gap.
    let inputs = count_unlabelled_controls(html);
    checks.push(A11yCheck {
        kind: "labels".to_string(),
        detail: format!("{inputs} control(s) without a label or aria-label"),
        pass: inputs == 0,
    });
    checks.push(A11yCheck {
        kind: "focus".to_string(),
        detail: ":focus style".to_string(),
        pass: lower.contains(":focus") || lower.contains("focus-visible"),
    });
    checks.push(A11yCheck {
        kind: "viewport".to_string(),
        detail: "viewport meta".to_string(),
        pass: lower.contains("name=\"viewport\"") || lower.contains("name='viewport'"),
    });
    let passed = checks.iter().filter(|check| check.pass).count();
    let total = checks.len();
    A11yScan {
        checks,
        passed,
        total,
    }
}

/// `<img …>` tags whose attributes carry `alt=` with a value — a tolerant
/// scan: it reads each tag's text up to its closing `>`, not a parse.
fn count_imgs_without_alt(html: &str) -> usize {
    let lower = html.to_ascii_lowercase();
    let mut missing = 0;
    let mut at = 0;
    while let Some(found) = lower[at..].find("<img") {
        let start = at + found;
        let Some(close) = html[start..].find('>') else {
            break;
        };
        let tag = &lower[start..start + close];
        if !tag.contains("alt=") {
            missing += 1;
        }
        at = start + close;
    }
    missing
}

fn count_unlabelled_controls(html: &str) -> usize {
    let lower = html.to_ascii_lowercase();
    let mut unlabelled = 0;
    for tag in ["<input", "<select", "<textarea"] {
        let mut at = 0;
        while let Some(found) = lower[at..].find(tag) {
            let start = at + found;
            let Some(close) = lower[start..].find('>') else {
                break;
            };
            let body = &lower[start..start + close];
            let labelled = body.contains("aria-label")
                || body.contains("aria-labelledby")
                || body.contains("id=");
            let is_button_like = body.contains("type=\"submit\"")
                || body.contains("type=\"hidden\"")
                || body.contains("type='submit'")
                || body.contains("type='hidden'");
            if !labelled && !is_button_like {
                unlabelled += 1;
            }
            at = start + close;
        }
    }
    unlabelled
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-canvas-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const PAGE: &str = "<!doctype html><html><head><style>:root{--page:#0b0e14;--ink:#e6e9ef;--accent:#3b6cf6}body{color:var(--ink);background:var(--page)}img{max-width:100%}:focus{outline:2px solid var(--accent)}</style><meta name=\"viewport\" content=\"width=device-width\"></head><body><h1>hi</h1><img src=\"data:image/png;base64,AA\" alt=\"a mark\"><form><input aria-label=\"email\"></form></body></html>";

    /// The design surface's own model set (the owner's ruling,
    /// 2026-10-31): three models, in the owner's order, and not a routing
    /// chain — a design run pins the one this page names.
    #[test]
    fn the_design_page_carries_the_owners_three_models() {
        assert_eq!(
            DESIGN_MODELS,
            &[
                ("deepseek", "deepseek-flash"),
                ("zai-coding-plan", "GLM-5.3-Flash"),
                ("ollama-cloud", "glm-5.3-flash"),
            ],
            "the owner's designation, in their order"
        );
        // A picker offers these and only these: the head is strong at a
        // page, and the third is the same family on another host.
        assert_eq!(DESIGN_MODELS.len(), 3);
    }

    /// W7: the sanitizer loses the dangerous parts and keeps the picture.
    #[test]
    fn the_svg_sanitizer_strips_the_script_and_keeps_the_picture() {
        // A clean file is unchanged, byte for byte.
        let clean = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 0 L10 10" stroke="red"/></svg>"#;
        assert_eq!(svg_sanitize(clean).unwrap(), clean);
        // A handler, a script target and an external reference all go;
        // the shape stays.
        let hostile = r#"<svg onload="alert(1)" xmlns="http://www.w3.org/2000/svg"><a href="javascript:alert(1)"><circle r="2"/></a><image href="https://evil.example/x.png"/></svg>"#;
        let cleaned = svg_sanitize(hostile).unwrap();
        assert!(!cleaned.contains("onload"), "{cleaned}");
        assert!(!cleaned.contains("javascript:"), "{cleaned}");
        assert!(!cleaned.contains("evil.example"), "{cleaned}");
        assert!(cleaned.contains("<circle r=\"2\"/>"), "{cleaned}");
        // An in-document reference and a data URI stay: they reach
        // nothing outside the file.
        let local = r##"<svg xmlns="http://www.w3.org/2000/svg"><image href="data:image/png;base64,AAAA"/><use xlink:href="#mark"/></svg>"##;
        assert_eq!(svg_sanitize(local).unwrap(), local);
        // Text that is not an SVG is refused by name.
        let error = svg_sanitize("<html><body>not a mark</body></html>").unwrap_err();
        assert!(error.contains("not an SVG"), "{error}");
    }

    /// W14: a page becomes a React component (attrs renamed, styles as
    /// objects, scripts held out as a TODO, tweaks as props) and the token
    /// diff names exactly the variables that changed.
    #[test]
    fn a_frame_becomes_a_react_component() {
        let tweaks: Vec<TweakSpec> = serde_json::from_value(serde_json::json!([
            {"key": "accent", "label": "the accent", "kind": "color",
             "default": "#3b6cf6", "css_var": "accent"},
            {"key": "density", "label": "density", "kind": "slider",
             "min": 0.8, "max": 1.4, "step": 0.05, "default": 1, "css_var": "density"}
        ]))
        .unwrap();
        let values = BTreeMap::from([("accent".to_string(), serde_json::json!("#b4552d"))]);
        let page = "<!doctype html><html><head><style>:root{--ink:#111;--accent:#b4552d}</style></head>\
<body><div class=\"card\" style=\"color: var(--ink); margin-top: 8px\"><label for=\"e\">Email</label>\
<input id=\"e\" type=\"text\"><br><img src=\"data:image/png;base64,AA\" alt=\"a mark\"></div></body></html>";
        let tsx = to_tsx("Hero panel", page, &tweaks, &values);
        assert!(tsx.contains("export default function HeroPanel("), "{tsx}");
        assert!(tsx.contains("className=\"card\""), "{tsx}");
        assert!(tsx.contains("htmlFor=\"e\""), "{tsx}");
        assert!(
            tsx.contains("style={{ color: \"var(--ink)\", marginTop: \"8px\" }}"),
            "{tsx}"
        );
        // Void tags self-close; the mark keeps its data URI.
        assert!(tsx.contains("<br />"), "{tsx}");
        assert!(
            tsx.contains("<img src=\"data:image/png;base64,AA\" alt=\"a mark\" />"),
            "{tsx}"
        );
        // The tweaks are props, and each one overrides its custom property.
        assert!(tsx.contains("accent = \"#b4552d\","), "{tsx}");
        assert!(tsx.contains("density?: number;"), "{tsx}");
        assert!(tsx.contains("--accent: ${accent} as any,"), "{tsx}");
        assert!(tsx.contains("--density: ${density} as any,"), "{tsx}");
        // A script is held out of the component and named as a TODO.
        let scripted = "<!doctype html><html><head><style>:root{--ink:#111}</style></head>\
<body><p>hello</p><script>console.log(\"hi\")</script></body></html>";
        let tsx = to_tsx("Scripted", scripted, &[], &BTreeMap::new());
        assert!(tsx.contains("TODO: port inline script(s)"), "{tsx}");
        assert!(!tsx.contains("<script"), "{tsx}");
        assert!(tsx.contains("hello"), "{tsx}");
        // A name nothing can start an identifier with still yields one.
        assert_eq!(to_component_name("3 hero — v2"), "Frame3HeroV2");
    }

    /// W14: the diff names added, dropped and changed tokens, in order.
    #[test]
    fn the_token_diff_names_the_changed_variables() {
        let before = "<style>:root{--ink:#111111;--accent:#111111;--gone:1px}</style>";
        let after = "<style>:root{--ink:#111111;--accent:#b4552d;--new:2px}</style>";
        let changes = token_diff(before, after);
        assert_eq!(changes.len(), 3, "{changes:?}");
        assert_eq!(changes[0].token, "accent");
        assert_eq!(changes[0].before.as_deref(), Some("#111111"));
        assert_eq!(changes[0].after.as_deref(), Some("#b4552d"));
        assert_eq!(changes[1].token, "gone");
        assert_eq!(changes[1].after, None);
        assert_eq!(changes[2].token, "new");
        assert_eq!(changes[2].before, None);
        // An unchanged page has nothing to name.
        assert!(token_diff(before, before).is_empty());
    }

    /// W14: a selection packet holds exactly the frames the owner checked —
    /// no more, and none of the board's other pages.
    #[test]
    fn a_selection_packet_holds_only_the_frames_picked() {
        let home = scratch("selection");
        let (slug, _) = create(&home, "Board").unwrap();
        let one = put_frame(&home, &slug, "one", PAGE, None, None, "m").unwrap();
        let two = put_frame(&home, &slug, "two", PAGE, None, None, "m").unwrap();
        let three = put_frame(&home, &slug, "three", PAGE, None, None, "m").unwrap();
        let bytes = selection_packet(
            &home,
            &slug,
            &[one.clone(), three.clone()],
            &BTreeMap::new(),
        )
        .unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("one.html"), "the first picked page");
        assert!(text.contains("three.html"), "the second picked page");
        assert!(
            !text.contains("two.html"),
            "an unpicked page is not in the packet"
        );
        assert_eq!(&bytes[0..2], b"PK");
        // An empty selection is refused by name.
        let error = selection_packet(&home, &slug, &[], &BTreeMap::new()).unwrap_err();
        assert!(error.contains("no frame"), "{error}");
        let _ = two;
    }

    /// W13: the review names eight scores, the ask drives two captures,
    /// and the history the store keeps is bounded.
    #[test]
    fn a_review_names_eight_scores_and_its_ask_drives_two_captures() {
        assert_eq!(
            REVIEW_SCORES,
            &[
                "hierarchy",
                "spacing",
                "contrast",
                "accessibility",
                "polish",
                "motion",
                "aiux",
                "responsive",
            ]
        );
        let captures = vec![
            ("desktop".to_string(), "/tmp/hero-desktop.png".to_string()),
            ("phone".to_string(), "/tmp/hero-phone.png".to_string()),
        ];
        let task = review_task("board", "hero", "f-one", &captures);
        assert!(task.contains("/tmp/hero-desktop.png"), "{task}");
        assert!(task.contains("/tmp/hero-phone.png"), "{task}");
        assert!(
            task.contains(
                "hierarchy, spacing, contrast, accessibility, polish, motion, aiux, responsive"
            ),
            "{task}"
        );
        assert!(task.contains("review_frame"), "{task}");
        // A review run with no captures yet still asks for both sizes.
        let bare = review_task("board", "hero", "f-one", &[]);
        assert!(bare.contains("390×844"), "{bare}");
        // The history is bounded: the newest review plus the ten before it.
        let home = scratch("review-history");
        let (slug, _) = create(&home, "Reviewed").unwrap();
        let id = put_frame(&home, &slug, "hero", PAGE, None, None, "m").unwrap();
        for at in 0..12 {
            put_review(
                &home,
                &slug,
                &id,
                serde_json::json!({
                    "kind": "vision",
                    "scores": {"hierarchy": at},
                    "summary": format!("review {at}"),
                }),
            )
            .unwrap();
        }
        let project = load(&home, &slug).unwrap();
        let frame = project.frames.iter().find(|frame| frame.id == id).unwrap();
        let review = frame.review.as_ref().expect("a review");
        assert_eq!(review["summary"], "review 11");
        let history = review["history"].as_array().expect("history");
        assert_eq!(history.len(), MAX_REVIEWS);
        // The oldest fell off: eleven reviews land, ten are kept.
        assert_eq!(history[0]["summary"], "review 1");
        assert_eq!(history[MAX_REVIEWS - 1]["summary"], "review 10");
    }

    /// W11: a set's pages share tokens, its links resolve, its slugs are
    /// unique — and the report names what is not.
    #[test]
    fn a_page_set_is_coherent_or_names_what_is_not() {
        let home = scratch("page-set");
        let (slug, _) = create(&home, "Site").unwrap();
        let page = |title: &str, tokens: &str, links: &str| {
            format!(
                "<!doctype html><html><head><style>:root{{{tokens}}}</style></head>\
                 <body><h1>{title}</h1>{links}</body></html>"
            )
        };
        let home_id = put_frame(
            &home,
            &slug,
            "home",
            &page(
                "Home",
                "--ink:#111111;--page:#ffffff;--accent:#b4552d",
                "<a href=\"pricing\">Pricing</a><a href=\"about\">About</a>",
            ),
            None,
            None,
            "m",
        )
        .unwrap();
        // The pricing page lost `--page`, points at a ghost and shares a slug
        // with nothing — the other two do collide on `about`.
        let pricing_id = put_frame(
            &home,
            &slug,
            "pricing",
            &page(
                "Pricing",
                "--ink:#111111;--accent:#b4552d",
                "<a href=\"/about\">About</a><a href=\"ghost\">Ghost</a>",
            ),
            None,
            None,
            "m",
        )
        .unwrap();
        let about_id = put_frame(
            &home,
            &slug,
            "about",
            &page(
                "About",
                "--ink:#111111;--page:#ffffff;--accent:#b4552d",
                "<a href=\"home\">Home</a>",
            ),
            None,
            None,
            "m",
        )
        .unwrap();
        let about_dup = put_frame(
            &home,
            &slug,
            "About",
            &page("About again", "--ink:#111111;--page:#ffffff", ""),
            None,
            None,
            "m",
        )
        .unwrap();
        for id in [&home_id, &pricing_id, &about_id, &about_dup] {
            set_frame_set(&home, &slug, id, "site").unwrap();
        }
        let project = load(&home, &slug).unwrap();
        let frames = page_set(&project, "site");
        assert_eq!(frames.len(), 4);
        let report = coherence(&frames);
        // `--ink` is the one token every page declares.
        assert_eq!(report.shared, vec!["ink".to_string()], "{report:?}");
        assert!(
            report.missing.contains(&"pricing: page".to_string()),
            "{report:?}"
        );
        assert!(
            report.missing.contains(&"About: accent".to_string()),
            "{report:?}"
        );
        assert!(
            report.broken.iter().any(|line| line.contains("ghost")),
            "{report:?}"
        );
        // The `/about` link resolves, so it is not reported.
        assert!(
            !report.broken.iter().any(|line| line.contains("/about")),
            "{report:?}"
        );
        assert_eq!(report.duplicates, vec!["about".to_string()], "{report:?}");
        assert!(!report.ok());
        // A coherent set says so.
        let ids = page_set_slugs(&frames);
        let rewritten = rewrite_page_links(
            "<a href=\"pricing\">P</a><a href=\"/about\">A</a><a href=\"https://x.test/\">X</a>",
            &ids,
            "home",
        );
        assert!(rewritten.contains("href=\"pricing.html\""), "{rewritten}");
        assert!(rewritten.contains("href=\"about.html\""), "{rewritten}");
        assert!(rewritten.contains("https://x.test/"), "{rewritten}");
        // The packet carries every page, the tokens, the manifest and the
        // changelog — and it opens as a zip.
        let bytes = set_packet(&home, &slug, "site", &std::collections::BTreeMap::new()).unwrap();
        assert_eq!(&bytes[0..2], b"PK", "a zip archive");
        let text = String::from_utf8_lossy(&bytes);
        for name in [
            "home.html",
            "pricing.html",
            "about.html",
            "tokens.css",
            "manifest.json",
            "changelog.md",
        ] {
            assert!(text.contains(name), "{name} missing from the packet");
        }
        let error =
            set_packet(&home, &slug, "nope", &std::collections::BTreeMap::new()).unwrap_err();
        assert!(error.contains("nope"), "{error}");
    }

    /// W9: the reference fetch refuses what it must, by name, before any
    /// socket is opened — and a URL normalizes to one shape.
    #[test]
    fn the_reference_fetch_refuses_the_private_and_the_malformed() {
        // Only http(s); credentials are never sent.
        assert!(normalize_url("ftp://example.com")
            .unwrap_err()
            .contains("http(s)"));
        assert!(normalize_url("https://user:pw@example.com")
            .unwrap_err()
            .contains("credentials"));
        assert!(normalize_url("https://").unwrap_err().contains("host"));
        assert!(normalize_url("   ").unwrap_err().contains("no URL"));
        // A URL normalizes: host lowercased, the default port dropped.
        assert_eq!(
            normalize_url(" HTTPS://Example.COM:443/Path?q=1 ").unwrap(),
            "https://example.com/Path?q=1"
        );
        assert_eq!(
            normalize_url("http://example.com:8080/x").unwrap(),
            "http://example.com:8080/x"
        );
        // Loopback, private, link-local and local names are all private.
        for host in [
            "localhost",
            "api.localhost",
            "printer.local",
            "127.0.0.1",
            "10.0.0.7",
            "192.168.1.5",
            "172.16.4.4",
            "169.254.1.1",
            "[::1]",
            "::1",
        ] {
            assert!(is_private_host(host), "{host} must count as private");
        }
        assert!(!is_private_host("example.com"));
        assert!(!is_private_host("8.8.8.8"));
        // The fetch itself refuses a loopback target without a socket.
        let error = fetch_site("http://127.0.0.1:3080/").unwrap_err();
        assert!(error.contains("private or loopback"), "{error}");
        let error = fetch_site("http://localhost/admin").unwrap_err();
        assert!(error.contains("private or loopback"), "{error}");
    }

    /// W9: the redirect cap and the byte cap refuse by name.
    #[test]
    fn the_reference_fetch_caps_redirects_and_size() {
        // A redirect that always answers with itself runs out at the cap.
        struct Loop;
        impl SiteTransport for Loop {
            fn get(
                &self,
                url: &str,
                _pinned: Vec<std::net::SocketAddr>,
            ) -> Result<SiteResponse, String> {
                let _ = url;
                Ok(SiteResponse {
                    status: 302,
                    location: Some("https://example.com/loop".to_string()),
                    content_type: None,
                    body: String::new(),
                })
            }
        }
        let error = fetch_with("https://example.com/loop", &Loop).unwrap_err();
        assert!(error.contains("redirects more than"), "{error}");
        // A body past the cap is refused, not read.
        struct Huge;
        impl SiteTransport for Huge {
            fn get(
                &self,
                _url: &str,
                _pinned: Vec<std::net::SocketAddr>,
            ) -> Result<SiteResponse, String> {
                Ok(SiteResponse {
                    status: 200,
                    location: None,
                    content_type: Some("text/html".to_string()),
                    body: "x".repeat(MAX_SITE_BYTES + 1),
                })
            }
        }
        let error = fetch_with("https://example.com/big", &Huge).unwrap_err();
        assert!(error.contains("larger than"), "{error}");
        // A non-page content type is refused by name too.
        struct Pdf;
        impl SiteTransport for Pdf {
            fn get(
                &self,
                _url: &str,
                _pinned: Vec<std::net::SocketAddr>,
            ) -> Result<SiteResponse, String> {
                Ok(SiteResponse {
                    status: 200,
                    location: None,
                    content_type: Some("application/pdf".to_string()),
                    body: String::new(),
                })
            }
        }
        let error = fetch_with("https://example.com/doc.pdf", &Pdf).unwrap_err();
        assert!(error.contains("application/pdf"), "{error}");
    }

    /// W10: the fact scan reads a page's own declarations — palette, type,
    /// scale, radii, layout signals — and its same-site pages.
    #[test]
    fn style_facts_are_what_the_page_declares() {
        let html = r##"<!doctype html><html><head><title>Bellows Coffee</title>
<meta name="description" content="Slow coffee, roasted by hand">
<style>
:root{--ink:#1c1a17;--page:#f7f2ea;--accent:#b4552d}
body{font-family:Inter, sans-serif;color:#1c1a17;background:#f7f2ea;max-width:1200px}
h1{font-size:64px;border-radius:2px}
.card{display:grid;font-family:Georgia, serif;font-size:18px;border-radius:12px}
.row{display:flex;font-size:18px}
</style></head><body>
<a href="/menu">Menu</a><a href="https://other.example/x">Other</a>
<a href="#top">Top</a><a href="about">About</a>
<svg><path fill="#b4552d"/></svg></body></html>"##;
        let facts = style_facts("https://bellows.example/", html);
        assert_eq!(facts.title, "Bellows Coffee");
        assert_eq!(facts.description, "Slow coffee, roasted by hand");
        assert!(
            facts.palette.contains(&"#1c1a17".to_string()),
            "{:?}",
            facts.palette
        );
        assert!(
            facts.palette.contains(&"#f7f2ea".to_string()),
            "{:?}",
            facts.palette
        );
        assert!(
            facts.palette.contains(&"#b4552d".to_string()),
            "{:?}",
            facts.palette
        );
        assert!(
            facts.fonts.iter().any(|font| font.contains("Inter")),
            "{:?}",
            facts.fonts
        );
        assert!(
            facts.fonts.iter().any(|font| font.contains("Georgia")),
            "{:?}",
            facts.fonts
        );
        assert!(
            facts.sizes.contains(&"64px".to_string()),
            "{:?}",
            facts.sizes
        );
        assert!(
            facts.radii.contains(&"12px".to_string()),
            "{:?}",
            facts.radii
        );
        assert!(
            facts.layout.contains(&"grid×1".to_string()),
            "{:?}",
            facts.layout
        );
        assert!(
            facts.layout.contains(&"flex×1".to_string()),
            "{:?}",
            facts.layout
        );
        // Only the site's own links: the foreign host and the anchor drop.
        assert_eq!(facts.pages, vec!["/menu", "/about"]);
        // The block names what it found, and says so where it found none.
        let block = facts.block();
        assert!(block.contains("palette: "), "{block}");
        assert!(block.contains("pages: /menu /about"), "{block}");
        let empty = style_facts("https://blank.example/", "<html><body>x</body></html>");
        assert!(
            empty.block().contains("(no colors declared)"),
            "{}",
            empty.block()
        );
        assert!(
            empty.block().contains("(no same-site links)"),
            "{}",
            empty.block()
        );
    }

    /// The discovered pages are the site's own, deduped and capped.
    #[test]
    fn discovered_pages_are_the_sites_own_and_capped() {
        let html = "<a href=\"/a\">a</a><a href=\"/a\">a again</a><a href=\"/b#top\">b</a><a href=\"https://example.com/c\">c</a><a href=\"https://other.example/d\">d</a><a href=\"mailto:x@example.com\">mail</a><a href=\"tel:1\">tel</a>";
        let pages = discover_pages(html, "https://example.com/start");
        assert_eq!(pages, vec!["/a", "/b", "https://example.com/c"]);
        let mut many = String::new();
        for at in 0..40 {
            many.push_str(&format!("<a href=\"/p{at}\">p</a>"));
        }
        assert_eq!(
            discover_pages(&many, "https://example.com/").len(),
            MAX_DISCOVERED_PAGES
        );
    }

    /// An asset is stored, sniffed and inlined; a placeholder that names
    /// nothing is refused by name rather than left to render literally.
    #[test]
    fn an_asset_is_stored_inlined_and_refused_when_it_is_not_an_image() {
        let home = scratch("assets");
        let (slug, _) = create(&home, "Assets").unwrap();
        let png: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0];
        let asset = put_asset(&home, &slug, "Logo Mark", &png, None).unwrap();
        assert_eq!(asset.name, "logo-mark");
        assert_eq!(asset.mime, "image/png");
        let uri = asset_data_uri(&home, &slug, "logo-mark").unwrap();
        assert!(uri.starts_with("data:image/png;base64,"), "{uri}");
        let mut resolved = BTreeMap::new();
        resolved.insert("logo-mark".to_string(), uri.clone());
        let html = substitute_assets("<img src=\"{{img:logo-mark}}\">", &resolved).unwrap();
        assert!(html.contains(&uri), "{html}");
        let error = substitute_assets("<img src=\"{{img:ghost}}\">", &resolved).unwrap_err();
        assert!(error.contains("ghost"), "{error}");
        // Bytes that are not an image are refused by name…
        let error = put_asset(&home, &slug, "notes", b"plain text, no image", None).unwrap_err();
        assert!(error.contains("not an image"), "{error}");
        // …a declared type must agree with them…
        let error = put_asset(&home, &slug, "logo-mark", &png, Some("image/gif")).unwrap_err();
        assert!(error.contains("image/png"), "{error}");
        // …and the byte cap is a refusal, not a trim.
        let error =
            put_asset(&home, &slug, "big", &vec![0u8; MAX_ASSET_BYTES + 1], None).unwrap_err();
        assert!(error.contains("cap"), "{error}");
        assert_eq!(assets(&home, &slug).len(), 1);
        delete_asset(&home, &slug, "logo-mark").unwrap();
        assert!(assets(&home, &slug).is_empty());
    }

    /// An SVG is cleaned on the way in (the active elements go), an event
    /// handler is refused rather than rewritten, and the imported mark can
    /// become a page.
    #[test]
    fn an_svg_is_cleaned_and_a_page_can_come_from_it() {
        let home = scratch("svg");
        let (slug, _) = create(&home, "Marks").unwrap();
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script><circle r=\"4\"/><foreignObject><p>x</p></foreignObject></svg>";
        let asset = put_asset(&home, &slug, "dot", svg.as_bytes(), None).unwrap();
        assert_eq!(asset.mime, "image/svg+xml");
        let (_, bytes) = asset_bytes(&home, &slug, "dot").unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.to_lowercase().contains("<script"), "{text}");
        assert!(!text.to_lowercase().contains("foreignobject"), "{text}");
        assert!(text.contains("<circle"), "{text}");
        let hostile = "<svg onload=\"alert(1)\"><circle r=\"2\"/></svg>";
        put_asset(&home, &slug, "hostile", hostile.as_bytes(), None).unwrap();
        let (_, bytes) = asset_bytes(&home, &slug, "hostile").unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.contains("onload"), "{text}");
        assert!(
            text.contains("<circle"),
            "the picture still renders: {text}"
        );
        let id = frame_from_svg(&home, &slug, "dot").unwrap();
        let project = load(&home, &slug).unwrap();
        let frame = project.frames.iter().find(|frame| frame.id == id).unwrap();
        assert!(frame.html.starts_with("<!doctype html>"), "{}", frame.html);
        assert!(frame.html.contains("<circle"), "{}", frame.html);
    }

    /// The `frame` tool inlines the assets a page names: a file handed in
    /// by path is stored first, then every placeholder becomes its data
    /// URI — and a placeholder naming a file the board does not keep is
    /// refused by name.
    #[test]
    fn the_frame_tool_inlines_the_assets_the_page_names() {
        let home = scratch("tool-assets");
        let (slug, _) = create(&home, "Tool").unwrap();
        let dir = canvas_root(&home).join("incoming");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("mark.png");
        std::fs::write(&file, [0x89u8, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]).unwrap();
        let html =
            "<!doctype html><html><body><img alt=\"mark\" src=\"{{img:mark}}\"></body></html>";
        let arguments = serde_json::json!({
            "project": slug, "name": "hero", "html": html,
            "assets": [{"name": "mark", "path": file.to_str().unwrap()}],
        });
        let (_, id, _) = frame_from_tool(&home, &arguments).unwrap();
        let frame = load(&home, &slug)
            .unwrap()
            .frames
            .into_iter()
            .find(|frame| frame.id == id)
            .unwrap();
        assert!(
            frame.html.contains("data:image/png;base64,"),
            "{}",
            frame.html
        );
        assert!(!frame.html.contains("{{img:"), "{}", frame.html);
        assert_eq!(assets(&home, &slug).len(), 1, "the file was stored");
        let arguments = serde_json::json!({
            "project": slug, "name": "x",
            "html": "<!doctype html><html><body><img src=\"{{img:ghost}}\"></body></html>",
        });
        let error = frame_from_tool(&home, &arguments).unwrap_err();
        assert!(error.contains("ghost"), "{error}");
    }

    /// A brand is the board's design system: its tokens default the
    /// colours of every page landed there, the choice is written on the
    /// board, and a choice naming no brand is refused by name.
    #[test]
    fn a_brand_seeds_the_pages_colors_and_the_board_keeps_the_choice() {
        let home = scratch("brand");
        let (slug, _) = create(&home, "Branded").unwrap();
        let mut tokens = BTreeMap::new();
        tokens.insert("accent".to_string(), "#b4552d".to_string());
        tokens.insert("scale".to_string(), "1.3".to_string());
        let brand = put_brand(&home, "Bellows", tokens, "Warm, quiet, editorial.").unwrap();
        assert_eq!(brand.slug, "bellows");
        assert_eq!(brand.tokens["accent"], "#b4552d");
        set_brand(&home, &slug, Some("bellows")).unwrap();
        assert_eq!(
            load(&home, &slug).unwrap().brand.as_deref(),
            Some("bellows")
        );
        // The brand is not a board: the canvas lists only projects.
        let listed: Vec<String> = projects(&home).into_iter().map(|one| one.slug).collect();
        assert_eq!(listed, vec![slug.clone()], "{listed:?}");

        let spec = serde_json::json!([
            {"key":"accent","label":"accent","kind":"color","default":"#111","css_var":"accent"},
            {"key":"scale","label":"scale","kind":"slider","min":0.8,"max":1.4,"step":0.05,
             "default":1,"css_var":"scale"}
        ]);
        let arguments = serde_json::json!({
            "project": slug, "name": "hero", "html": PAGE, "tweak_spec": spec,
        });
        let (_, id, _) = frame_from_tool(&home, &arguments).unwrap();
        let frame = load(&home, &slug)
            .unwrap()
            .frames
            .into_iter()
            .find(|frame| frame.id == id)
            .unwrap();
        // A color token becomes the page's colour; a slider keeps its own
        // numeric default, because a token is a declaration, not a number.
        assert_eq!(
            frame.tweak_values.get("accent"),
            Some(&Value::String("#b4552d".to_string()))
        );
        assert!(
            !frame.tweak_values.contains_key("scale"),
            "{:?}",
            frame.tweak_values
        );

        // A choice naming no brand is refused by name.
        let error = set_brand(&home, &slug, Some("nope")).unwrap_err();
        assert!(error.contains("nope"), "{error}");
        // A token value cannot smuggle CSS: `;` and `}` are refused.
        let mut hostile = BTreeMap::new();
        hostile.insert("accent".to_string(), "red;}</style>".to_string());
        let error = put_brand(&home, "Hostile", hostile, "").unwrap_err();
        assert!(error.contains("accent"), "{error}");
        // Dropping the brand leaves the choice recorded but unresolvable,
        // which the read names rather than the board silently forgetting.
        delete_brand(&home, "bellows").unwrap();
        assert!(brands(&home).is_empty());
        assert_eq!(
            load(&home, &slug).unwrap().brand.as_deref(),
            Some("bellows")
        );
    }

    /// A preset is two prompts: its spec, and the avoid list the library
    /// wrote as the direction's negative.
    #[test]
    fn a_preset_carries_its_spec_and_its_avoid_list() {
        let facts = DesignFacts {
            preset: Some(Preset {
                label: "Agency grid".to_string(),
                spec: "LAYOUT: a disciplined grid".to_string(),
                avoid: "rounded corners, heavy gradients, playful type".to_string(),
            }),
            ..Default::default()
        };
        let task = designer_task(DesignMode::Generate, "a pricing page", "board", &facts);
        assert!(
            task.contains("Style: Agency grid \u{2014} LAYOUT: a disciplined grid"),
            "{task}"
        );
        assert!(
            task.contains("Avoid: rounded corners, heavy gradients, playful type"),
            "{task}"
        );
        // A preset with no avoid list says nothing rather than "Avoid: ".
        let bare = DesignFacts {
            preset: Some(Preset {
                label: "Bare".to_string(),
                spec: "LAYOUT: none".to_string(),
                avoid: String::new(),
            }),
            ..Default::default()
        };
        let task = designer_task(DesignMode::Generate, "a page", "board", &bare);
        assert!(!task.contains("Avoid:"), "{task}");
    }

    /// The task builders: every surface asks in the same words — the
    /// board and the frame are named, a review's fixes ride, the size and
    /// the directions compose, and a brand's tokens are binding.
    #[test]
    fn a_task_names_the_board_the_frame_and_what_was_asked() {
        let home = scratch("task");
        let (slug, _) = create(&home, "Task").unwrap();
        let id = put_frame(&home, &slug, "hero", PAGE, None, None, "m").unwrap();
        let project = load(&home, &slug).unwrap();
        let frame = project.frames[0].clone();
        // Generate: the board, and the frame tool.
        let task = designer_task(
            DesignMode::Generate,
            "a pricing page for a developer tool",
            &slug,
            &DesignFacts {
                size: Some(frame_size_for("a pricing page")),
                directions: 3,
                ..Default::default()
            },
        );
        assert!(task.contains(&slug), "{task}");
        assert!(task.contains("`frame` tool"), "{task}");
        assert!(task.contains("Give 3 directions"), "{task}");
        assert!(task.contains("desktop (1280"), "{task}");
        // A mobile brief sizes the frame.
        let (size, w, h) = frame_size_for("a mobile app screen");
        assert_eq!((size, w, h), ("mobile", 390.0, 844.0));
        let (size, _, _) = frame_size_for("a tablet dashboard");
        assert_eq!(size, "tablet");
        // Refine: the frame id, so identity holds.
        let task = designer_task(
            DesignMode::Refine,
            "make the hero quieter",
            &slug,
            &DesignFacts {
                frame: Some(frame.clone()),
                ..Default::default()
            },
        );
        assert!(task.contains(&format!("frame_id `{id}`")), "{task}");
        assert!(task.contains("keeps its identity"), "{task}");
        // Fixes: the review's lines ride, and the pins the frame carries.
        let review = serde_json::json!({
            "kind": "vision",
            "fixes": ["tighten the masthead", "raise the contrast of the subhead"],
            "scores": { "hierarchy": 6.0 }
        });
        let task = designer_task(
            DesignMode::Fixes,
            "",
            &slug,
            &DesignFacts {
                frame: Some(frame.clone()),
                review: Some(review),
                pins: vec!["the spacing is off".to_string()],
                ..Default::default()
            },
        );
        assert!(task.contains("tighten the masthead"), "{task}");
        assert!(task.contains("raise the contrast of the subhead"), "{task}");
        assert!(task.contains("the spacing is off"), "{task}");
        assert!(task.contains(&format!("frame_id `{id}`")), "{task}");
        // A brand is binding: its tokens reach the task.
        let mut tokens = BTreeMap::new();
        tokens.insert("accent".to_string(), "#1f6feb".to_string());
        let task = designer_task(
            DesignMode::Generate,
            "a landing page",
            &slug,
            &DesignFacts {
                brand: Some((
                    "Acme".to_string(),
                    tokens,
                    "Quiet, typographic.".to_string(),
                )),
                ..Default::default()
            },
        );
        assert!(task.contains("--accent: #1f6feb;"), "{task}");
        assert!(task.contains("Quiet, typographic."), "{task}");
        // Ask writes nothing and names no tool — but it carries the
        // board's own inventory, so a question about the board is answered
        // from the facts rather than by exploring the console.
        let task = designer_task(
            DesignMode::Ask,
            "how many pages?",
            &slug,
            &DesignFacts {
                board: vec!["hero (id f-1, 1280 by 900, m)".to_string()],
                ..Default::default()
            },
        );
        assert!(task.contains("About the design board"), "{task}");
        assert!(task.contains("holds 1 frame"), "{task}");
        assert!(task.contains("hero (id f-1"), "{task}");
        assert!(!task.contains("`frame` tool"), "{task}");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The board's chat is the document's: a turn round-trips, the frames
    /// ride it, and the list is bounded.
    #[test]
    fn a_chat_turn_round_trips_and_the_cap_holds() {
        let home = scratch("chat");
        let (slug, _) = create(&home, "Chat").unwrap();
        let id = put_frame(&home, &slug, "hero", PAGE, None, None, "m").unwrap();
        append_chat(&home, &slug, "owner", "a pricing page", &[]).unwrap();
        append_chat(
            &home,
            &slug,
            "designer",
            "here it is",
            std::slice::from_ref(&id),
        )
        .unwrap();
        let project = load(&home, &slug).unwrap();
        assert_eq!(project.chat.len(), 2);
        assert_eq!(project.chat[0].role, "owner");
        assert_eq!(project.chat[1].frames, vec![id]);
        for index in 0..(MAX_CHAT_TURNS + 5) {
            append_chat(&home, &slug, "owner", &format!("turn {index}"), &[]).unwrap();
        }
        let project = load(&home, &slug).unwrap();
        assert_eq!(project.chat.len(), MAX_CHAT_TURNS);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The bundled sample board IS a document this reader accepts — the
    /// surface's first-run board, pinned as a parse. (It carries the
    /// source's camelCase spellings: the aliases are what make it land.)
    #[test]
    fn the_bundled_sample_board_parses() {
        let text = include_str!("gui/assets/design-sample.json");
        let project: Project = serde_json::from_str(text)
            .unwrap_or_else(|error| panic!("the bundled sample must parse: {error}"));
        assert_eq!(project.frames.len(), 2, "the two fixture frames");
        let with_tweaks = project
            .frames
            .iter()
            .filter(|frame| !frame.tweak_spec.is_empty())
            .count();
        assert!(with_tweaks >= 1, "a fixture frame exposes tweaks");
        // And the adoption writes it on a home with no canvas.
        let home = scratch("sample");
        ensure_sample(&home).unwrap();
        let projects = projects(&home);
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].frames, 2);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// D2: the document is the home's, and a frame written through the
    /// store round-trips: geometry, spec, pins.
    #[test]
    fn a_frame_round_trips_through_the_board_document() {
        let home = scratch("round-trip");
        let (slug, _) = create(&home, "Test board").unwrap();
        let spec = vec![TweakSpec {
            key: "accent".to_string(),
            label: "accent".to_string(),
            kind: "color".to_string(),
            min: None,
            max: None,
            step: None,
            unit: None,
            default: Value::String("#3b6cf6".to_string()),
            options: None,
            css_var: "accent".to_string(),
        }];
        let id = put_frame(&home, &slug, "hero", PAGE, Some(&spec), None, "glm").unwrap();
        let pin = add_pin(&home, &slug, &id, 0.5, 0.25, "the spacing is off").unwrap();
        reply_pin(&home, &slug, &id, &pin, "on it").unwrap();
        let project = load(&home, &slug).unwrap();
        assert_eq!(project.frames.len(), 1);
        let frame = &project.frames[0];
        assert_eq!(frame.name, "hero");
        assert_eq!(frame.x, 0.0);
        assert_eq!(frame.w, 1280.0);
        assert_eq!(frame.tweak_spec.len(), 1);
        assert_eq!(frame.pins.len(), 1);
        assert_eq!(frame.pins[0].x, 0.5);
        assert_eq!(frame.pins[0].replies.len(), 1);
        // The generate call snapshotted the page as the first version.
        assert_eq!(frame.versions.len(), 1);
        assert_eq!(frame.versions[0].source, "generate");
        // A refine snapshots again and takes the new page.
        let refined = put_frame(
            &home,
            &slug,
            "hero",
            &PAGE.replace("hi", "hi again"),
            None,
            Some(&id),
            "glm",
        )
        .unwrap();
        assert_eq!(refined, id);
        let project = load(&home, &slug).unwrap();
        assert_eq!(project.frames[0].versions.len(), 2);
        assert!(project.frames[0].html.contains("hi again"));
        let _ = std::fs::remove_dir_all(&home);
    }

    /// D2: the caps refuse by name — a frame past the byte cap, a frame
    /// that is not a document.
    #[test]
    fn the_frame_caps_refuse_by_name() {
        let home = scratch("caps");
        let (slug, _) = create(&home, "Caps").unwrap();
        let huge = format!(
            "<!doctype html><html><body>{}</body></html>",
            "x".repeat(MAX_FRAME_BYTES)
        );
        let error = put_frame(&home, &slug, "huge", &huge, None, None, "m").unwrap_err();
        assert!(error.contains("the cap is"), "{error}");
        let error =
            put_frame(&home, &slug, "fragment", "<h1>hi</h1>", None, None, "m").unwrap_err();
        assert!(error.contains("not a complete document"), "{error}");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// D5: injection is the one mutation — a replace, an insert, escaped
    /// values, and the token extraction the exporters read.
    #[test]
    fn inject_tweaks_is_the_one_mutation_path() {
        let spec = |key: &str, css: &str| TweakSpec {
            key: key.to_string(),
            label: key.to_string(),
            kind: if css == "accent" { "color" } else { "slider" }.to_string(),
            min: None,
            max: None,
            step: None,
            unit: Some("rem".to_string()),
            default: Value::from(1.0),
            options: None,
            css_var: css.to_string(),
        };
        let specs = vec![spec("accent", "accent"), spec("scale", "scale")];
        let mut values = BTreeMap::new();
        values.insert("accent".to_string(), Value::String("#ff0000".to_string()));
        values.insert("scale".to_string(), Value::from(1.25));
        let injected = inject_tweaks(PAGE, &specs, &values);
        assert!(injected.contains("--accent: #ff0000;"), "{injected}");
        assert!(injected.contains("--scale: 1.25rem;"), "{injected}");
        // A second pass replaces, never stacks.
        let twice = inject_tweaks(&injected, &specs, &values);
        assert_eq!(twice.matches("fleet-tweaks").count(), 1, "{twice}");
        // A hostile value never reaches the sheet.
        let mut hostile = BTreeMap::new();
        hostile.insert(
            "accent".to_string(),
            Value::String("red;}body{display:none".to_string()),
        );
        let guarded = inject_tweaks(PAGE, &specs, &hostile);
        assert!(!guarded.contains("display:none"), "{guarded}");
        // The tokens the page declares are the exporters' set.
        let tokens = extract_root_tokens(PAGE);
        let names: Vec<&str> = tokens.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["page", "ink", "accent"]);
        let css = tokens_css(PAGE);
        assert!(css.contains("--ink: #e6e9ef;"), "{css}");
        assert!(tokens_json(PAGE, "hero")["hero"]["ink"]["$value"] == "#e6e9ef");
    }

    /// D9: the scan is a scanner — the page's own token contrast, alt,
    /// labels, focus and viewport, each with its fact named.
    #[test]
    fn the_scan_measures_the_pages_own_facts() {
        let scan = a11y_scan(PAGE);
        assert!(scan.total >= 5, "{scan:?}");
        assert_eq!(scan.passed, scan.total, "{scan:?}");
        let contrast: Vec<&A11yCheck> = scan
            .checks
            .iter()
            .filter(|check| check.kind == "contrast")
            .collect();
        assert!(!contrast.is_empty());
        assert!(
            contrast
                .iter()
                .any(|check| check.detail.contains("ink on page")),
            "{scan:?}"
        );
        // A page that fails the checks says so.
        let bad = "<!doctype html><html><head><style>:root{--page:#ffffff;--ink:#f5f5f5}</style></head><body><img src=\"x.png\"><input></body></html>";
        let scan = a11y_scan(bad);
        assert!(scan.passed < scan.total, "{scan:?}");
        assert!(scan
            .checks
            .iter()
            .any(|check| !check.pass && check.kind == "alt"));
        assert!(scan
            .checks
            .iter()
            .any(|check| !check.pass && check.kind == "viewport"));
    }

    /// W5: the frame tool sizes the page it lands — a phone brief's frame
    /// is a phone canvas, and the default stays the board's desktop.
    #[test]
    fn the_frame_tool_sizes_the_page_it_lands() {
        let home = scratch("sizing");
        let (slug, _) = create(&home, "Sizing").unwrap();
        let (_, mobile, _) = frame_from_tool(
            &home,
            &serde_json::json!({ "project": slug, "name": "phone", "html": PAGE,
                "viewport": "mobile" }),
        )
        .unwrap();
        let (_, plain, _) = frame_from_tool(
            &home,
            &serde_json::json!({ "project": slug, "name": "desk", "html": PAGE }),
        )
        .unwrap();
        let project = load(&home, &slug).unwrap();
        let by_id = |id: &str| {
            project
                .frames
                .iter()
                .find(|frame| frame.id == id)
                .unwrap()
                .clone()
        };
        assert_eq!((by_id(&mobile).w, by_id(&mobile).h), (390.0, 844.0));
        assert_eq!((by_id(&plain).w, by_id(&plain).h), (1280.0, 900.0));
        assert_eq!(sizing_for("tablet"), ("tablet", 768.0, 1024.0));
        assert_eq!(sizing_for("nonsense"), ("desktop", 1280.0, 900.0));
        // The brief's own rule picks the word the task asks for.
        assert_eq!(frame_size_for("a mobile checkout").0, "mobile");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The tools' handlers: the frame tool refuses a board it cannot
    /// find, and a review lands on the frame.
    #[test]
    fn the_tool_handlers_refuse_and_store() {
        let home = scratch("tools");
        let error = frame_from_tool(
            &home,
            &serde_json::json!({ "project": "nope", "name": "x", "html": PAGE }),
        )
        .unwrap_err();
        assert!(error.contains("no canvas board"), "{error}");
        let (slug, _) = create(&home, "Tools").unwrap();
        let (got, frame_id, added) = frame_from_tool(
            &home,
            &serde_json::json!({ "project": slug, "name": "hero", "html": PAGE }),
        )
        .unwrap();
        assert_eq!(got, slug);
        assert!(added);
        let (_, reviewed) = review_from_tool(
            &home,
            &serde_json::json!({
                "project": slug,
                "frame_id": frame_id,
                "scores": { "hierarchy": 8.5 },
                "summary": "reads well",
                "fixes": ["tighten the masthead"],
            }),
        )
        .unwrap();
        assert_eq!(reviewed, frame_id);
        let project = load(&home, &slug).unwrap();
        let review = project.frames[0].review.clone().unwrap();
        assert_eq!(review["kind"], "vision");
        assert_eq!(review["scores"]["hierarchy"], 8.5);
        // A second review keeps the first in its bound history.
        review_from_tool(
            &home,
            &serde_json::json!({ "project": slug, "frame_id": frame_id, "summary": "again" }),
        )
        .unwrap();
        let project = load(&home, &slug).unwrap();
        let history = project.frames[0].review.as_ref().unwrap()["history"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!(history, 1, "one review in history");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// D9: a restore snapshots the current page first — restoring is
    /// itself a version, so nothing is ever lost by one.
    #[test]
    fn a_restore_is_itself_a_version() {
        let home = scratch("restore");
        let (slug, _) = create(&home, "Restore").unwrap();
        let id = put_frame(&home, &slug, "one", PAGE, None, None, "m").unwrap();
        // Refine to a second page: version 2 snapshots "hi".
        put_frame(
            &home,
            &slug,
            "one",
            &PAGE.replace("hi", "later"),
            None,
            Some(&id),
            "m",
        )
        .unwrap();
        let project = load(&home, &slug).unwrap();
        let first = project.frames[0].versions[0].id.clone();
        // Restore the original: the current page is kept as a version.
        restore_version(&home, &slug, &id, &first).unwrap();
        let project = load(&home, &slug).unwrap();
        assert_eq!(
            project.frames[0].versions.len(),
            3,
            "refine + restore snapshot"
        );
        assert!(
            project.frames[0].html.contains("<h1>hi</h1>"),
            "the original page is back"
        );
        assert_eq!(project.frames[0].versions.last().unwrap().source, "restore");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The deletion the source never persisted: a delete leaves the
    /// document, and a duplicate lands to the right of its source.
    #[test]
    fn delete_persists_and_duplicate_offsets() {
        let home = scratch("delete");
        let (slug, _) = create(&home, "Delete").unwrap();
        let id = put_frame(&home, &slug, "one", PAGE, None, None, "m").unwrap();
        let copy = duplicate_frame(&home, &slug, &id).unwrap();
        let project = load(&home, &slug).unwrap();
        assert_eq!(project.frames.len(), 2);
        let source = project.frames.iter().find(|frame| frame.id == id).unwrap();
        let duplicated = project
            .frames
            .iter()
            .find(|frame| frame.id == copy)
            .unwrap();
        assert_eq!(duplicated.x, source.x + source.w + 96.0);
        delete_frame(&home, &slug, &id).unwrap();
        let project = load(&home, &slug).unwrap();
        assert_eq!(project.frames.len(), 1);
        assert_eq!(project.frames[0].id, copy);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// K1 + K3 (the designs-are-project-knowledge plan): the binding is a
    /// field that survives the store round-trip, and every save of a
    /// bound board regenerates its digest at the project-keyed path —
    /// while an unbound board writes none, and a clear removes the file
    /// the old binding had left.
    #[test]
    fn a_bound_board_writes_its_digest_and_a_clear_removes_it() {
        let home = scratch("digest");
        let (slug, _) = create(&home, "Digest").unwrap();
        // Unbound: the save writes no digest anywhere.
        save(&home, &slug, &load(&home, &slug).unwrap()).unwrap();
        assert!(
            !digest_path(&home, "fleet", &slug).exists(),
            "an unbound board must not write a digest"
        );
        // Bound: the digest lands at the project-keyed path.
        let mut project = load(&home, &slug).unwrap();
        project.project = Some("/Users/owner/Projects/fleet".to_string());
        put_frame(&home, &slug, "hero", PAGE, None, None, "deepseek-flash").unwrap();
        let mut project = load(&home, &slug).unwrap();
        project.project = Some("/Users/owner/Projects/fleet".to_string());
        save(&home, &slug, &project).unwrap();
        let path = digest_path(&home, "fleet", &slug);
        assert!(path.is_file(), "a bound board's save writes its digest");
        let digest = std::fs::read_to_string(&path).unwrap();
        assert!(digest.contains("project: /Users/owner/Projects/fleet"));
        assert!(digest.contains("- hero \u{b7} 1280x900 \u{b7} model deepseek-flash"));
        assert!(
            digest.contains("accent, ink, page"),
            "the token union rides, sorted, bare names"
        );
        // The binding round-trips through the document itself.
        assert_eq!(
            load(&home, &slug).unwrap().project.as_deref(),
            Some("/Users/owner/Projects/fleet")
        );
        // A clear removes the digest the old binding left behind — the
        // op that clears knows the old key; the store never guesses.
        remove_digest(&home, "fleet", &slug).unwrap();
        assert!(!path.exists(), "clearing the binding removes the digest");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The digest is deterministic and carries the whole account: the
    /// name, the pages with their sizes and models, the sorted token
    /// union, and a page's review scores when it carries them.
    #[test]
    fn the_digest_names_every_page_tokens_and_review() {
        let now = "2026-11-01T00:00:00Z".to_string();
        let mut frame = Frame {
            id: "f-1".to_string(),
            name: "Landing".to_string(),
            description: String::new(),
            html: PAGE.to_string(),
            x: 0.0,
            y: 0.0,
            w: 1280.0,
            h: 900.0,
            tweak_spec: Vec::new(),
            tweak_values: BTreeMap::new(),
            model: "deepseek-flash".to_string(),
            set: Some("site".to_string()),
            gen_meta: None,
            versions: Vec::new(),
            pins: Vec::new(),
            review: Some(serde_json::json!({"scores": {"spacing": 7, "hierarchy": 9}})),
            created: now.clone(),
            updated: now.clone(),
        };
        frame.description = String::new();
        let project = Project {
            version: 1,
            name: "Site".to_string(),
            created: now.clone(),
            updated: now,
            frames: vec![frame],
            chat: Vec::new(),
            brand: Some("acme".to_string()),
            reference: None,
            project: Some("/x/projects/acme".to_string()),
        };
        let digest = digest_markdown("site", &project);
        assert_eq!(
            digest,
            "# Site\n\nboard: site\nproject: /x/projects/acme\nupdated: \
             2026-11-01T00:00:00Z\nbrand: acme\npages: 1\n\n## Pages\n\n- \
             Landing \u{b7} 1280x900 \u{b7} model deepseek-flash \u{b7} set \
             site\n\n## Tokens\n\naccent, ink, page\n\n## Reviews\n\n- \
             Landing: hierarchy 9, spacing 7\n",
            "the digest is byte-deterministic"
        );
        let _ = std::fs::remove_dir_all(scratch("digest-exact"));
    }

    /// The key is the bound path's final segment — the name the refresh
    /// jobs and the vector stores already use — and a root path reads
    /// as unkeyable rather than inventing a key.
    #[test]
    fn the_project_key_is_the_paths_final_segment() {
        assert_eq!(
            project_key("/Users/owner/Projects/fleet"),
            Some("fleet".to_string())
        );
        assert_eq!(
            project_key("/Users/owner/Projects/fleet/"),
            Some("fleet".to_string())
        );
        assert_eq!(project_key("/"), None);
        assert_eq!(project_key(""), None);
    }

    /// P2 (design-parity C5): a board deletes whole — its directory
    /// and its digest — and a missing board is a named error, not a
    /// silent no-op.
    #[test]
    fn deleting_a_board_removes_its_store_and_its_digest() {
        let home = scratch("delete-project");
        let (slug, _) = create(&home, "Doomed").unwrap();
        put_frame(&home, &slug, "p", PAGE, None, None, "m").unwrap();
        let mut project = load(&home, &slug).unwrap();
        project.project = Some("/x/Projects/acme".to_string());
        save(&home, &slug, &project).unwrap();
        let digest = digest_path(&home, "acme", &slug);
        assert!(digest.is_file());
        delete_project(&home, &slug).unwrap();
        assert!(!exists(&home, &slug), "the board's directory is gone");
        assert!(!digest.exists(), "the board's digest is gone");
        let refused = delete_project(&home, &slug).expect_err("a missing board refuses");
        assert!(refused.contains(&slug), "{refused}");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// K6 (designs-are-project-knowledge): the context section reads a
    /// project's digests — both directions: a project with boards
    /// yields the bounded account (newest first, one line per board),
    /// and a project with none yields nothing rather than an empty
    /// section.
    #[test]
    fn the_design_context_reads_a_projects_digests_or_none() {
        let home = scratch("context");
        // A project with two bound boards: two digests under its key.
        for name in ["Harbor", "Kiosk"] {
            let (slug, _) = create(&home, name).unwrap();
            let mut project = load(&home, &slug).unwrap();
            project.project = Some("/x/Projects/acme".to_string());
            put_frame(&home, &slug, "p", PAGE, None, None, "m").unwrap();
            let mut project = load(&home, &slug).unwrap();
            project.project = Some("/x/Projects/acme".to_string());
            save(&home, &slug, &project).unwrap();
        }
        let section = design_context_section(&home, "acme").expect("the section");
        assert!(
            section.contains("Design boards bound to `acme`"),
            "{section}"
        );
        assert_eq!(section.matches("- ").count(), 2, "{section}");
        assert!(section.contains("Harbor"), "{section}");
        assert!(section.contains("Kiosk"), "{section}");
        assert!(section.contains("1 page"), "{section}");
        // A project with no digests: none, not an empty section.
        assert_eq!(design_context_section(&home, "nowhere"), None);
        let _ = std::fs::remove_dir_all(&home);
    }
}
