//! Roles: the named specializations a delegation can run under.
//!
//! A role is a file under `<home>/roles/<name>.md` — the home's own
//! conventions, never a foreign layout — carrying the specialization a
//! delegated child runs under: its description, the system-prompt
//! overlay that rides beside the compiled-in one, the tool set it
//! admits, the skills it loads, and the memory shapes its recall draws
//! from. A role with no overlay is still a role: the tool scope and
//! the skill set are the specialization, and the prompt stays the
//! parent's.
//!
//! The loader is deliberately strict about names and deliberately
//! forgiving about files: a name that could name a path is refused
//! (a role's name is a name, not a path), and a broken file is named
//! and skipped rather than sinking the surface that lists the others.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// `<home>/roles` — the directory a role file lives in.
pub const ROLES_DIR: &str = "roles";

/// How much of a role's overlay rides the child's system prompt. A
/// role is standing instruction, not a document; the bound is named
/// rather than discovered.
const OVERLAY_CAP: usize = 4_000;

/// One role, as its file declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// The role's name — the file's stem, and the value the
    /// `subagent` tool's `role` argument names.
    pub name: String,
    /// What the role is for, one line: what the surface's card shows.
    pub description: String,
    /// The tools this role admits, by name. Empty means the brief's
    /// own declaration stands — a role that narrows nothing admits
    /// what the delegation asked for.
    pub tools: Vec<String>,
    /// The skills this role loads beside the parent's, by name.
    pub skills: Vec<String>,
    /// The memory shapes this role's recall draws from. Empty means
    /// every shape, which is the parent's own recall.
    pub shapes: Vec<String>,
    /// The system-prompt overlay: everything after the frontmatter
    /// fence, bounded where it rides.
    pub overlay: String,
    /// The file the role was read from.
    pub path: PathBuf,
    /// The file's mtime, for the surface's card. `None` when the
    /// metadata could not be read.
    pub modified: Option<SystemTime>,
}

impl Role {
    /// Read one role out of its file's text. The frontmatter's `name`
    /// and `description` are required — a role neither named nor
    /// described is a file, not a role — and `tools`, `skills`, and
    /// `shapes` are comma-separated lists, absent meaning "narrows
    /// nothing".
    fn parse(name: &str, text: &str, path: PathBuf) -> Result<Role, String> {
        let lines: Vec<&str> = text.lines().collect();
        if lines.first().map(|line| line.trim_end()) != Some("---") {
            return Err("the role file does not open with a `---` frontmatter fence".to_string());
        }
        let mut fields: Vec<(String, String)> = Vec::new();
        let mut closing: Option<usize> = None;
        for (index, line) in lines.iter().enumerate().skip(1) {
            if line.trim_end() == "---" {
                closing = Some(index);
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.starts_with(' ') || line.starts_with('\t') || trimmed.starts_with('-') {
                return Err(format!(
                    "frontmatter line {} is not a `key: value` pair (nested YAML is not supported)",
                    index + 1
                ));
            }
            let Some((key, value)) = trimmed.split_once(':') else {
                return Err(format!(
                    "frontmatter line {} is not a `key: value` pair",
                    index + 1
                ));
            };
            let key = unquote(key.trim()).to_string();
            if key.is_empty() {
                return Err(format!("frontmatter line {} has an empty key", index + 1));
            }
            let value = unquote(value.trim()).to_string();
            if value.is_empty() {
                return Err(format!("frontmatter key `{key}` has an empty value"));
            }
            if fields.iter().any(|(seen, _)| *seen == key) {
                return Err(format!("frontmatter key `{key}` appears twice"));
            }
            fields.push((key, value));
        }
        let Some(closing) = closing else {
            return Err("the frontmatter fence never closes".to_string());
        };
        let field = |key: &str| {
            fields
                .iter()
                .find(|(seen, _)| seen == key)
                .map(|(_, value)| value.clone())
        };
        let description = field("description").ok_or_else(|| {
            "the role file names no `description` — a role neither named nor described is a file, \
             not a role"
                .to_string()
        })?;
        let modified = std::fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .ok();
        Ok(Role {
            name: name.to_string(),
            description,
            tools: list_field(&field("tools")),
            skills: list_field(&field("skills")),
            shapes: list_field(&field("shapes")),
            overlay: lines[closing + 1..].join("\n").trim().to_string(),
            path,
            modified,
        })
    }

    /// Whether this role admits `tool`. A role that names no tools
    /// narrows nothing: the brief's own declaration stands. The
    /// delegation tool itself is never admissible inside a role's
    /// scope — the depth cap's own rule, which a role cannot widen.
    pub fn admits(&self, tool: &str) -> bool {
        if tool == "subagent" {
            return false;
        }
        self.tools.is_empty() || self.tools.iter().any(|admitted| admitted == tool)
    }
}

/// `<home>/roles`.
pub fn roles_dir(home: &Path) -> PathBuf {
    home.join(ROLES_DIR)
}

/// Load one role by name. A name that could name a path is refused —
/// the role's name is a name, not a path — and a name whose file does
/// not exist is a named refusal that says where the roles live.
pub fn load(home: &Path, name: &str) -> Result<Role, String> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name == "."
        || name.starts_with('.')
    {
        return Err(format!(
            "`{name}` is not a role name — a role's name is a name, not a path"
        ));
    }
    let path = roles_dir(home).join(format!("{name}.md"));
    let text = std::fs::read_to_string(&path).map_err(|_| {
        format!(
            "no role named `{name}` under {} — `fleet roles` lists them",
            roles_dir(home).display()
        )
    })?;
    Role::parse(name, &text, path)
}

/// Every role in the home, sorted by name. A broken file is named on
/// one line and skipped — a broken role is never silent and never
/// sinks the surface that lists the others.
pub fn load_all(home: &Path) -> Vec<Role> {
    let dir = roles_dir(home);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut roles: Vec<Role> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        // Owned, so the error arm can name the file after the path
        // moves into the parsed role.
        let name = name.to_string();
        // Read first, so the borrow ends before the path moves into
        // the parsed role.
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("fleet: roles: skipping `{name}` — {error}");
                continue;
            }
        };
        match Role::parse(&name, &text, path) {
            Ok(role) => roles.push(role),
            Err(reason) => eprintln!("fleet: roles: skipping `{name}` — {reason}"),
        }
    }
    roles.sort_by(|left, right| left.name.cmp(&right.name));
    roles
}

/// One role as the editor reads it: the parsed role, or the named reason
/// the file is not one. A broken file is named rather than skipped
/// silently on the settings surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleEntry {
    pub name: String,
    pub role: Option<Role>,
    pub broken: Option<String>,
}

/// Every role file in the home, in name order, with broken files named.
pub fn catalog(home: &Path) -> Vec<RoleEntry> {
    let dir = roles_dir(home);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<RoleEntry> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let name = name.to_string();
        let entry = match std::fs::read_to_string(&path) {
            Ok(text) => match Role::parse(&name, &text, path) {
                Ok(role) => RoleEntry {
                    name,
                    role: Some(role),
                    broken: None,
                },
                Err(reason) => RoleEntry {
                    name,
                    role: None,
                    broken: Some(reason),
                },
            },
            Err(error) => RoleEntry {
                name: name.clone(),
                role: None,
                broken: Some(format!("{} is unreadable: {error}", path.display())),
            },
        };
        out.push(entry);
    }
    out.sort_by(|left, right| left.name.cmp(&right.name));
    out
}

/// The fields the editor owns. The writer preserves every frontmatter
/// key it does not own and replaces the overlay's body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleFields {
    pub description: String,
    pub tools: Vec<String>,
    pub skills: Vec<String>,
    pub shapes: Vec<String>,
    pub overlay: String,
}

/// Write one role file through the loader's own grammar. The name is
/// validated with the same rule `load` applies; the description is
/// required; unknown frontmatter keys survive the edit; the write is a
/// unique temp file and a rename.
pub fn write(home: &Path, name: &str, fields: &RoleFields) -> Result<(), String> {
    validate_name(name)?;
    let description = fields.description.trim();
    if description.is_empty() {
        return Err("a role needs a description".to_string());
    }
    let path = roles_dir(home).join(format!("{name}.md"));
    let existing = std::fs::read_to_string(&path).ok();
    let mut pairs: Vec<(String, String)> = Vec::new();
    if let Some(text) = &existing {
        let lines: Vec<&str> = text.lines().collect();
        if lines.first().map(|line| line.trim_end()) == Some("---") {
            for line in lines.iter().skip(1) {
                if line.trim_end() == "---" {
                    break;
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Some((key, value)) = trimmed.split_once(':') {
                    pairs.push((
                        unquote(key.trim()).to_string(),
                        unquote(value.trim()).to_string(),
                    ));
                }
            }
        }
    }
    let known = ["description", "tools", "skills", "shapes"];
    pairs.retain(|(key, _)| !known.contains(&key.as_str()));
    pairs.push(("description".to_string(), description.to_string()));
    for (key, values) in [
        ("tools", &fields.tools),
        ("skills", &fields.skills),
        ("shapes", &fields.shapes),
    ] {
        if !values.is_empty() {
            pairs.push((key.to_string(), values.join(", ")));
        }
    }
    pairs.sort_by(|left, right| left.0.cmp(&right.0));
    let mut text = String::from("---\n");
    for (key, value) in pairs {
        text.push_str(&format!("{key}: {value}\n"));
    }
    text.push_str("---\n");
    let overlay = fields.overlay.trim();
    if !overlay.is_empty() {
        text.push_str(overlay);
        text.push('\n');
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{} cannot be created: {error}", parent.display()))?;
    }
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temp, text)
        .map_err(|error| format!("{} cannot be written: {error}", temp.display()))?;
    std::fs::rename(&temp, &path)
        .map_err(|error| format!("{} cannot be replaced: {error}", path.display()))
}

/// Remove one role file. The name is validated with the loader's rule, so
/// a path-shaped name can never address a file outside the roles dir.
pub fn remove(home: &Path, name: &str) -> Result<(), String> {
    validate_name(name)?;
    let path = roles_dir(home).join(format!("{name}.md"));
    std::fs::remove_file(&path)
        .map_err(|error| format!("{} cannot be removed: {error}", path.display()))
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name == "."
        || name.starts_with('.')
    {
        return Err(format!(
            "`{name}` is not a role name \u{2014} a role's name is a name, not a path"
        ));
    }
    Ok(())
}

/// The frame a role's overlay rides in on the child's system prompt:
/// delimited so the child knows what is standing instruction for this
/// delegation's role and what is the compiled-in harness prompt, and
/// bounded like every other model-visible text.
pub fn overlay_prompt(role: &Role) -> String {
    if role.overlay.is_empty() {
        return String::new();
    }
    let bounded = bound(&role.overlay, OVERLAY_CAP);
    format!(
        "ROLE OVERLAY — standing instruction for this delegation's role `{name}`, beside the \
         compiled-in harness prompt above:\n----- ROLE OVERLAY BEGIN -----\n{bounded}\n\
         ----- ROLE OVERLAY END -----",
        name = role.name
    )
}

/// The cards `fleet roles` renders: one per role, with its
/// description, what it narrows, and its file. Never a plain-text
/// table.
pub fn render(roles: &[Role]) -> String {
    if roles.is_empty() {
        return frame(
            "roles",
            "the named specializations",
            vec![format!(
                "{DIM}no roles under the home yet — a role is <home>/roles/<name>.md{RESET}"
            )],
            "the `subagent` tool's `role` argument names one; an unknown name is a named refusal",
        );
    }
    let mut body = Vec::new();
    for role in roles {
        let narrows = if role.tools.is_empty() {
            "narrows nothing".to_string()
        } else {
            format!("tools: {}", role.tools.join(", "))
        };
        body.push(format!(
            "{BOLD}{ACCENT}{name}{RESET} {DIM}\u{b7}{RESET} {TEXT}{description}{RESET}",
            name = bound(&role.name, 24),
            description = bound(&role.description, 40)
        ));
        body.push(format!(
            "{DIM}\u{b7} {narrows} \u{b7} skills: {skills} \u{b7} shapes: {shapes}{RESET}",
            narrows = bound(&narrows, 40),
            skills = if role.skills.is_empty() {
                "the parent's".to_string()
            } else {
                role.skills.join(", ")
            },
            shapes = if role.shapes.is_empty() {
                "every shape".to_string()
            } else {
                role.shapes.join(", ")
            },
        ));
    }
    frame(
        "roles",
        &format!("{} role(s)", roles.len()),
        body,
        "the `subagent` tool's `role` argument names one; an unknown name is a named refusal",
    )
}

// --- the small shared helpers ----------------------------------------------

/// Strip one layer of matching surrounding quotes.
fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return &value[1..value.len() - 1];
        }
    }
    value
}

/// A comma-separated frontmatter list, trimmed, empties dropped. An
/// absent field is an empty list — narrows nothing.
fn list_field(value: &Option<String>) -> Vec<String> {
    value
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

/// Truncate `text` to at most `cap` characters with a marker, never
/// mid-word silently. The marker counts against `cap`.
fn bound(text: &str, cap: usize) -> String {
    const MARKER: &str = " \u{2026}[truncated]";
    if text.chars().count() <= cap {
        return text.to_string();
    }
    let keep = cap.saturating_sub(MARKER.chars().count());
    let kept: String = text.chars().take(keep).collect();
    format!("{kept}{MARKER}")
}

/// The card frame, the house family's shape: a titled box whose every
/// line is bounded by construction.
fn frame(title: &str, subtitle: &str, body: Vec<String>, footer: &str) -> String {
    const WIDTH: usize = 72;
    let right = bound(subtitle, WIDTH.saturating_sub(title.chars().count() + 8));
    let dashes = WIDTH.saturating_sub(title.chars().count() + right.chars().count() + 8);
    let rule = "\u{2500}".repeat(dashes);
    let mut out = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{title}{RESET} {DIM}{rule} {TEXT}{right}{RESET} {DIM}\u{2510}{RESET}\n"
    );
    for line in body {
        out.push_str(&format!("{DIM}\u{2502}{RESET} {line}\n"));
    }
    out.push_str(&format!(
        "{DIM}\u{2514}{}\u{2518}{RESET}\n",
        "\u{2500}".repeat(WIDTH - 4)
    ));
    out.push_str(&format!("{DIM}{footer}{RESET}\n"));
    out
}

const ACCENT: &str = "\u{1b}[36m";
const DIM: &str = "\u{1b}[90m";
const TEXT: &str = "\u{1b}[37m";
const BOLD: &str = "\u{1b}[1m";
const RESET: &str = "\u{1b}[0m";

#[cfg(test)]
mod tests {
    use super::*;

    fn home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-roles-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(ROLES_DIR)).unwrap();
        dir
    }

    fn write_role(home: &Path, name: &str, text: &str) {
        std::fs::write(home.join(ROLES_DIR).join(format!("{name}.md")), text).unwrap();
    }

    #[test]
    fn a_role_file_parses_its_specialization() {
        let home = home("parse");
        write_role(
            &home,
            "auditor",
            "---\nname: auditor\ndescription: reads a diff and names what it proves\ntools: bash, read_file\nskills: review-changes\nshapes: failure, resolution\n---\n\nYou are auditing a change. Read the diff before you judge it.\n",
        );
        let role = load(&home, "auditor").expect("the role loads");
        assert_eq!(role.name, "auditor");
        assert_eq!(role.description, "reads a diff and names what it proves");
        assert_eq!(role.tools, vec!["bash", "read_file"]);
        assert_eq!(role.skills, vec!["review-changes"]);
        assert_eq!(role.shapes, vec!["failure", "resolution"]);
        assert!(role.overlay.contains("Read the diff before you judge it."));
        // A role that names tools admits exactly those, and never the
        // delegation tool itself — the depth cap's own rule.
        assert!(role.admits("bash"));
        assert!(!role.admits("write_file"));
        assert!(!role.admits("subagent"));
        // A role that names no tools narrows nothing.
        write_role(
            &home,
            "plain",
            "---\ndescription: narrows nothing\n---\n\nStanding instruction.\n",
        );
        let plain = load(&home, "plain").expect("the plain role loads");
        assert!(plain.tools.is_empty());
        assert!(plain.admits("write_file"));
        assert!(!plain.admits("subagent"));
        // The overlay rides delimited and bounded; a role with no
        // overlay rides nothing.
        let prompt = overlay_prompt(&role);
        assert!(prompt.contains("ROLE OVERLAY BEGIN"), "{prompt}");
        assert!(prompt.contains("role `auditor`"), "{prompt}");
        assert!(overlay_prompt(&Role {
            overlay: String::new(),
            ..plain.clone()
        })
        .is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_unknown_or_unsafe_name_is_refused_by_name() {
        let home = home("refuse");
        // Absent.
        let error = load(&home, "no-such").unwrap_err();
        assert!(error.contains("no role named `no-such`"), "{error}");
        assert!(error.contains("fleet roles"), "{error}");
        // A name that could name a path.
        for bad in ["../etc/passwd", "a/b", "a\\b", "..", "."] {
            let error = load(&home, bad).unwrap_err();
            assert!(error.contains("not a role name"), "{error}");
        }
        // A broken file is named and skipped, and the others survive.
        write_role(&home, "broken", "no fence here\n");
        write_role(&home, "fine", "---\ndescription: fine\n---\n\nBody.\n");
        let all = load_all(&home);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "fine");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_cards_render_every_role_and_never_a_table() {
        let home = home("cards");
        write_role(
            &home,
            "auditor",
            "---\ndescription: reads a diff\ntools: bash,read_file\n---\n\nBody.\n",
        );
        write_role(
            &home,
            "scribe",
            "---\ndescription: writes the log\n---\n\nBody.\n",
        );
        let rendered = render(&load_all(&home));
        assert!(rendered.contains("auditor"), "{rendered}");
        assert!(rendered.contains("reads a diff"), "{rendered}");
        assert!(rendered.contains("tools: bash, read_file"), "{rendered}");
        assert!(rendered.contains("the parent's"), "{rendered}");
        assert!(!rendered.contains("| id |"), "{rendered}");
        // An empty home says so rather than drawing an empty table.
        let empty = render(&[]);
        assert!(empty.contains("no roles"), "{empty}");
        let _ = std::fs::remove_dir_all(&home);
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-roles-write-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The writer round-trips through the loader's own parser, and an
    /// unknown frontmatter key survives the edit.
    #[test]
    fn a_role_write_round_trips_and_preserves_unknown_keys() {
        let home = scratch("round-trip");
        std::fs::create_dir_all(roles_dir(&home)).unwrap();
        std::fs::write(
            roles_dir(&home).join("auditor.md"),
            "---\ndescription: old\nowner_note: keep me\n---\nold overlay\n",
        )
        .unwrap();
        write(
            &home,
            "auditor",
            &RoleFields {
                description: "reads a diff".to_string(),
                tools: vec!["bash".to_string(), "read_file".to_string()],
                skills: vec!["codebase-survey".to_string()],
                shapes: vec!["rendering".to_string()],
                overlay: "Prefer the smallest diff.".to_string(),
            },
        )
        .unwrap();
        let text = std::fs::read_to_string(roles_dir(&home).join("auditor.md")).unwrap();
        assert!(text.contains("owner_note: keep me"), "{text}");
        let role = load(&home, "auditor").unwrap();
        assert_eq!(role.description, "reads a diff");
        assert_eq!(role.tools, vec!["bash", "read_file"]);
        assert_eq!(role.skills, vec!["codebase-survey"]);
        assert_eq!(role.shapes, vec!["rendering"]);
        assert_eq!(role.overlay, "Prefer the smallest diff.");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A path-shaped name is refused by the writer with the loader's own
    /// rule, and a broken file is named by the catalog rather than
    /// skipped.
    #[test]
    fn the_writer_refuses_a_path_and_the_catalog_names_a_broken_file() {
        let home = scratch("refusals");
        let fields = RoleFields {
            description: "x".to_string(),
            ..RoleFields::default()
        };
        let error = write(&home, "../escape", &fields).expect_err("a path-like name");
        assert!(error.contains("not a path"), "{error}");
        let error = write(&home, "empty", &RoleFields::default()).expect_err("no description");
        assert!(error.contains("description"), "{error}");
        std::fs::create_dir_all(roles_dir(&home)).unwrap();
        std::fs::write(roles_dir(&home).join("broken.md"), "no fence\n").unwrap();
        let entries = catalog(&home);
        let broken = entries
            .iter()
            .find(|entry| entry.name == "broken")
            .expect("the broken file is named");
        assert!(broken.broken.is_some());
        assert!(broken.role.is_none());
        remove(&home, "broken").unwrap();
        assert!(catalog(&home).is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }
}
