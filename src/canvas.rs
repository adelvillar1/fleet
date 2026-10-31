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

/// One project: the board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub created: String,
    pub updated: String,
    pub frames: Vec<Frame>,
}

/// A project as the pickers list it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProjectSummary {
    pub slug: String,
    pub name: String,
    pub frames: usize,
    pub updated: String,
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
            let project = load(home, &slug).ok()?;
            Some(ProjectSummary {
                slug,
                name: project.name,
                frames: project.frames.len(),
                updated: project.updated,
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
/// so a reader never sees half a document.
pub fn save(home: &Path, slug: &str, project: &Project) -> Result<(), String> {
    let path = project_path(home, slug);
    let dir = path
        .parent()
        .ok_or_else(|| "the project path has no directory".to_string())?;
    std::fs::create_dir_all(dir)
        .map_err(|error| format!("could not make {}: {error}", dir.display()))?;
    let text = serde_json::to_string_pretty(project)
        .map_err(|error| format!("the project does not serialize: {error}"))?;
    crate::settings::write_atomic(&path, &text)
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
    };
    save(home, &slug, &project)?;
    Ok((slug, project))
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
        w: VIEWPORT_PRESETS[0].1,
        h: VIEWPORT_PRESETS[0].2,
        tweak_spec: tweak_spec.map(<[TweakSpec]>::to_vec).unwrap_or_default(),
        tweak_values: BTreeMap::new(),
        model: model.to_string(),
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
    let added = frame_id.is_none();
    let id = put_frame(
        home,
        &slug,
        name,
        html,
        spec.as_deref(),
        frame_id,
        "designer",
    )?;
    Ok((slug, id, added))
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
}
